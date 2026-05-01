//! Styled QR rendering for `/v1/links/{link_id}/qr.{format}`.
//!
//! Two layers:
//! - `render_link_qr` — top-level orchestrator called from the route
//!   handler; resolves the link, builds options, dispatches to `render_qr`.
//! - Lower helpers (parse_*, fetch_logo, render_qr) wrap the
//!   `qr_code_styling` crate with our defaults, validation, and
//!   per-link canonical URL.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use qr_code_styling::config::{
    BackgroundOptions, Color, CornersDotOptions, CornersSquareOptions, DotsOptions, ImageOptions,
    QROptions,
};
use qr_code_styling::types::{
    CornerDotType, CornerSquareType, DotType, ErrorCorrectionLevel, OutputFormat, ShapeType,
};
use qr_code_styling::QRCodeStyling;
use serde_json::json;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use super::models::QrCodeQuery;
use super::routes::canonical_link_url;
use crate::api::auth::models::TenantId;
use crate::app::AppState;
use image::ImageFormat;

#[derive(Debug, Clone, Copy)]
pub(crate) enum QrOutputFormat {
    Png,
    Svg,
}

pub(crate) async fn render_link_qr(
    state: Arc<AppState>,
    tenant: TenantId,
    link_id: String,
    query: QrCodeQuery,
    format: QrOutputFormat,
) -> Response {
    let Some(repo) = &state.links_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(link) = repo
        .find_link_by_tenant_and_id(&tenant.0, &link_id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Link not found", "code": "not_found" })),
        )
            .into_response();
    };

    let options = match QrRenderOptions::try_from_query(&query).await {
        Ok(options) => options,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": message, "code": "invalid_qr_options" })),
            )
                .into_response();
        }
    };

    let url = canonical_link_url(&state, &link).await;
    match render_qr(&url, &options, format) {
        Ok(bytes) => {
            let content_type = match format {
                QrOutputFormat::Png => "image/png",
                QrOutputFormat::Svg => "image/svg+xml; charset=utf-8",
            };
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, content_type),
                    (header::CACHE_CONTROL, "no-store"),
                ],
                bytes,
            )
                .into_response()
        }
        Err(message) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": message, "code": "qr_render_error" })),
        )
            .into_response(),
    }
}

// ── QR Rendering Helpers ──

const QR_DEFAULT_SIZE: u32 = 600;
const QR_MIN_SIZE: u32 = 128;
const QR_MAX_SIZE: u32 = 2048;
const QR_DEFAULT_MARGIN: u32 = 2;
const QR_MAX_MARGIN: u32 = 16;
const QR_MAX_LOGO_BYTES: usize = 512 * 1024;
const QR_LOGO_TIMEOUT_SECS: u64 = 5;

struct QrRenderOptions {
    size: u32,
    margin: u32,
    level: ErrorCorrectionLevel,
    fg: Color,
    bg: Color,
    logo: Option<LogoImage>,
    dot_type: DotType,
    corner_square_type: CornerSquareType,
    corner_dot_type: CornerDotType,
    shape: ShapeType,
    dot_color: Option<Color>,
    corner_square_color: Option<Color>,
    corner_dot_color: Option<Color>,
}

struct LogoImage {
    png_bytes: Vec<u8>,
}

impl QrRenderOptions {
    async fn try_from_query(query: &QrCodeQuery) -> Result<Self, String> {
        let size = query.size.unwrap_or(QR_DEFAULT_SIZE);
        if !(QR_MIN_SIZE..=QR_MAX_SIZE).contains(&size) {
            return Err(format!(
                "size must be between {QR_MIN_SIZE} and {QR_MAX_SIZE}"
            ));
        }

        let margin = query.margin.unwrap_or_else(|| {
            if query.include_margin == Some(false) {
                0
            } else {
                QR_DEFAULT_MARGIN
            }
        });
        if margin > QR_MAX_MARGIN {
            return Err(format!("margin must be between 0 and {QR_MAX_MARGIN}"));
        }

        let will_render_logo = !query.hide_logo && query.logo.is_some();
        // A centered logo covers the middle of the QR, so force max error correction when the
        // caller didn't pick one — otherwise the default (L) becomes unreadable with a logo.
        let level = match query.level.as_deref() {
            Some(v) => parse_ec_level(v)?,
            None if will_render_logo => ErrorCorrectionLevel::H,
            None => ErrorCorrectionLevel::L,
        };
        let fg = parse_hex_color(query.fg_color.as_deref().unwrap_or("#000000"), "fgColor")?;
        let bg = parse_hex_color(query.bg_color.as_deref().unwrap_or("#FFFFFF"), "bgColor")?;
        let logo = if will_render_logo {
            Some(fetch_logo(query.logo.as_deref().unwrap()).await?)
        } else {
            None
        };

        let dot_type = match query.dot_type.as_deref() {
            Some(v) => parse_dot_type(v)?,
            None => DotType::Rounded,
        };
        let corner_square_type = match query.corner_square_type.as_deref() {
            Some(v) => parse_corner_square_type(v)?,
            None => CornerSquareType::ExtraRounded,
        };
        let corner_dot_type = match query.corner_dot_type.as_deref() {
            Some(v) => parse_corner_dot_type(v)?,
            None => CornerDotType::Dot,
        };
        let shape = match query.shape.as_deref() {
            Some(v) => parse_shape(v)?,
            None => ShapeType::Square,
        };
        let dot_color = match query.dot_color.as_deref() {
            Some(v) => Some(parse_hex_color(v, "dotColor")?),
            None => None,
        };
        let corner_square_color = match query.corner_square_color.as_deref() {
            Some(v) => Some(parse_hex_color(v, "cornerSquareColor")?),
            None => None,
        };
        let corner_dot_color = match query.corner_dot_color.as_deref() {
            Some(v) => Some(parse_hex_color(v, "cornerDotColor")?),
            None => None,
        };

        Ok(Self {
            size,
            margin,
            level,
            fg,
            bg,
            logo,
            dot_type,
            corner_square_type,
            corner_dot_type,
            shape,
            dot_color,
            corner_square_color,
            corner_dot_color,
        })
    }
}

fn parse_ec_level(value: &str) -> Result<ErrorCorrectionLevel, String> {
    match value {
        "L" | "l" => Ok(ErrorCorrectionLevel::L),
        "M" | "m" => Ok(ErrorCorrectionLevel::M),
        "Q" | "q" => Ok(ErrorCorrectionLevel::Q),
        "H" | "h" => Ok(ErrorCorrectionLevel::H),
        _ => Err("level must be one of L, M, Q, H".to_string()),
    }
}

fn parse_dot_type(value: &str) -> Result<DotType, String> {
    match value.to_ascii_lowercase().as_str() {
        "square" => Ok(DotType::Square),
        "dots" => Ok(DotType::Dots),
        "rounded" => Ok(DotType::Rounded),
        "classy" => Ok(DotType::Classy),
        "classy-rounded" | "classyrounded" => Ok(DotType::ClassyRounded),
        "extra-rounded" | "extrarounded" => Ok(DotType::ExtraRounded),
        _ => Err(
            "dotType must be one of square, dots, rounded, classy, classy-rounded, extra-rounded"
                .to_string(),
        ),
    }
}

fn parse_corner_square_type(value: &str) -> Result<CornerSquareType, String> {
    match value.to_ascii_lowercase().as_str() {
        "square" => Ok(CornerSquareType::Square),
        "dot" => Ok(CornerSquareType::Dot),
        "extra-rounded" | "extrarounded" => Ok(CornerSquareType::ExtraRounded),
        _ => Err("cornerSquareType must be one of square, dot, extra-rounded".to_string()),
    }
}

fn parse_corner_dot_type(value: &str) -> Result<CornerDotType, String> {
    match value.to_ascii_lowercase().as_str() {
        "dot" => Ok(CornerDotType::Dot),
        "square" => Ok(CornerDotType::Square),
        _ => Err("cornerDotType must be one of dot, square".to_string()),
    }
}

fn parse_shape(value: &str) -> Result<ShapeType, String> {
    match value.to_ascii_lowercase().as_str() {
        "square" => Ok(ShapeType::Square),
        "circle" => Ok(ShapeType::Circle),
        _ => Err("shape must be one of square, circle".to_string()),
    }
}

fn parse_hex_color(value: &str, name: &str) -> Result<Color, String> {
    let value = value.trim();
    let Some(hex) = value.strip_prefix('#') else {
        return Err(format!("{name} must be #RGB or #RRGGBB"));
    };
    if hex.len() != 3 && hex.len() != 6 {
        return Err(format!("{name} must be #RGB or #RRGGBB"));
    }
    Color::from_hex(value).map_err(|_| format!("{name} must be #RGB or #RRGGBB"))
}

async fn fetch_logo(url: &str) -> Result<LogoImage, String> {
    crate::core::validation::validate_web_url(url).map_err(|e| format!("logo: {e}"))?;
    // SSRF guard: validate_web_url only checks the user-supplied string. Following redirects
    // would let a public origin rebind to an internal IP mid-request, so disable them outright.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(QR_LOGO_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("logo client error: {e}"))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("logo fetch failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("logo fetch returned {}", response.status()));
    }
    if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
        let content_type = content_type.to_str().unwrap_or_default().to_lowercase();
        let allowed = content_type.starts_with("image/png")
            || content_type.starts_with("image/jpeg")
            || content_type.starts_with("image/webp");
        if !allowed {
            return Err("logo must be PNG, JPEG, or WebP".to_string());
        }
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("logo read failed: {e}"))?;
    if bytes.len() > QR_MAX_LOGO_BYTES {
        return Err(format!("logo must be under {QR_MAX_LOGO_BYTES} bytes"));
    }

    let image = image::load_from_memory(&bytes)
        .map_err(|_| "logo must be a valid PNG, JPEG, or WebP image".to_string())?;
    let mut png_bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| format!("logo encode failed: {e}"))?;

    Ok(LogoImage { png_bytes })
}

fn render_qr(
    url: &str,
    options: &QrRenderOptions,
    format: QrOutputFormat,
) -> Result<Vec<u8>, String> {
    let margin_px = options.margin.saturating_mul(10);
    let dot_color = options.dot_color.unwrap_or(options.fg);
    let corner_square_color = options.corner_square_color.unwrap_or(options.fg);
    let corner_dot_color = options.corner_dot_color.unwrap_or(options.fg);
    let mut builder = QRCodeStyling::builder()
        .data(url)
        .size(options.size)
        .margin(margin_px)
        .shape(options.shape)
        .qr_options(QROptions::new().with_error_correction_level(options.level))
        .dots_options(DotsOptions::new(options.dot_type).with_color(dot_color))
        .corners_square_options(
            CornersSquareOptions::new(options.corner_square_type).with_color(corner_square_color),
        )
        .corners_dot_options(
            CornersDotOptions::new(options.corner_dot_type).with_color(corner_dot_color),
        )
        .background_options(BackgroundOptions::new(options.bg));

    if let Some(logo) = &options.logo {
        builder = builder.image(logo.png_bytes.clone()).image_options(
            ImageOptions::new()
                .with_image_size(0.22)
                .with_margin(6)
                .with_hide_background_dots(true)
                .with_save_as_blob(true),
        );
    }

    let qr = builder
        .build()
        .map_err(|e| format!("failed to build QR code: {e}"))?;
    match format {
        QrOutputFormat::Png => qr
            .render(OutputFormat::Png)
            .map_err(|e| format!("failed to render PNG: {e}")),
        QrOutputFormat::Svg => qr
            .render(OutputFormat::Svg)
            .map_err(|e| format!("failed to render SVG: {e}")),
    }
}

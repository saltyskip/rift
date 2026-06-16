//! Palette derivation engine for the landing page.
//!
//! Turns a single brand `theme_color` into a full, contrast-correct set of CSS
//! color tokens. The accent is clamped into a tasteful saturation/lightness
//! band so even a neon or muddy input renders well, and the on-accent text
//! color is chosen by WCAG luminance so the CTA is always readable. A garbage
//! color falls back to Rift's default accent rather than breaking the page.
//!
//! Pure presentation logic with no dependencies — lives in the HTTP transport
//! layer because the landing page is HTTP-only (MCP never renders HTML).

use crate::services::landing::models::{ColorScheme, DEFAULT_ACCENT};

// ── Public surface ──

/// A resolved set of CSS color values for one canvas (dark or light).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Palette {
    pub bg: String,
    pub surface: String,
    pub border: String,
    pub text: String,
    pub text_muted: String,
    pub accent: String,
    /// Lighter tone of the accent (gradient top, hover, accent text on dark).
    pub accent_bright: String,
    /// Darker tone of the accent (CTA gradient floor).
    pub accent_deep: String,
    /// Text/icon color that sits *on* the accent (the CTA button label).
    pub accent_text: String,
    /// `rgba(...)` glow used for the button shadow.
    pub accent_glow: String,
}

/// The palette(s) a template applies: `root` at `:root`, plus an optional
/// `prefers_light` override emitted under `@media (prefers-color-scheme: light)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DerivedPalettes {
    pub root: Palette,
    pub prefers_light: Option<Palette>,
}

/// Derive the palette(s) for `theme_color` under the given `scheme`.
///
/// - `Dark` → dark `root`, no override
/// - `Light` → light `root`, no override
/// - `Auto` → dark `root` + a light `prefers-color-scheme` override
pub(crate) fn derive_palette(theme_color: &str, scheme: ColorScheme) -> DerivedPalettes {
    let rgb = parse_hex(theme_color)
        .or_else(|| parse_hex(DEFAULT_ACCENT))
        .unwrap_or((13, 148, 136));
    let accent_rgb = clamp_accent(rgb);
    let accent = to_hex(accent_rgb);
    // Lighter/darker tones for the gradient CTA, derived by shifting lightness.
    let (ah, as_, al) = rgb_to_hsl(accent_rgb);
    let accent_bright = to_hex(hsl_to_rgb((ah, as_, (al + 0.14).min(0.82))));
    let accent_deep = to_hex(hsl_to_rgb((ah, as_, (al - 0.16).max(0.20))));
    let accent_text = if relative_luminance(accent_rgb) > 0.45 {
        "#0a0a0a".to_string()
    } else {
        "#fafafa".to_string()
    };
    let accent_glow = {
        let (r, g, b) = accent_rgb;
        format!("rgba({r},{g},{b},0.35)")
    };

    let make = |dark: bool| {
        let (base_bg, base_surface, base_border, text, text_muted) = if dark {
            (
                (10, 10, 10),
                (17, 17, 19),
                (38, 38, 43),
                "#fafafa",
                "#a1a1aa",
            )
        } else {
            (
                (255, 255, 255),
                (250, 250, 251),
                (228, 228, 231),
                "#18181b",
                "#52525b",
            )
        };
        Palette {
            bg: to_hex(mix(base_bg, accent_rgb, 0.06)),
            surface: to_hex(mix(base_surface, accent_rgb, 0.08)),
            border: to_hex(mix(base_border, accent_rgb, 0.12)),
            text: text.to_string(),
            text_muted: text_muted.to_string(),
            accent: accent.clone(),
            accent_bright: accent_bright.clone(),
            accent_deep: accent_deep.clone(),
            accent_text: accent_text.clone(),
            accent_glow: accent_glow.clone(),
        }
    };

    match scheme {
        ColorScheme::Dark => DerivedPalettes {
            root: make(true),
            prefers_light: None,
        },
        ColorScheme::Light => DerivedPalettes {
            root: make(false),
            prefers_light: None,
        },
        ColorScheme::Auto => DerivedPalettes {
            root: make(true),
            prefers_light: Some(make(false)),
        },
    }
}

// ── Helpers ──

/// Parse `#RGB` or `#RRGGBB` into an `(r, g, b)` triple. Mirrors the format
/// accepted by `core::validation::validate_hex_color`.
fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let hex = s.trim().strip_prefix('#')?;
    let expand = |c: char| -> Option<u8> {
        let d = c.to_digit(16)? as u8;
        Some(d * 16 + d)
    };
    match hex.len() {
        3 => {
            let mut it = hex.chars();
            Some((
                expand(it.next()?)?,
                expand(it.next()?)?,
                expand(it.next()?)?,
            ))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

fn to_hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Linear-RGB blend: `amount` is the weight of `tint` mixed into `base`
/// (`0.0` → all base, `1.0` → all tint).
fn mix(base: (u8, u8, u8), tint: (u8, u8, u8), amount: f64) -> (u8, u8, u8) {
    let a = amount.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| -> u8 { (f64::from(x) * (1.0 - a) + f64::from(y) * a).round() as u8 };
    (
        lerp(base.0, tint.0),
        lerp(base.1, tint.1),
        lerp(base.2, tint.2),
    )
}

/// WCAG relative luminance in `[0, 1]` (black → 0, white → 1).
fn relative_luminance((r, g, b): (u8, u8, u8)) -> f64 {
    let lin = |c: u8| {
        let s = f64::from(c) / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// Clamp an accent into a tasteful saturation/lightness band so neon stays
/// usable and muddy gets lifted — the "can't make trash" guarantee.
fn clamp_accent(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    let (h, s, l) = rgb_to_hsl(rgb);
    hsl_to_rgb((h, s.clamp(0.30, 0.90), l.clamp(0.40, 0.65)))
}

fn rgb_to_hsl((r, g, b): (u8, u8, u8)) -> (f64, f64, f64) {
    let r = f64::from(r) / 255.0;
    let g = f64::from(g) / 255.0;
    let b = f64::from(b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let delta = max - min;
    if delta.abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = delta / (1.0 - (2.0 * l - 1.0).abs());
    let h = if (max - r).abs() < f64::EPSILON {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if (max - g).abs() < f64::EPSILON {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    (h.rem_euclid(360.0), s, l)
}

fn hsl_to_rgb((h, s, l): (f64, f64, f64)) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to_u8 = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

#[cfg(test)]
#[path = "palette_tests.rs"]
mod tests;

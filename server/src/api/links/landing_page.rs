use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::sync::Arc;

use super::models::{AgentContext, Link, LinkThemeOverride};
use crate::api::domains::models::Domain;
use crate::api::themes::models::{
    BackgroundStyle, ButtonStyle, CardStyle, ContentAlignment, ContentWidth, FontPreset,
    LandingTheme, LayoutTemplate, RadiusPreset, ShadowPreset, ThemeBackground, ThemeCopy,
    ThemeLayout, ThemeMedia, ThemeModules, ThemePalette, ThemeSeo, ThemeShape, ThemeTokens,
    ThemeTypography, TypeScale,
};
use crate::api::AppState;

const LANDING_PAGE_TEMPLATE: &str = include_str!("landing_page.html");

pub struct LandingPageContext<'a> {
    pub platform_name: &'a str,
    pub is_android: bool,
    pub link: &'a Link,
    pub link_id: &'a str,
    pub theme: &'a EffectiveTheme,
    pub meta_title: Option<&'a str>,
    pub meta_description: Option<&'a str>,
    pub meta_image: Option<&'a str>,
    pub agent_context: Option<&'a AgentContext>,
    pub link_status: &'a str,
    pub tenant_domain: Option<&'a str>,
    pub tenant_verified: bool,
}

#[derive(Debug, Clone)]
pub struct EffectiveTheme {
    pub brand_name: String,
    pub tagline: Option<String>,
    pub headline: String,
    pub subheadline: Option<String>,
    pub badge_text: Option<String>,
    pub primary_cta_label: Option<String>,
    pub footer_text: Option<String>,
    pub icon_url: Option<String>,
    pub logo_url: Option<String>,
    pub wordmark_url: Option<String>,
    pub hero_image_url: Option<String>,
    pub og_title: String,
    pub og_description: String,
    pub og_image_url: Option<String>,
    pub template: LayoutTemplate,
    pub alignment: ContentAlignment,
    pub content_width: ContentWidth,
    pub background_style: BackgroundStyle,
    pub background_solid: String,
    pub background_gradient_from: String,
    pub background_gradient_to: String,
    pub background_gradient_angle: i32,
    pub background_image_url: Option<String>,
    pub overlay_opacity: f32,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub surface: String,
    pub surface_muted: String,
    pub text: String,
    pub text_muted: String,
    pub border: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    pub radius_px: usize,
    pub heading_font: &'static str,
    pub body_font: &'static str,
    pub mono_font: &'static str,
    pub heading_size_px: usize,
    pub body_size_px: usize,
    pub shadow_css: &'static str,
    pub card_surface_css: String,
    pub cta_style: ButtonStyle,
    pub show_logo: bool,
    pub show_icon: bool,
    pub show_hero_image: bool,
    pub show_footer: bool,
    pub show_store_badges: bool,
}

pub fn render_smart_landing_page(ctx: &LandingPageContext<'_>) -> String {
    let theme = ctx.theme;
    let link = ctx.link;
    let platform_js = js_escape(ctx.platform_name);

    let store_url = if ctx.platform_name == "ios" {
        link.ios_store_url.as_deref().unwrap_or("")
    } else if ctx.platform_name == "android" {
        link.android_store_url.as_deref().unwrap_or("")
    } else {
        ""
    };

    let store_url_with_referrer = if ctx.is_android && !store_url.is_empty() {
        let sep = if store_url.contains('?') { "&" } else { "?" };
        format!(
            "{}{}referrer={}",
            store_url,
            sep,
            urlencoding(&format!("rift_link={}", ctx.link_id))
        )
    } else {
        store_url.to_string()
    };
    let store_url_js = js_escape(&store_url_with_referrer);

    let web_url = link.web_url.as_deref().unwrap_or("");
    let web_url_js = js_escape(web_url);

    let json_ld = if let Some(ac) = ctx.agent_context {
        if ac.action.is_some() || ac.cta.is_some() || ac.description.is_some() {
            let action_type = ac
                .action
                .as_deref()
                .map(action_to_schema_type)
                .unwrap_or("ViewAction");

            let mut entry_points = Vec::new();
            if let Some(dl) = &ctx.link.ios_deep_link {
                entry_points.push(json!({
                    "@type": "EntryPoint",
                    "urlTemplate": dl,
                    "actionPlatform": "http://schema.org/IOSPlatform"
                }));
            }
            if let Some(dl) = &ctx.link.android_deep_link {
                entry_points.push(json!({
                    "@type": "EntryPoint",
                    "urlTemplate": dl,
                    "actionPlatform": "http://schema.org/AndroidPlatform"
                }));
            }
            if let Some(url) = &ctx.link.web_url {
                entry_points.push(json!({
                    "@type": "EntryPoint",
                    "urlTemplate": url,
                    "actionPlatform": "http://schema.org/DesktopWebPlatform"
                }));
            }

            let mut action = json!({
                "@context": "https://schema.org",
                "@type": action_type,
            });
            if let Some(cta) = &ac.cta {
                action["name"] = json!(cta);
            }
            if let Some(desc) = &ac.description {
                action["description"] = json!(desc);
            }
            if !entry_points.is_empty() {
                action["target"] = json!(entry_points);
            }

            if ctx.meta_title.is_some() || ctx.meta_description.is_some() {
                let mut product = json!({"@type": "Product"});
                if let Some(t) = ctx.meta_title {
                    product["name"] = json!(t);
                }
                if let Some(d) = ctx.meta_description {
                    product["description"] = json!(d);
                }
                action["object"] = product;
            }

            action["provider"] = json!({
                "@type": "Organization",
                "name": ctx.tenant_domain.unwrap_or("unknown"),
                "additionalProperty": [
                    { "@type": "PropertyValue", "name": "status", "value": ctx.link_status },
                    { "@type": "PropertyValue", "name": "verified", "value": ctx.tenant_verified },
                ]
            });

            let json_str = serde_json::to_string(&action).unwrap_or_default();
            let json_str = json_str.replace("</", "<\\/");
            format!(
                r#"    <script type="application/ld+json">{}</script>"#,
                json_str
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let og_image_tag = theme
        .og_image_url
        .as_deref()
        .or(ctx.meta_image)
        .map(|img| {
            format!(
                r#"    <meta property="og:image" content="{}" />"#,
                html_escape(img)
            )
        })
        .unwrap_or_default();

    let icon_html = theme
        .icon_url
        .as_deref()
        .map(|url| {
            format!(
                r#"<img class="app-icon" src="{}" alt="{}" />"#,
                html_escape(url),
                html_escape(&theme.brand_name),
            )
        })
        .unwrap_or_default();
    let logo_html = if theme.show_logo {
        if let Some(logo_url) = theme.logo_url.as_deref() {
            format!(
                r#"<div class="brand-lockup"><img class="brand-mark logo" src="{}" alt="{}" /><div class="brand-name">{}</div></div>"#,
                html_escape(logo_url),
                html_escape(&theme.brand_name),
                html_escape(&theme.brand_name)
            )
        } else {
            format!(
                r#"<div class="brand-name">{}</div>"#,
                html_escape(&theme.brand_name)
            )
        }
    } else {
        String::new()
    };
    let badge_html = theme
        .badge_text
        .as_deref()
        .map(|text| format!(r#"<div class="theme-badge">{}</div>"#, html_escape(text)))
        .unwrap_or_default();
    let tagline_html = theme
        .tagline
        .as_deref()
        .map(|text| format!(r#"<p class="tagline">{}</p>"#, html_escape(text)))
        .unwrap_or_default();
    let hero_image_html = if theme.show_hero_image {
        theme.hero_image_url
            .as_deref()
            .map(|url| {
                format!(
                    r#"<div class="hero-image-shell"><img class="hero-image" src="{}" alt="{}" /></div>"#,
                    html_escape(url),
                    html_escape(&theme.brand_name)
                )
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    let footer_html = if theme.show_footer {
        theme
            .footer_text
            .as_deref()
            .map(|text| format!(r#"<p class="human-footer">{}</p>"#, html_escape(text)))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let agent_description = ctx.agent_context.and_then(|ac| ac.description.as_deref());
    let meta_desc_tag = agent_description
        .or(Some(theme.og_description.as_str()))
        .map(|d| {
            format!(
                r#"    <meta name="description" content="{}" />"#,
                html_escape(d)
            )
        })
        .unwrap_or_default();

    let agent_panel = build_agent_panel(ctx);
    let bg_image_css = theme
        .background_image_url
        .as_deref()
        .map(|url| format!("url('{}')", html_escape(url)))
        .unwrap_or_else(|| "none".to_string());
    let background_css = match theme.background_style {
        BackgroundStyle::Solid => html_escape(&theme.background_solid),
        BackgroundStyle::Gradient => format!(
            "linear-gradient({}deg, {} 0%, {} 100%)",
            theme.background_gradient_angle,
            html_escape(&theme.background_gradient_from),
            html_escape(&theme.background_gradient_to)
        ),
        BackgroundStyle::Image => format!(
            "linear-gradient(rgba(15,23,42,{opacity}), rgba(15,23,42,{opacity})), {image}",
            opacity = theme.overlay_opacity,
            image = bg_image_css
        ),
    };

    render_template(&[
        ("{{OG_TITLE}}", html_escape(&theme.og_title)),
        ("{{OG_TITLE_ESCAPED}}", html_escape(&theme.og_title)),
        (
            "{{OG_DESCRIPTION_ESCAPED}}",
            html_escape(&theme.og_description),
        ),
        ("{{META_DESCRIPTION_TAG}}", meta_desc_tag),
        ("{{OG_IMAGE_TAG}}", og_image_tag),
        ("{{JSON_LD}}", json_ld),
        ("{{BACKGROUND_CSS}}", background_css),
        ("{{SURFACE}}", html_escape(&theme.surface)),
        ("{{SURFACE_MUTED}}", html_escape(&theme.surface_muted)),
        ("{{TEXT}}", html_escape(&theme.text)),
        ("{{TEXT_MUTED}}", html_escape(&theme.text_muted)),
        ("{{BORDER}}", html_escape(&theme.border)),
        ("{{PRIMARY}}", html_escape(&theme.primary)),
        (
            "{{PRIMARY_TEXT}}",
            preferred_text_on(&theme.primary).to_string(),
        ),
        ("{{SECONDARY}}", html_escape(&theme.secondary)),
        ("{{ACCENT}}", html_escape(&theme.accent)),
        ("{{SUCCESS}}", html_escape(&theme.success)),
        ("{{WARNING}}", html_escape(&theme.warning)),
        ("{{DANGER}}", html_escape(&theme.danger)),
        ("{{RADIUS}}", theme.radius_px.to_string()),
        ("{{SHADOW_CSS}}", theme.shadow_css.to_string()),
        ("{{CARD_SURFACE}}", theme.card_surface_css.clone()),
        ("{{HEADING_FONT}}", theme.heading_font.to_string()),
        ("{{BODY_FONT}}", theme.body_font.to_string()),
        ("{{MONO_FONT}}", theme.mono_font.to_string()),
        ("{{HEADING_SIZE}}", theme.heading_size_px.to_string()),
        ("{{BODY_SIZE}}", theme.body_size_px.to_string()),
        ("{{BADGE_HTML}}", badge_html),
        ("{{LOGO_HTML}}", logo_html),
        (
            "{{ICON_BLOCK}}",
            if theme.show_icon {
                icon_html
            } else {
                String::new()
            },
        ),
        ("{{TAGLINE_HTML}}", tagline_html),
        ("{{HEADLINE}}", html_escape(&theme.headline)),
        (
            "{{SUBHEADLINE}}",
            html_escape(
                theme
                    .subheadline
                    .as_deref()
                    .unwrap_or("Open the app to continue."),
            ),
        ),
        (
            "{{BUTTON_CLASS}}",
            button_class(&theme.cta_style).to_string(),
        ),
        (
            "{{DEFAULT_CTA}}",
            html_escape(theme.primary_cta_label.as_deref().unwrap_or("Continue")),
        ),
        ("{{FOOTER_HTML}}", footer_html),
        ("{{HERO_IMAGE_HTML}}", hero_image_html),
        ("{{AGENT_PANEL}}", agent_panel),
        ("{{PLATFORM_JS}}", platform_js),
        ("{{STORE_URL_JS}}", store_url_js),
        ("{{WEB_URL_JS}}", web_url_js),
        ("{{BRAND_NAME}}", js_escape(&theme.brand_name)),
    ])
}

pub struct ThemeResolutionInput<'a> {
    pub tenant_id: &'a ObjectId,
    pub resolved_domain: Option<&'a Domain>,
    pub link_override: Option<&'a LinkThemeOverride>,
    pub app_name: Option<&'a str>,
    pub app_icon_url: Option<&'a str>,
    pub meta_title: Option<&'a str>,
    pub meta_description: Option<&'a str>,
    pub meta_image: Option<&'a str>,
}

pub async fn resolve_effective_theme(
    state: &Arc<AppState>,
    input: &ThemeResolutionInput<'_>,
) -> EffectiveTheme {
    let mut themed = EffectiveTheme::default_from(
        input.app_name,
        input.app_icon_url,
        input.meta_title,
        input.meta_description,
    );

    if let Some(repo) = &state.themes_repo {
        if let Ok(Some(default_theme)) = repo.find_default_by_tenant(input.tenant_id).await {
            themed.apply_theme(&default_theme);
        }

        if let Some(domain_theme_id) = input
            .resolved_domain
            .and_then(|domain| domain.theme_id.as_ref())
        {
            if let Ok(Some(domain_theme)) = repo
                .find_by_tenant_and_id(input.tenant_id, domain_theme_id)
                .await
            {
                themed.apply_theme(&domain_theme);
            }
        }

        if let Some(theme_id) = input
            .link_override
            .and_then(|override_theme| override_theme.theme_id.as_deref())
            .and_then(|theme_id| ObjectId::parse_str(theme_id).ok())
        {
            if let Ok(Some(link_theme)) =
                repo.find_by_tenant_and_id(input.tenant_id, &theme_id).await
            {
                themed.apply_theme(&link_theme);
            }
        }
    }

    themed.apply_link_override(
        input.link_override,
        input.meta_title,
        input.meta_description,
        input.meta_image,
    );
    themed
}

pub fn demo_preview_theme(state: &AppState, theme_slug: &str) -> Option<EffectiveTheme> {
    let base = format!(
        "{}/__preview/assets/{theme_slug}",
        state.config.public_url.trim_end_matches('/')
    );
    let mut theme = match theme_slug {
        "nord-roast" => EffectiveTheme {
            brand_name: "Nord Roast".to_string(),
            tagline: Some("Small-batch coffee, delivered beautifully.".to_string()),
            headline: "Your next bag is one tap away".to_string(),
            subheadline: Some("Open the app to claim this roast, track delivery, and save your grind preferences.".to_string()),
            badge_text: Some("Roaster's Pick".to_string()),
            primary_cta_label: Some("Open Nord Roast".to_string()),
            footer_text: Some("Roasted weekly in Chicago.".to_string()),
            icon_url: Some(format!("{base}/icon.png")),
            logo_url: None,
            wordmark_url: None,
            hero_image_url: Some("https://images.pexels.com/photos/31950325/pexels-photo-31950325.jpeg?auto=compress&cs=tinysrgb&dpr=2&h=650&w=940".to_string()),
            og_title: "Your next bag is one tap away".to_string(),
            og_description: "Warm editorial coffee theme demo with tactile visuals and copper accents.".to_string(),
            og_image_url: Some("https://images.pexels.com/photos/31950325/pexels-photo-31950325.jpeg?auto=compress&cs=tinysrgb&dpr=2&h=650&w=940".to_string()),
            template: LayoutTemplate::Editorial,
            alignment: ContentAlignment::Left,
            content_width: ContentWidth::Regular,
            background_style: BackgroundStyle::Solid,
            background_solid: "#F4EBDD".to_string(),
            background_gradient_from: "#F4EBDD".to_string(),
            background_gradient_to: "#EBDDCB".to_string(),
            background_gradient_angle: 135,
            background_image_url: None,
            overlay_opacity: 0.18,
            primary: "#C46A2D".to_string(),
            secondary: "#7B4B2A".to_string(),
            accent: "#E6B17E".to_string(),
            surface: "#FFF8EF".to_string(),
            surface_muted: "#EBDDCB".to_string(),
            text: "#2F241D".to_string(),
            text_muted: "#6B5A4D".to_string(),
            border: "#D7C3AA".to_string(),
            success: "#4F7A4C".to_string(),
            warning: "#B7791F".to_string(),
            danger: "#B54A3A".to_string(),
            radius_px: 18,
            heading_font: font_stack(&FontPreset::EditorialSerif),
            body_font: font_stack(&FontPreset::HumanistSans),
            mono_font: font_stack(&FontPreset::SystemSans),
            heading_size_px: 56,
            body_size_px: 17,
            shadow_css: shadow_css(&ShadowPreset::Soft),
            card_surface_css: card_surface_css(&CardStyle::Elevated),
            cta_style: ButtonStyle::Solid,
            show_logo: true,
            show_icon: true,
            show_hero_image: true,
            show_footer: true,
            show_store_badges: true,
        },
        "volt-run" => EffectiveTheme {
            brand_name: "Volt Run".to_string(),
            tagline: Some("Train louder.".to_string()),
            headline: "Start the challenge in the app".to_string(),
            subheadline: Some("Unlock your plan, live splits, and team leaderboard in one place.".to_string()),
            badge_text: Some("Night Sprint".to_string()),
            primary_cta_label: Some("Launch Volt Run".to_string()),
            footer_text: Some("Performance training for urban athletes.".to_string()),
            icon_url: Some(format!("{base}/icon.png")),
            logo_url: None,
            wordmark_url: None,
            hero_image_url: Some("https://images.pexels.com/photos/12125324/pexels-photo-12125324.jpeg?auto=compress&cs=tinysrgb&dpr=2&h=650&w=940".to_string()),
            og_title: "Start the challenge in the app".to_string(),
            og_description: "High-contrast performance theme demo with neon accents and kinetic motion.".to_string(),
            og_image_url: Some("https://images.pexels.com/photos/12125324/pexels-photo-12125324.jpeg?auto=compress&cs=tinysrgb&dpr=2&h=650&w=940".to_string()),
            template: LayoutTemplate::Split,
            alignment: ContentAlignment::Left,
            content_width: ContentWidth::Wide,
            background_style: BackgroundStyle::Gradient,
            background_solid: "#05070A".to_string(),
            background_gradient_from: "#05070A".to_string(),
            background_gradient_to: "#0E1621".to_string(),
            background_gradient_angle: 145,
            background_image_url: None,
            overlay_opacity: 0.24,
            primary: "#B6FF00".to_string(),
            secondary: "#00E5FF".to_string(),
            accent: "#FFFFFF".to_string(),
            surface: "#0D1117".to_string(),
            surface_muted: "#151B23".to_string(),
            text: "#F5F7FA".to_string(),
            text_muted: "#9AA4B2".to_string(),
            border: "#232C37".to_string(),
            success: "#2DFF87".to_string(),
            warning: "#FFC247".to_string(),
            danger: "#FF5A6B".to_string(),
            radius_px: 8,
            heading_font: font_stack(&FontPreset::GeometricSans),
            body_font: font_stack(&FontPreset::ModernSans),
            mono_font: font_stack(&FontPreset::SystemSans),
            heading_size_px: 64,
            body_size_px: 18,
            shadow_css: shadow_css(&ShadowPreset::Dramatic),
            card_surface_css: card_surface_css(&CardStyle::Glass),
            cta_style: ButtonStyle::Outline,
            show_logo: true,
            show_icon: false,
            show_hero_image: true,
            show_footer: true,
            show_store_badges: true,
        },
        "atelier-stay" => EffectiveTheme {
            brand_name: "Atelier Stay".to_string(),
            tagline: Some("Private hotels for slower travel.".to_string()),
            headline: "Continue your reservation".to_string(),
            subheadline: Some("Open the app to view suite details, arrival notes, and concierge recommendations.".to_string()),
            badge_text: Some("Founding Guest".to_string()),
            primary_cta_label: Some("Open Atelier Stay".to_string()),
            footer_text: Some("Member support available 24/7.".to_string()),
            icon_url: Some(format!("{base}/icon.png")),
            logo_url: None,
            wordmark_url: None,
            hero_image_url: Some("https://images.pexels.com/photos/338504/pexels-photo-338504.jpeg?auto=compress&cs=tinysrgb&h=650&w=940".to_string()),
            og_title: "Continue your reservation".to_string(),
            og_description: "Quiet luxury hospitality theme demo with bright editorial imagery.".to_string(),
            og_image_url: Some("https://images.pexels.com/photos/338504/pexels-photo-338504.jpeg?auto=compress&cs=tinysrgb&h=650&w=940".to_string()),
            template: LayoutTemplate::Centered,
            alignment: ContentAlignment::Center,
            content_width: ContentWidth::Narrow,
            background_style: BackgroundStyle::Solid,
            background_solid: "#F7F4EF".to_string(),
            background_gradient_from: "#F7F4EF".to_string(),
            background_gradient_to: "#EFE8DE".to_string(),
            background_gradient_angle: 180,
            background_image_url: None,
            overlay_opacity: 0.12,
            primary: "#1E3A5F".to_string(),
            secondary: "#C8A96B".to_string(),
            accent: "#8DA9C4".to_string(),
            surface: "#FFFCF8".to_string(),
            surface_muted: "#EFE8DE".to_string(),
            text: "#1F2933".to_string(),
            text_muted: "#6B7280".to_string(),
            border: "#DDD4C8".to_string(),
            success: "#507A5B".to_string(),
            warning: "#B8893D".to_string(),
            danger: "#A94E4E".to_string(),
            radius_px: 26,
            heading_font: font_stack(&FontPreset::EditorialSerif),
            body_font: font_stack(&FontPreset::ModernSans),
            mono_font: font_stack(&FontPreset::SystemSans),
            heading_size_px: 60,
            body_size_px: 18,
            shadow_css: shadow_css(&ShadowPreset::None),
            card_surface_css: card_surface_css(&CardStyle::Flat),
            cta_style: ButtonStyle::Soft,
            show_logo: true,
            show_icon: false,
            show_hero_image: true,
            show_footer: true,
            show_store_badges: true,
        },
        _ => return None,
    };

    theme.og_image_url = theme.hero_image_url.clone();
    Some(theme)
}

pub fn preview_asset_path(theme_slug: &str, asset_name: &str) -> Option<String> {
    let allowed_theme = matches!(theme_slug, "nord-roast" | "volt-run" | "atelier-stay");
    let allowed_asset = matches!(
        asset_name,
        "hero.png" | "icon.png" | "logo.png" | "wordmark.png"
    );
    if !allowed_theme || !allowed_asset {
        return None;
    }

    Some(format!(
        "{}/../marketing/public/demo-themes/{theme_slug}/{asset_name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

pub fn content_type_for_asset(asset_name: &str) -> &'static str {
    if asset_name.ends_with(".png") {
        "image/png"
    } else if asset_name.ends_with(".jpg") || asset_name.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    }
}

impl EffectiveTheme {
    fn default_from(
        app_name: Option<&str>,
        app_icon_url: Option<&str>,
        meta_title: Option<&str>,
        meta_description: Option<&str>,
    ) -> Self {
        let brand_name = app_name.unwrap_or("App").to_string();
        let headline = meta_title
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("Open in {brand_name}"));
        let og_description = meta_description.unwrap_or("Open in app").to_string();

        Self {
            brand_name: brand_name.clone(),
            tagline: None,
            headline,
            subheadline: meta_description.map(ToString::to_string),
            badge_text: None,
            primary_cta_label: Some(format!("Open {brand_name}")),
            footer_text: None,
            icon_url: app_icon_url.map(ToString::to_string),
            logo_url: None,
            wordmark_url: None,
            hero_image_url: None,
            og_title: brand_name,
            og_description,
            og_image_url: None,
            template: LayoutTemplate::Split,
            alignment: ContentAlignment::Center,
            content_width: ContentWidth::Regular,
            background_style: BackgroundStyle::Gradient,
            background_solid: "#0B1020".to_string(),
            background_gradient_from: "#081019".to_string(),
            background_gradient_to: "#162235".to_string(),
            background_gradient_angle: 140,
            background_image_url: None,
            overlay_opacity: 0.28,
            primary: "#0d9488".to_string(),
            secondary: "#1f2937".to_string(),
            accent: "#14b8a6".to_string(),
            surface: "#0d1117".to_string(),
            surface_muted: "#121722".to_string(),
            text: "#f8fafc".to_string(),
            text_muted: "#94a3b8".to_string(),
            border: "#223148".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
            radius_px: 22,
            heading_font: font_stack(&FontPreset::ModernSans),
            body_font: font_stack(&FontPreset::HumanistSans),
            mono_font: font_stack(&FontPreset::SystemSans),
            heading_size_px: 54,
            body_size_px: 17,
            shadow_css: shadow_css(&ShadowPreset::Medium),
            card_surface_css: card_surface_css(&CardStyle::Elevated),
            cta_style: ButtonStyle::Solid,
            show_logo: true,
            show_icon: true,
            show_hero_image: true,
            show_footer: true,
            show_store_badges: true,
        }
    }

    fn apply_theme(&mut self, theme: &LandingTheme) {
        apply_tokens(self, &theme.tokens);
        apply_copy(self, &theme.copy);
        apply_media(self, &theme.media);
        apply_layout(self, &theme.layout);
        apply_modules(self, &theme.modules);
        apply_seo(self, &theme.seo);
    }

    fn apply_link_override(
        &mut self,
        link_override: Option<&LinkThemeOverride>,
        meta_title: Option<&str>,
        meta_description: Option<&str>,
        meta_image: Option<&str>,
    ) {
        if let Some(meta_title) = meta_title {
            self.og_title = meta_title.to_string();
        }
        if let Some(meta_description) = meta_description {
            self.og_description = meta_description.to_string();
        }
        if let Some(meta_image) = meta_image {
            self.og_image_url = Some(meta_image.to_string());
        }

        let Some(link_override) = link_override else {
            return;
        };

        if let Some(headline) = &link_override.headline {
            self.headline = headline.clone();
            self.og_title = headline.clone();
        }
        if let Some(subheadline) = &link_override.subheadline {
            self.subheadline = Some(subheadline.clone());
            self.og_description = subheadline.clone();
        }
        if let Some(badge_text) = &link_override.badge_text {
            self.badge_text = Some(badge_text.clone());
        }
        if let Some(hero_image_url) = &link_override.hero_image_url {
            self.hero_image_url = Some(hero_image_url.clone());
        }
        if let Some(primary_cta_label) = &link_override.primary_cta_label {
            self.primary_cta_label = Some(primary_cta_label.clone());
        }
        if let Some(og_title) = &link_override.og_title {
            self.og_title = og_title.clone();
        }
        if let Some(og_description) = &link_override.og_description {
            self.og_description = og_description.clone();
        }
        if let Some(og_image_url) = &link_override.og_image_url {
            self.og_image_url = Some(og_image_url.clone());
        }
    }
}

fn apply_tokens(theme: &mut EffectiveTheme, tokens: &ThemeTokens) {
    if let Some(palette) = &tokens.palette {
        apply_palette(theme, palette);
    }
    if let Some(typography) = &tokens.typography {
        apply_typography(theme, typography);
    }
    if let Some(shape) = &tokens.shape {
        apply_shape(theme, shape);
    }
    if let Some(background) = &tokens.background {
        apply_background(theme, background);
    }
}

fn apply_palette(theme: &mut EffectiveTheme, palette: &ThemePalette) {
    if let Some(value) = &palette.primary {
        theme.primary = value.clone();
    }
    if let Some(value) = &palette.secondary {
        theme.secondary = value.clone();
    }
    if let Some(value) = &palette.accent {
        theme.accent = value.clone();
    }
    if let Some(value) = &palette.background {
        theme.background_solid = value.clone();
    }
    if let Some(value) = &palette.surface {
        theme.surface = value.clone();
    }
    if let Some(value) = &palette.surface_muted {
        theme.surface_muted = value.clone();
    }
    if let Some(value) = &palette.text {
        theme.text = value.clone();
    }
    if let Some(value) = &palette.text_muted {
        theme.text_muted = value.clone();
    }
    if let Some(value) = &palette.border {
        theme.border = value.clone();
    }
    if let Some(value) = &palette.success {
        theme.success = value.clone();
    }
    if let Some(value) = &palette.warning {
        theme.warning = value.clone();
    }
    if let Some(value) = &palette.danger {
        theme.danger = value.clone();
    }
}

fn apply_typography(theme: &mut EffectiveTheme, typography: &ThemeTypography) {
    if let Some(value) = &typography.heading_font {
        theme.heading_font = font_stack(value);
    }
    if let Some(value) = &typography.body_font {
        theme.body_font = font_stack(value);
    }
    if let Some(value) = &typography.mono_font {
        theme.mono_font = font_stack(value);
    }
    if let Some(scale) = &typography.scale {
        match scale {
            TypeScale::Compact => {
                theme.heading_size_px = 44;
                theme.body_size_px = 15;
            }
            TypeScale::Comfortable => {
                theme.heading_size_px = 54;
                theme.body_size_px = 17;
            }
            TypeScale::Spacious => {
                theme.heading_size_px = 64;
                theme.body_size_px = 18;
            }
        }
    }
}

fn apply_shape(theme: &mut EffectiveTheme, shape: &ThemeShape) {
    if let Some(radius) = &shape.radius {
        theme.radius_px = match radius {
            RadiusPreset::Sharp => 8,
            RadiusPreset::Soft => 18,
            RadiusPreset::Rounded => 26,
            RadiusPreset::Pill => 999,
        };
    }
    if let Some(button_style) = &shape.button_style {
        theme.cta_style = button_style.clone();
    }
    if let Some(card_style) = &shape.card_style {
        theme.card_surface_css = card_surface_css(card_style);
    }
    if let Some(shadow) = &shape.shadow {
        theme.shadow_css = shadow_css(shadow);
    }
}

fn apply_background(theme: &mut EffectiveTheme, background: &ThemeBackground) {
    if let Some(style) = &background.style {
        theme.background_style = style.clone();
    }
    if let Some(solid) = &background.solid {
        theme.background_solid = solid.clone();
    }
    if let Some(gradient) = &background.gradient {
        if let Some(from) = &gradient.from {
            theme.background_gradient_from = from.clone();
        }
        if let Some(to) = &gradient.to {
            theme.background_gradient_to = to.clone();
        }
        if let Some(angle) = gradient.angle {
            theme.background_gradient_angle = angle;
        }
    }
    if let Some(image_url) = &background.image_url {
        theme.background_image_url = Some(image_url.clone());
    }
    if let Some(overlay_opacity) = background.overlay_opacity {
        theme.overlay_opacity = overlay_opacity;
    }
}

fn apply_copy(theme: &mut EffectiveTheme, copy: &ThemeCopy) {
    if let Some(value) = &copy.brand_name {
        theme.brand_name = value.clone();
    }
    if let Some(value) = &copy.tagline {
        theme.tagline = Some(value.clone());
    }
    if let Some(value) = &copy.default_headline {
        theme.headline = value.clone();
        theme.og_title = value.clone();
    }
    if let Some(value) = &copy.default_subheadline {
        theme.subheadline = Some(value.clone());
        theme.og_description = value.clone();
    }
    if let Some(value) = &copy.primary_cta_label {
        theme.primary_cta_label = Some(value.clone());
    }
    if let Some(value) = &copy.footer_text {
        theme.footer_text = Some(value.clone());
    }
}

fn apply_media(theme: &mut EffectiveTheme, media: &ThemeMedia) {
    if let Some(value) = &media.logo_url {
        theme.logo_url = Some(value.clone());
    }
    if let Some(value) = &media.wordmark_url {
        theme.wordmark_url = Some(value.clone());
    }
    if let Some(value) = &media.icon_url {
        theme.icon_url = Some(value.clone());
    }
    if let Some(value) = &media.hero_image_url {
        theme.hero_image_url = Some(value.clone());
    }
    if let Some(value) = &media.og_image_url {
        theme.og_image_url = Some(value.clone());
    }
}

fn apply_layout(theme: &mut EffectiveTheme, layout: &ThemeLayout) {
    if let Some(value) = &layout.template {
        theme.template = value.clone();
    }
    if let Some(value) = &layout.alignment {
        theme.alignment = value.clone();
    }
    if let Some(value) = &layout.content_width {
        theme.content_width = value.clone();
    }
}

fn apply_modules(theme: &mut EffectiveTheme, modules: &ThemeModules) {
    if let Some(value) = modules.show_logo {
        theme.show_logo = value;
    }
    if let Some(value) = modules.show_icon {
        theme.show_icon = value;
    }
    if let Some(value) = modules.show_hero_image {
        theme.show_hero_image = value;
    }
    if let Some(value) = modules.show_footer {
        theme.show_footer = value;
    }
    if let Some(value) = modules.show_store_badges {
        theme.show_store_badges = value;
    }
}

fn apply_seo(theme: &mut EffectiveTheme, seo: &ThemeSeo) {
    if let Some(value) = &seo.default_og_title_template {
        theme.og_title = value.replace("{{link.title}}", &theme.headline);
    }
    if let Some(value) = &seo.default_og_description_template {
        theme.og_description = value.replace(
            "{{link.description}}",
            theme.subheadline.as_deref().unwrap_or("Open in app"),
        );
    }
}

fn build_agent_panel(ctx: &LandingPageContext<'_>) -> String {
    let ac = ctx.agent_context;
    let link = ctx.link;
    let theme = ctx.theme.primary.as_str();

    let mut html = String::new();
    html.push_str(&format!(
        r#"<div class="badge"><svg width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="3" y="4" width="10" height="8" rx="2" stroke="{theme}" stroke-width="1.4"/><circle cx="6.25" cy="8" r="1" fill="{theme}"/><circle cx="9.75" cy="8" r="1" fill="{theme}"/><line x1="5" y1="3" x2="5" y2="4.5" stroke="{theme}" stroke-width="1.2" stroke-linecap="round"/><line x1="11" y1="3" x2="11" y2="4.5" stroke="{theme}" stroke-width="1.2" stroke-linecap="round"/></svg>Machine-Readable Link</div>"#,
        theme = html_escape(theme)
    ));
    html.push_str(
        r#"<p class="agent-tagline">This link is structured for both humans and AI agents.</p>"#,
    );
    html.push_str(r#"<div class="trust-group trust-verified"><div class="trust-group-header">Verified by Rift</div>"#);
    if let Some(domain) = ctx.tenant_domain {
        let check = if ctx.tenant_verified {
            r#"<span class="check">&#10003;</span>"#
        } else {
            ""
        };
        html.push_str(&format!(
            r#"<div class="trust-row"><span class="trust-label">Domain</span><span class="trust-value">{}{}</span></div>"#,
            html_escape(domain), check
        ));
    }
    let status_class = match ctx.link_status {
        "expired" => " expired",
        "flagged" => " flagged",
        _ => "",
    };
    html.push_str(&format!(
        r#"<div class="trust-row"><span class="trust-label">Status</span><span class="trust-value"><span class="status-dot{}"></span>{}</span></div>"#,
        status_class,
        html_escape(&ctx.link_status[..1].to_uppercase()) + &ctx.link_status[1..]
    ));
    html.push_str("</div>");

    if ac.is_some_and(|a| a.action.is_some() || a.cta.is_some() || a.description.is_some()) {
        let ac = ac.unwrap();
        html.push_str(r#"<div class="trust-group trust-creator"><div class="trust-group-header">Provided by link creator</div>"#);
        if let Some(action) = &ac.action {
            html.push_str(&format!(
                r#"<div class="trust-row"><span class="trust-label">Action</span><span class="trust-value">{}</span></div>"#,
                html_escape(action)
            ));
        }
        if let Some(cta) = &ac.cta {
            html.push_str(&format!(
                r#"<div class="trust-row"><span class="trust-label">CTA</span><span class="trust-value">{}</span></div>"#,
                html_escape(cta)
            ));
        }
        if let Some(desc) = &ac.description {
            html.push_str(&format!(
                r#"<div class="desc-block">{}</div>"#,
                html_escape(desc)
            ));
            if let Some(domain) = ctx.tenant_domain {
                html.push_str(&format!(
                    r#"<p class="attr-note">Provided by the owner of {}. Not independently verified.</p>"#,
                    html_escape(domain)
                ));
            }
        }
        html.push_str("</div>");
    }

    let mut dests = Vec::new();
    if let Some(v) = &link.ios_deep_link {
        dests.push(("iOS", v.as_str()));
    }
    if let Some(v) = &link.android_deep_link {
        dests.push(("Android", v.as_str()));
    }
    if let Some(v) = &link.web_url {
        dests.push(("Web", v.as_str()));
    }
    if let Some(v) = &link.ios_store_url {
        dests.push(("App Store", v.as_str()));
    }
    if let Some(v) = &link.android_store_url {
        dests.push(("Play Store", v.as_str()));
    }
    if !dests.is_empty() {
        html.push_str(r#"<div class="dest-section"><div class="dest-header">Destinations</div>"#);
        for (label, url) in &dests {
            let display_url = url
                .trim_start_matches("https://")
                .trim_start_matches("http://");
            html.push_str(&format!(
                r#"<div class="dest-item"><span class="dest-type">{}</span><span class="dest-arrow">&rarr;</span><a href="{}" class="dest-url">{}</a></div>"#,
                label,
                html_escape(url),
                html_escape(display_url)
            ));
        }
        html.push_str("</div>");
    }

    html.push_str(r#"<div class="agent-footer">"#);
    html.push_str(r#"<p class="powered">Powered by <a href="https://riftl.ink">Rift</a> &mdash; Deep links for humans and agents</p>"#);
    html.push_str(r#"<p class="hint">AI agents: request with Accept: application/json for structured data</p>"#);
    html.push_str("</div>");
    html
}

fn render_template(replacements: &[(&str, String)]) -> String {
    replacements
        .iter()
        .fold(LANDING_PAGE_TEMPLATE.to_string(), |html, (key, value)| {
            html.replace(key, value)
        })
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

fn button_class(style: &ButtonStyle) -> &'static str {
    match style {
        ButtonStyle::Solid => "btn-solid",
        ButtonStyle::Outline => "btn-outline",
        ButtonStyle::Soft => "btn-soft",
    }
}

fn js_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '<' => out.push_str("\\x3c"),
            '>' => out.push_str("\\x3e"),
            '&' => out.push_str("\\x26"),
            '/' => out.push_str("\\/"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn font_stack(font: &FontPreset) -> &'static str {
    match font {
        FontPreset::SystemSans => {
            "ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif"
        }
        FontPreset::ModernSans => "\"Avenir Next\", \"Segoe UI\", Helvetica, Arial, sans-serif",
        FontPreset::HumanistSans => "\"Gill Sans\", \"Trebuchet MS\", \"Segoe UI\", sans-serif",
        FontPreset::GeometricSans => "\"Futura\", \"Century Gothic\", \"Avenir Next\", sans-serif",
        FontPreset::EditorialSerif => {
            "\"Iowan Old Style\", \"Palatino Linotype\", \"Book Antiqua\", Georgia, serif"
        }
    }
}

fn shadow_css(shadow: &ShadowPreset) -> &'static str {
    match shadow {
        ShadowPreset::None => "none",
        ShadowPreset::Soft => "0 18px 45px rgba(15, 23, 42, 0.16)",
        ShadowPreset::Medium => "0 24px 64px rgba(15, 23, 42, 0.24)",
        ShadowPreset::Dramatic => "0 32px 90px rgba(0, 0, 0, 0.42)",
    }
}

fn card_surface_css(card_style: &CardStyle) -> String {
    match card_style {
        CardStyle::Flat => "var(--surface-muted)".to_string(),
        CardStyle::Elevated => {
            "linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0.01))".to_string()
        }
        CardStyle::Glass => {
            "linear-gradient(180deg, rgba(255,255,255,0.14), rgba(255,255,255,0.04))".to_string()
        }
    }
}

fn preferred_text_on(bg: &str) -> &'static str {
    let rgb = bg.trim().trim_start_matches('#');
    let expanded = match rgb.len() {
        3 => rgb.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => rgb.to_string(),
        _ => return "#FFFFFF",
    };

    let r = u8::from_str_radix(&expanded[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&expanded[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&expanded[4..6], 16).unwrap_or(0) as f64 / 255.0;
    let luminance =
        0.2126 * linear_channel(r) + 0.7152 * linear_channel(g) + 0.0722 * linear_channel(b);

    if luminance > 0.45 {
        "#000000"
    } else {
        "#FFFFFF"
    }
}

fn linear_channel(value: f64) -> f64 {
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn action_to_schema_type(action: &str) -> &'static str {
    match action {
        "buy" | "purchase" => "BuyAction",
        "reserve" | "book" => "ReserveAction",
        "watch" => "WatchAction",
        "play" => "PlayAction",
        "read" => "ReadAction",
        "download" | "install" => "DownloadAction",
        _ => "ViewAction",
    }
}

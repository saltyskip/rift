//! Smart landing page renderer used by `do_resolve` for browser-targeted
//! GETs against `/r/{link_id}` and `/{link_id}` (custom domain). Returns
//! HTML; the JSON resolve flow lives in `routes.rs`.
//!
//! The visual layer is the first-party `Default` template. It consumes a
//! [`LandingTheme`] (brand config) + the link's content, derives a full color
//! palette from the brand accent (`palette::derive_palette`), and renders
//! everything through CSS custom properties so a single template flatters many
//! brands without anyone being able to author trash.

use mongodb::bson::DateTime;
use serde_json::json;

use super::palette::{derive_palette, DerivedPalettes, Palette};
use super::routes::{append_query_param, html_escape};
use crate::core::platform::Os;
use crate::services::landing::models::{CornerStyle, FontPreset, LandingTheme, DEFAULT_ACCENT};
use crate::services::links::models::{AgentContext, Link, SocialPreview};

// ── Public surface ──

pub(crate) struct LandingPageContext<'a> {
    pub os: Os,
    pub link: &'a Link,
    pub link_id: &'a str,
    /// Resolved brand config — the renderer's sole branding input.
    pub theme: &'a LandingTheme,
    pub social_preview: Option<&'a SocialPreview>,
    pub agent_context: Option<&'a AgentContext>,
    pub link_status: &'a str,
    pub tenant_domain: Option<&'a str>,
    pub tenant_verified: bool,
    pub alternate_domain: Option<&'a str>,
}

pub(crate) fn render_smart_landing_page(ctx: &LandingPageContext) -> String {
    let theme = ctx.theme;
    let app_name_display = theme.brand_name.as_deref().unwrap_or("App");
    let os = ctx.os;
    let link = ctx.link;
    let platform_js = js_escape(os.as_str());

    // Derive the palette from the brand accent, then express it as CSS custom
    // properties. Every rule below references var(--…) so the dark/light split
    // and per-brand tint are data, not duplicated CSS.
    let palettes = derive_palette(
        theme.theme_color.as_deref().unwrap_or(DEFAULT_ACCENT),
        theme.color_scheme,
    );
    let css_vars = build_css_vars(&palettes, theme.font, theme.corner_style);

    let metadata_fallback = if ctx.social_preview.is_none() {
        social_preview_from_metadata(link.metadata.as_ref())
    } else {
        None
    };
    let effective_preview = ctx.social_preview.or(metadata_fallback.as_ref());

    let store_url = match os {
        Os::Ios => link.ios_store_url.as_deref().unwrap_or(""),
        Os::Android => link.android_store_url.as_deref().unwrap_or(""),
        Os::Mac => link.macos_store_url.as_deref().unwrap_or(""),
        Os::Windows => link.windows_store_url.as_deref().unwrap_or(""),
        Os::Other => "",
    };

    // Append the store's attribution parameter: Play Store install `referrer`,
    // Microsoft Store campaign `cid`. (App Store / Mac App Store carry no
    // referrer — attribution there is deferred via the clipboard on tap.)
    let store_url_attributed = if store_url.is_empty() {
        String::new()
    } else {
        match os {
            Os::Android => {
                append_query_param(store_url, "referrer", &format!("rift_link={}", ctx.link_id))
            }
            Os::Windows => append_query_param(store_url, "cid", ctx.link_id),
            _ => store_url.to_string(),
        }
    };
    let store_url_js = js_escape(&store_url_attributed);

    let web_url = link.web_url.as_deref().unwrap_or("");
    let web_url_js = js_escape(web_url);

    // Raw iOS App Store URL, passed separately so the client can correct an
    // iPad masquerading as macOS (iPadOS desktop mode reports a Mac UA/hint) —
    // see the touch check in the button script.
    let ios_store_url_js = js_escape(link.ios_store_url.as_deref().unwrap_or(""));

    // Alternate domain URL for the "Open in App" button (cross-domain Universal Link trigger).
    let alternate_url = ctx
        .alternate_domain
        .map(|d| format!("https://{}/{}", d, ctx.link_id))
        .unwrap_or_default();
    let alternate_url_js = js_escape(&alternate_url);

    let preview_title = effective_preview.and_then(|p| p.title.as_deref());
    let preview_description = effective_preview.and_then(|p| p.description.as_deref());
    let preview_image = effective_preview.and_then(|p| p.image_url.as_deref());
    let og_title = preview_title.unwrap_or(app_name_display);
    let og_description = preview_description.unwrap_or("Open in app");

    // CTA label: a brand override, else "Open in {brand}". The platform script
    // swaps in store/web verbs ("Get …", "Continue") when the app can't open.
    let cta_label = theme
        .cta_label
        .clone()
        .unwrap_or_else(|| format!("Open in {app_name_display}"));
    let cta_label_js = js_escape(&cta_label);

    let json_ld = build_json_ld(ctx, preview_title, preview_description);

    let og_image_tag = preview_image
        .map(|img| {
            format!(
                r#"    <meta property="og:image" content="{img}" />
    <meta name="twitter:image" content="{img}" />"#,
                img = html_escape(img)
            )
        })
        .unwrap_or_default();

    // Preview image rendered as a hero banner (previously only in OG tags).
    let hero_html = preview_image
        .map(|img| format!(r#"<img class="hero" src="{}" alt="" />"#, html_escape(img)))
        .unwrap_or_default();

    let icon_html = theme
        .icon_url
        .as_deref()
        .map(|url| {
            format!(
                r#"<img class="icon" src="{}" alt="{}" />"#,
                html_escape(url),
                html_escape(app_name_display),
            )
        })
        .unwrap_or_default();

    let title_html = preview_title
        .map(|t| format!(r#"<h1>{}</h1>"#, html_escape(t)))
        .unwrap_or_default();

    let desc_html = theme
        .tagline
        .as_deref()
        .or(preview_description)
        .map(|d| format!(r#"<p class="subtitle">{}</p>"#, html_escape(d)))
        .unwrap_or_default();

    let expiry_html = expiry_notice(link);

    let agent_description = ctx.agent_context.and_then(|ac| ac.description.as_deref());

    let meta_desc_tag = agent_description
        .or(preview_description)
        .map(|d| {
            format!(
                r#"    <meta name="description" content="{}" />"#,
                html_escape(d)
            )
        })
        .unwrap_or_default();

    let (agent_panel, layout_class) = if theme.show_agent_panel {
        (build_agent_panel(ctx), "split")
    } else {
        (String::new(), "split solo")
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{og_title} — Rift</title>
    <meta property="og:title" content="{og_title_escaped}" />
    <meta property="og:description" content="{og_desc_escaped}" />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content="{og_title_escaped}" />
    <meta name="twitter:description" content="{og_desc_escaped}" />
{meta_desc_tag}
{og_image_tag}
{json_ld}
    <style>
        {css_vars}
        *,*::before,*::after {{ box-sizing:border-box; margin:0; padding:0; }}
        body {{ font-family:var(--font); background:var(--bg); color:var(--text); min-height:100vh; display:flex; flex-direction:column; }}
        .split {{ display:flex; flex:1; min-height:100vh; }}
        .side-human {{ width:60%; display:flex; align-items:center; justify-content:center; padding:48px 40px; border-right:1px solid var(--border); }}
        .side-agent {{ width:40%; background:var(--surface); padding:36px 28px; display:flex; flex-direction:column; overflow-y:auto; }}
        .split.solo .side-human {{ width:100%; border-right:none; }}
        .split.solo .side-agent {{ display:none; }}
        .human-inner {{ text-align:center; max-width:360px; width:100%; }}
        .hero {{ width:100%; height:160px; object-fit:cover; border-radius:var(--radius); margin-bottom:22px; border:1px solid var(--border); }}
        .icon {{ width:72px; height:72px; border-radius:18px; margin-bottom:18px; box-shadow:0 6px 24px rgba(0,0,0,0.35); }}
        .brand {{ font-size:11px; font-weight:700; letter-spacing:3px; text-transform:uppercase; color:var(--accent); margin-bottom:18px; }}
        .human-inner h1 {{ font-size:26px; font-weight:700; line-height:1.25; margin-bottom:10px; letter-spacing:-0.01em; }}
        .human-inner .subtitle {{ font-size:15px; line-height:1.5; color:var(--text-muted); margin-bottom:32px; }}
        .btn {{ display:inline-flex; align-items:center; gap:8px; background:var(--accent); color:var(--accent-text); font-size:16px; font-weight:600; padding:15px 44px; border-radius:var(--radius-btn); text-decoration:none; box-shadow:0 8px 28px var(--accent-glow); transition:transform .12s ease, box-shadow .12s ease; }}
        .btn:hover {{ transform:translateY(-1px); box-shadow:0 12px 36px var(--accent-glow); }}
        .btn:active {{ transform:translateY(0); }}
        .sub {{ color:var(--text-muted); font-size:12px; margin-top:16px; }}
        .expiry {{ font-size:12px; color:var(--text-muted); margin-top:16px; }}
        .expiry.expired {{ color:#f59e0b; font-weight:600; }}
        .badge {{ display:inline-flex; align-items:center; gap:8px; background:color-mix(in srgb, var(--accent) 10%, transparent); border:1px solid color-mix(in srgb, var(--accent) 28%, transparent); border-radius:20px; padding:6px 14px; font-size:12px; font-weight:600; color:var(--accent); margin-bottom:8px; width:fit-content; }}
        .badge svg {{ flex-shrink:0; stroke:currentColor; }}
        .agent-tagline {{ font-size:13px; color:var(--text-muted); margin-bottom:24px; }}
        .trust-group {{ border-radius:var(--radius); background:var(--bg); border:1px solid var(--border); padding:16px 18px; margin-bottom:12px; }}
        .trust-group-header {{ font-size:10px; font-weight:700; letter-spacing:1.5px; text-transform:uppercase; margin-bottom:12px; display:flex; align-items:center; gap:8px; }}
        .trust-verified {{ border-left:3px solid var(--accent); }}
        .trust-verified .trust-group-header {{ color:var(--accent); }}
        .trust-creator {{ border-left:3px solid var(--border); }}
        .trust-creator .trust-group-header {{ color:var(--text-muted); }}
        .trust-row {{ display:flex; align-items:baseline; margin-bottom:8px; font-size:13px; line-height:1.5; }}
        .trust-row:last-child {{ margin-bottom:0; }}
        .trust-label {{ color:var(--text-muted); min-width:80px; flex-shrink:0; font-size:12px; }}
        .trust-value {{ color:var(--text); font-size:13px; }}
        .trust-value .check {{ color:var(--accent); margin-left:4px; }}
        .status-dot {{ display:inline-block; width:7px; height:7px; border-radius:50%; background:#22c55e; margin-right:6px; vertical-align:middle; position:relative; top:-1px; }}
        .status-dot.expired {{ background:#ef4444; }}
        .status-dot.flagged {{ background:#f59e0b; }}
        .desc-block {{ margin-top:10px; padding:12px 14px; background:var(--surface); border-radius:8px; border:1px solid var(--border); font-size:12px; line-height:1.65; color:var(--text-muted); }}
        .attr-note {{ font-size:11px; color:var(--text-muted); margin-top:8px; font-style:italic; }}
        .dest-section {{ margin-top:8px; }}
        .dest-header {{ font-size:11px; font-weight:700; color:var(--text-muted); text-transform:uppercase; letter-spacing:1px; margin-bottom:10px; }}
        .dest-item {{ display:flex; align-items:center; gap:8px; background:var(--bg); border:1px solid var(--border); border-radius:8px; padding:10px 14px; font-size:12px; margin-bottom:6px; }}
        .dest-type {{ color:var(--text-muted); min-width:70px; flex-shrink:0; font-weight:500; }}
        .dest-arrow {{ color:var(--text-muted); }}
        .dest-url {{ color:var(--accent); text-decoration:none; word-break:break-all; }}
        .dest-url:hover {{ text-decoration:underline; }}
        .agent-footer {{ margin-top:auto; padding-top:24px; }}
        .agent-footer .powered {{ font-size:12px; color:var(--text-muted); }}
        .agent-footer .powered a {{ color:var(--text-muted); text-decoration:none; }}
        .agent-footer .powered a:hover {{ color:var(--accent); }}
        .agent-footer .hint {{ font-size:10px; color:var(--text-muted); opacity:0.7; margin-top:6px; }}
        @media (max-width:767px) {{
            .split {{ flex-direction:column; min-height:auto; }}
            .side-human {{ width:100%; border-right:none; border-bottom:1px solid var(--border); padding:56px 24px; min-height:55vh; }}
            .side-agent {{ width:100%; padding:28px 20px; }}
            .human-inner h1 {{ font-size:23px; }}
        }}
    </style>
</head>
<body>
<div class="{layout_class}">
    <div class="side-human">
        <div class="human-inner">
            {hero_html}
            {icon_html}
            <div class="brand">{app_name_escaped}</div>
            {title_html}
            {desc_html}
            <a id="open-btn" class="btn" href="#">{cta_label_escaped}</a>
            <p class="sub" id="fallback-msg"></p>
            {expiry_html}
        </div>
    </div>
    <div class="side-agent">
        {agent_panel}
    </div>
</div>
    <script>
    (function() {{
        var platform = "{platform_js}";
        var storeUrl = "{store_url_js}";
        var iosStoreUrl = "{ios_store_url_js}";
        var webUrl = "{web_url_js}";
        var alternateUrl = "{alternate_url_js}";
        var ctaLabel = "{cta_label_js}";

        var btn = document.getElementById("open-btn");
        var msg = document.getElementById("fallback-msg");

        // iPadOS desktop mode reports a Mac User-Agent / Sec-CH-UA-Platform, so
        // the server detects "macos". A real Mac has no touch screen; an iPad
        // reports touch points. Correct it client-side so iPads route to the
        // iOS App Store, not the Mac App Store.
        if (platform === "macos" && /Macintosh/.test(navigator.userAgent)
            && (navigator.maxTouchPoints || 0) > 1) {{
            platform = "ios";
        }}

        // Copy link URL to clipboard on button tap (requires user gesture).
        btn.addEventListener("click", function() {{
            if (navigator.clipboard) {{
                navigator.clipboard.writeText(window.location.href).catch(function(){{}});
            }}
        }});

        if (platform === "ios") {{
            // Universal Link trampoline opens the app if installed; otherwise
            // the iOS App Store. `iosStoreUrl` (not the OS-selected storeUrl) so
            // a corrected iPad never falls through to the Mac App Store.
            if (alternateUrl) {{
                btn.href = alternateUrl;
                btn.textContent = ctaLabel;
            }} else if (iosStoreUrl) {{
                btn.href = iosStoreUrl;
                btn.textContent = "Get {app_name_escaped}";
            }} else if (webUrl) {{
                btn.href = webUrl;
                btn.textContent = "Continue";
            }}
        }} else if (platform === "android") {{
            if (alternateUrl) {{
                // Cross-domain hop triggers App Links. If app installed → opens.
                // If not → alternate domain redirects to the Play Store.
                btn.href = alternateUrl;
                btn.textContent = ctaLabel;
            }} else if (storeUrl) {{
                btn.href = storeUrl;
                btn.textContent = "Get {app_name_escaped}";
            }} else if (webUrl) {{
                btn.href = webUrl;
                btn.textContent = "Continue";
            }}
        }} else if (platform === "macos" || platform === "windows") {{
            // Desktop with a native store: prefer the store, fall back to web.
            if (storeUrl) {{
                btn.href = storeUrl;
                btn.textContent = "Get {app_name_escaped}";
            }} else if (webUrl) {{
                btn.href = webUrl;
                btn.textContent = "Continue";
            }}
        }} else {{
            if (webUrl) {{
                btn.href = webUrl;
                btn.textContent = "Continue";
            }} else if (storeUrl) {{
                btn.href = storeUrl;
                btn.textContent = "Get {app_name_escaped}";
            }}
        }}
    }})();
    </script>
</body>
</html>"##,
        og_title = html_escape(og_title),
        og_title_escaped = html_escape(og_title),
        og_desc_escaped = html_escape(og_description),
        meta_desc_tag = meta_desc_tag,
        og_image_tag = og_image_tag,
        json_ld = json_ld,
        css_vars = css_vars,
        hero_html = hero_html,
        icon_html = icon_html,
        app_name_escaped = html_escape(app_name_display),
        cta_label_escaped = html_escape(&cta_label),
        title_html = title_html,
        desc_html = desc_html,
        expiry_html = expiry_html,
        layout_class = layout_class,
        agent_panel = agent_panel,
        platform_js = platform_js,
        store_url_js = store_url_js,
        ios_store_url_js = ios_store_url_js,
        web_url_js = web_url_js,
        alternate_url_js = alternate_url_js,
        cta_label_js = cta_label_js,
    )
}

// ── Helpers ──

/// Emit the `:root` custom-property block (plus an optional
/// `prefers-color-scheme: light` override) from a derived palette.
fn build_css_vars(palettes: &DerivedPalettes, font: FontPreset, corners: CornerStyle) -> String {
    let (radius_card, radius_btn) = corner_radii(corners);
    let font_stack = font_stack(font);
    let root = render_palette_vars(&palettes.root);
    let mut css = format!(
        ":root {{ {root} --font:{font_stack}; --radius:{radius_card}; --radius-btn:{radius_btn}; }}"
    );
    if let Some(light) = &palettes.prefers_light {
        let light_vars = render_palette_vars(light);
        css.push_str(&format!(
            " @media (prefers-color-scheme: light) {{ :root {{ {light_vars} }} }}"
        ));
    }
    css
}

fn render_palette_vars(p: &Palette) -> String {
    format!(
        "--bg:{}; --surface:{}; --border:{}; --text:{}; --text-muted:{}; --accent:{}; --accent-text:{}; --accent-glow:{};",
        p.bg, p.surface, p.border, p.text, p.text_muted, p.accent, p.accent_text, p.accent_glow
    )
}

fn font_stack(font: FontPreset) -> &'static str {
    match font {
        FontPreset::System => "system-ui,-apple-system,'Segoe UI',sans-serif",
        FontPreset::Serif => "Georgia,'Times New Roman',serif",
        FontPreset::Rounded => {
            "ui-rounded,'SF Pro Rounded','Hiragino Maru Gothic ProN',system-ui,sans-serif"
        }
        FontPreset::Mono => "ui-monospace,'SF Mono','Cascadia Code',Menlo,monospace",
    }
}

/// `(card_radius, button_radius)` for a corner style.
fn corner_radii(corners: CornerStyle) -> (&'static str, &'static str) {
    match corners {
        CornerStyle::Sharp => ("4px", "4px"),
        CornerStyle::Rounded => ("14px", "10px"),
        CornerStyle::Pill => ("16px", "999px"),
    }
}

/// Human-readable expiry line, or empty when the link never expires.
fn expiry_notice(link: &Link) -> String {
    let Some(expires_at) = link.expires_at else {
        return String::new();
    };
    let remaining_ms = expires_at.timestamp_millis() - DateTime::now().timestamp_millis();
    if remaining_ms <= 0 {
        return r#"<p class="expiry expired">This link has expired</p>"#.to_string();
    }
    let days = remaining_ms / (1000 * 60 * 60 * 24);
    let label = match days {
        0 => "Expires today".to_string(),
        1 => "Expires in 1 day".to_string(),
        n => format!("Expires in {n} days"),
    };
    format!(r#"<p class="expiry">{label}</p>"#)
}

fn build_json_ld(
    ctx: &LandingPageContext,
    preview_title: Option<&str>,
    preview_description: Option<&str>,
) -> String {
    let Some(ac) = ctx.agent_context else {
        return String::new();
    };
    if ac.action.is_none() && ac.cta.is_none() && ac.description.is_none() {
        return String::new();
    }

    let action_type = ac
        .action
        .as_deref()
        .map(action_to_schema_type)
        .unwrap_or("ViewAction");

    let entry_points: Vec<_> = [
        (
            ctx.link.ios_deep_link.as_deref(),
            "http://schema.org/IOSPlatform",
        ),
        (
            ctx.link.android_deep_link.as_deref(),
            "http://schema.org/AndroidPlatform",
        ),
        (
            ctx.link.web_url.as_deref(),
            "http://schema.org/DesktopWebPlatform",
        ),
    ]
    .into_iter()
    .filter_map(|(opt, platform)| {
        opt.map(|url| {
            json!({
                "@type": "EntryPoint",
                "urlTemplate": url,
                "actionPlatform": platform,
            })
        })
    })
    .collect();

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

    // Add public preview info if available.
    if preview_title.is_some() || preview_description.is_some() {
        let mut product = json!({"@type": "Product"});
        if let Some(t) = preview_title {
            product["name"] = json!(t);
        }
        if let Some(d) = preview_description {
            product["description"] = json!(d);
        }
        action["object"] = product;
    }

    // Add provenance metadata.
    action["provider"] = json!({
        "@type": "Organization",
        "name": ctx.tenant_domain.unwrap_or("unknown"),
        "additionalProperty": [
            { "@type": "PropertyValue", "name": "status", "value": ctx.link_status },
            { "@type": "PropertyValue", "name": "verified", "value": ctx.tenant_verified },
        ]
    });

    let json_str = serde_json::to_string(&action).unwrap_or_default();
    // Escape </script> in JSON-LD to prevent XSS.
    let json_str = json_str.replace("</", "<\\/");
    format!(r#"    <script type="application/ld+json">{json_str}</script>"#)
}

fn action_to_schema_type(action: &str) -> &'static str {
    match action {
        "purchase" => "BuyAction",
        "subscribe" => "SubscribeAction",
        "signup" => "RegisterAction",
        "download" => "DownloadAction",
        "read" => "ReadAction",
        "book" => "ReserveAction",
        _ => "ViewAction",
    }
}

/// Legacy fallback: before `social_preview` existed, customers used `metadata.{title,description,image}`
/// for OG tags. Read those keys when the link has no `social_preview` so existing links don't silently
/// lose their previews on deploy.
fn social_preview_from_metadata(
    metadata: Option<&mongodb::bson::Document>,
) -> Option<SocialPreview> {
    let meta = metadata?;
    let title = meta.get_str("title").ok().map(str::to_string);
    let description = meta.get_str("description").ok().map(str::to_string);
    let image_url = meta.get_str("image").ok().map(str::to_string);
    if title.is_none() && description.is_none() && image_url.is_none() {
        return None;
    }
    Some(SocialPreview {
        title,
        description,
        image_url,
    })
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

fn build_agent_panel(ctx: &LandingPageContext) -> String {
    let ac = ctx.agent_context;
    let link = ctx.link;

    let mut html = String::new();

    // Badge — colors inherit from CSS vars (`.badge { color: var(--accent) }`),
    // so the inline SVG uses currentColor.
    html.push_str(
        r#"<div class="badge"><svg width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="3" y="4" width="10" height="8" rx="2" stroke="currentColor" stroke-width="1.4"/><circle cx="6.25" cy="8" r="1" fill="currentColor"/><circle cx="9.75" cy="8" r="1" fill="currentColor"/><line x1="5" y1="3" x2="5" y2="4.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><line x1="11" y1="3" x2="11" y2="4.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>Machine-Readable Link</div>"#,
    );
    html.push_str(
        r#"<p class="agent-tagline">This link is structured for both humans and AI agents.</p>"#,
    );

    // Verified by Rift
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

    // Provided by creator
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

    // Destinations
    let dests: Vec<(&str, &str)> = [
        ("iOS", link.ios_deep_link.as_deref()),
        ("Android", link.android_deep_link.as_deref()),
        ("Web", link.web_url.as_deref()),
        ("App Store", link.ios_store_url.as_deref()),
        ("Play Store", link.android_store_url.as_deref()),
        ("Mac App Store", link.macos_store_url.as_deref()),
        ("Microsoft Store", link.windows_store_url.as_deref()),
    ]
    .into_iter()
    .filter_map(|(label, opt)| opt.map(|v| (label, v)))
    .collect();
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

    // Footer
    if !ctx.theme.hide_powered_by {
        html.push_str(r#"<div class="agent-footer">"#);
        html.push_str(r#"<p class="powered">Powered by <a href="https://riftl.ink">Rift</a> &mdash; Deep links for humans and agents</p>"#);
        html.push_str(r#"<p class="hint">AI agents: request with Accept: application/json for structured data</p>"#);
        html.push_str("</div>");
    }

    html
}

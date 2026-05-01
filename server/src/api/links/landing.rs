//! Smart landing page renderer used by `do_resolve` for browser-targeted
//! GETs against `/r/{link_id}` and `/{link_id}` (custom domain). Returns
//! HTML; the JSON resolve flow lives in `routes.rs`.

use serde_json::json;

use super::routes::{html_escape, urlencoding, Platform};
use crate::services::links::models::{AgentContext, Link, SocialPreview};

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

// ── Smart Landing Page ──

pub(crate) struct LandingPageContext<'a> {
    pub platform: Platform,
    pub link: &'a Link,
    pub link_id: &'a str,
    pub app_name: Option<&'a str>,
    pub icon_url: Option<&'a str>,
    pub theme_color: Option<&'a str>,
    pub social_preview: Option<&'a SocialPreview>,
    pub agent_context: Option<&'a AgentContext>,
    pub link_status: &'a str,
    pub tenant_domain: Option<&'a str>,
    pub tenant_verified: bool,
    pub alternate_domain: Option<&'a str>,
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

pub(crate) fn render_smart_landing_page(ctx: &LandingPageContext) -> String {
    let app_name_display = ctx.app_name.unwrap_or("App");
    let theme = ctx.theme_color.unwrap_or("#0d9488");
    let platform = ctx.platform;
    let link = ctx.link;
    let platform_js = js_escape(platform.as_str());

    let metadata_fallback = if ctx.social_preview.is_none() {
        social_preview_from_metadata(link.metadata.as_ref())
    } else {
        None
    };
    let effective_preview = ctx.social_preview.or(metadata_fallback.as_ref());

    let store_url = match platform {
        Platform::Ios => link.ios_store_url.as_deref().unwrap_or(""),
        Platform::Android => link.android_store_url.as_deref().unwrap_or(""),
        Platform::Other => "",
    };

    // For Android, append referrer with link_id to store URL.
    let store_url_with_referrer = if platform == Platform::Android && !store_url.is_empty() {
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

    let json_ld = if let Some(ac) = ctx.agent_context {
        if ac.action.is_some() || ac.cta.is_some() || ac.description.is_some() {
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

    let og_image_tag = preview_image
        .map(|img| {
            format!(
                r#"    <meta property="og:image" content="{img}" />
    <meta name="twitter:image" content="{img}" />"#,
                img = html_escape(img)
            )
        })
        .unwrap_or_default();

    let icon_html = ctx
        .icon_url
        .map(|url| {
            format!(
                r#"<img src="{}" alt="{}" style="width:64px;height:64px;border-radius:14px;margin-bottom:16px;" />"#,
                html_escape(url),
                html_escape(app_name_display),
            )
        })
        .unwrap_or_default();

    let title_html = preview_title
        .map(|t| {
            format!(
                r#"<h1 style="font-size:20px;font-weight:600;margin-bottom:8px;">{}</h1>"#,
                html_escape(t)
            )
        })
        .unwrap_or_default();

    let desc_html = preview_description
        .map(|d| {
            format!(
                r#"<p style="color:#a3a3a3;font-size:14px;margin-bottom:8px;">{}</p>"#,
                html_escape(d)
            )
        })
        .unwrap_or_default();

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

    let agent_panel = build_agent_panel(ctx);

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
        *,*::before,*::after {{ box-sizing:border-box; margin:0; padding:0; }}
        body {{ font-family:system-ui,-apple-system,sans-serif; background:#0a0a0a; color:#fafafa; min-height:100vh; display:flex; flex-direction:column; }}
        .split {{ display:flex; flex:1; min-height:100vh; }}
        .side-human {{ width:60%; display:flex; align-items:center; justify-content:center; padding:48px 40px; border-right:1px solid #1e1e22; }}
        .side-agent {{ width:40%; background:#0d0d0f; padding:36px 28px; display:flex; flex-direction:column; overflow-y:auto; }}
        .human-inner {{ text-align:center; max-width:320px; }}
        .brand {{ font-size:11px; font-weight:700; letter-spacing:3px; text-transform:uppercase; color:{theme}; margin-bottom:20px; }}
        .human-inner h1 {{ font-size:22px; font-weight:700; line-height:1.3; margin-bottom:8px; }}
        .human-inner .subtitle {{ font-size:14px; color:#71717a; margin-bottom:32px; }}
        .btn {{ display:inline-block; background:{theme}; color:#fff; font-size:15px; font-weight:600; padding:14px 40px; border-radius:10px; text-decoration:none; }}
        .btn:hover {{ opacity:0.9; }}
        .sub {{ color:#737373; font-size:12px; margin-top:16px; }}
        .badge {{ display:inline-flex; align-items:center; gap:8px; background:rgba(13,148,136,0.08); border:1px solid rgba(13,148,136,0.25); border-radius:20px; padding:6px 14px; font-size:12px; font-weight:600; color:{theme}; margin-bottom:8px; width:fit-content; }}
        .badge svg {{ flex-shrink:0; }}
        .agent-tagline {{ font-size:13px; color:#52525b; margin-bottom:24px; }}
        .trust-group {{ border-radius:10px; background:#111113; border:1px solid #1e1e22; padding:16px 18px; margin-bottom:12px; }}
        .trust-group-header {{ font-size:10px; font-weight:700; letter-spacing:1.5px; text-transform:uppercase; margin-bottom:12px; display:flex; align-items:center; gap:8px; }}
        .trust-verified {{ border-left:3px solid {theme}; }}
        .trust-verified .trust-group-header {{ color:{theme}; }}
        .trust-creator {{ border-left:3px solid #3f3f46; }}
        .trust-creator .trust-group-header {{ color:#71717a; }}
        .trust-row {{ display:flex; align-items:baseline; margin-bottom:8px; font-size:13px; line-height:1.5; }}
        .trust-row:last-child {{ margin-bottom:0; }}
        .trust-label {{ color:#71717a; min-width:80px; flex-shrink:0; font-size:12px; }}
        .trust-value {{ color:#fafafa; font-size:13px; }}
        .trust-value .check {{ color:{theme}; margin-left:4px; }}
        .status-dot {{ display:inline-block; width:7px; height:7px; border-radius:50%; background:#22c55e; margin-right:6px; vertical-align:middle; position:relative; top:-1px; }}
        .status-dot.expired {{ background:#ef4444; }}
        .status-dot.flagged {{ background:#f59e0b; }}
        .desc-block {{ margin-top:10px; padding:12px 14px; background:#0d0d0f; border-radius:8px; border:1px solid #1e1e22; font-size:12px; line-height:1.65; color:#a1a1aa; }}
        .attr-note {{ font-size:11px; color:#52525b; margin-top:8px; font-style:italic; }}
        .dest-section {{ margin-top:8px; }}
        .dest-header {{ font-size:11px; font-weight:700; color:#71717a; text-transform:uppercase; letter-spacing:1px; margin-bottom:10px; }}
        .dest-item {{ display:flex; align-items:center; gap:8px; background:#111113; border:1px solid #1e1e22; border-radius:8px; padding:10px 14px; font-size:12px; margin-bottom:6px; }}
        .dest-type {{ color:#71717a; min-width:70px; flex-shrink:0; font-weight:500; }}
        .dest-arrow {{ color:#3f3f46; }}
        .dest-url {{ color:{theme}; text-decoration:none; word-break:break-all; }}
        .dest-url:hover {{ text-decoration:underline; }}
        .agent-footer {{ margin-top:auto; padding-top:24px; }}
        .agent-footer .powered {{ font-size:12px; color:#52525b; }}
        .agent-footer .powered a {{ color:#71717a; text-decoration:none; }}
        .agent-footer .powered a:hover {{ color:{theme}; }}
        .agent-footer .hint {{ font-size:10px; color:#3f3f46; margin-top:6px; }}
        @media (max-width:767px) {{
            .split {{ flex-direction:column; min-height:auto; }}
            .side-human {{ width:100%; border-right:none; border-bottom:1px solid #1e1e22; padding:56px 24px; min-height:55vh; }}
            .side-agent {{ width:100%; padding:28px 20px; }}
        }}
    </style>
</head>
<body>
<div class="split">
    <div class="side-human">
        <div class="human-inner">
            {icon_html}
            <div class="brand">{app_name_escaped}</div>
            {title_html}
            {desc_html}
            <a id="open-btn" class="btn" href="#">Open in {app_name_escaped}</a>
            <p class="sub" id="fallback-msg"></p>
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
        var webUrl = "{web_url_js}";
        var alternateUrl = "{alternate_url_js}";

        var btn = document.getElementById("open-btn");
        var msg = document.getElementById("fallback-msg");

        // Copy link URL to clipboard on button tap (requires user gesture).
        btn.addEventListener("click", function() {{
            if (navigator.clipboard) {{
                navigator.clipboard.writeText(window.location.href).catch(function(){{}});
            }}
        }});

        if (platform === "ios" || platform === "android") {{
            if (alternateUrl) {{
                // Cross-domain hop triggers Universal Links / App Links.
                // If app installed → opens. If not → alternate domain redirects to store.
                btn.href = alternateUrl;
                btn.textContent = "Open in {app_name_escaped}";
            }} else if (storeUrl) {{
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
        theme = html_escape(theme),
        icon_html = icon_html,
        app_name_escaped = html_escape(app_name_display),
        title_html = title_html,
        desc_html = desc_html,
        agent_panel = agent_panel,
        platform_js = platform_js,
        store_url_js = store_url_js,
        web_url_js = web_url_js,
        alternate_url_js = alternate_url_js,
    )
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
    let theme = ctx.theme_color.unwrap_or("#0d9488");

    let mut html = String::new();

    // Badge
    html.push_str(&format!(
        r#"<div class="badge"><svg width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="3" y="4" width="10" height="8" rx="2" stroke="{theme}" stroke-width="1.4"/><circle cx="6.25" cy="8" r="1" fill="{theme}"/><circle cx="9.75" cy="8" r="1" fill="{theme}"/><line x1="5" y1="3" x2="5" y2="4.5" stroke="{theme}" stroke-width="1.2" stroke-linecap="round"/><line x1="11" y1="3" x2="11" y2="4.5" stroke="{theme}" stroke-width="1.2" stroke-linecap="round"/></svg>Machine-Readable Link</div>"#,
        theme = html_escape(theme)
    ));
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
    html.push_str(r#"<div class="agent-footer">"#);
    html.push_str(r#"<p class="powered">Powered by <a href="https://riftl.ink">Rift</a> &mdash; Deep links for humans and agents</p>"#);
    html.push_str(r#"<p class="hint">AI agents: request with Accept: application/json for structured data</p>"#);
    html.push_str("</div>");

    html
}

//! Shared HTML-interstitial renderer for email-link flows that consume
//! single-use tokens (magic-link signin, team-invite verify).
//!
//! Corporate email security products (Avanan, Microsoft Defender Safe Links,
//! ProofPoint URL Defense, Mimecast) pre-fetch links via GET to scan them.
//! Single-use tokens consumed on GET get burned by the scanner before the
//! human's click reaches the server. The fix: GET renders a form, POST
//! consumes — scanners follow GETs but don't submit forms.
//!
//! Each flow has its own copy (title / body / button) but the form shape,
//! cache headers, and HTML scaffold are identical — hence this module.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

/// Customization knobs for the interstitial page. The token (and optional
/// `next`) are always rendered as hidden form fields submitting to `action`.
pub(crate) struct InterstitialContent<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub button: &'a str,
}

/// Render the interstitial HTML response. Sets `Content-Type: text/html`,
/// `Cache-Control: no-store` (token URLs shouldn't sit in caches), and
/// `X-Robots-Tag: noindex` (in case a stray link ever escapes into the wild).
pub(crate) fn render(
    action: &str,
    token: &str,
    next: Option<&str>,
    content: InterstitialContent<'_>,
) -> Response {
    let html = build_html(action, token, next, content);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header("X-Robots-Tag", "noindex")
        .body(axum::body::Body::from(html))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ── Helpers ──

fn build_html(
    action: &str,
    token: &str,
    next: Option<&str>,
    content: InterstitialContent<'_>,
) -> String {
    let token_attr = html_escape(token);
    let next_field = next
        .map(|n| {
            format!(
                r#"<input type="hidden" name="next" value="{}">"#,
                html_escape(n)
            )
        })
        .unwrap_or_default();
    let action_attr = html_escape(action);
    let title = html_escape(content.title);
    let body = html_escape(content.body);
    let button = html_escape(content.button);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>{title}</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; background: #fafafa; color: #18181b; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; padding: 20px; }}
  .card {{ max-width: 420px; width: 100%; background: #fff; border: 1px solid #e4e4e7; border-radius: 12px; padding: 32px; text-align: center; box-sizing: border-box; }}
  h1 {{ margin: 0 0 8px; font-size: 20px; font-weight: 600; }}
  p {{ color: #71717a; margin: 0 0 24px; font-size: 14px; line-height: 1.5; }}
  button {{ background: #0d9488; color: #fff; border: none; border-radius: 6px; padding: 12px 24px; font-size: 15px; font-weight: 500; cursor: pointer; width: 100%; }}
  button:hover {{ background: #0f766e; }}
</style>
</head>
<body>
<div class="card">
<h1>{title}</h1>
<p>{body}</p>
<form method="post" action="{action_attr}">
<input type="hidden" name="token" value="{token_attr}">
{next_field}
<button type="submit">{button}</button>
</form>
</div>
</body>
</html>"#
    )
}

/// Escape the five characters that matter inside HTML attribute values
/// and PCDATA. Cheap to apply uniformly; safer than reasoning about which
/// inputs are already URL-safe.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

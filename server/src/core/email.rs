use serde_json::json;

#[cfg(test)]
#[path = "email_tests.rs"]
mod tests;

/// Send an email via the Resend API.
pub async fn send_email(
    resend_api_key: &str,
    from: &str,
    to: &str,
    subject: &str,
    html: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let body = json!({
        "from": from,
        "to": [to],
        "subject": subject,
        "html": html,
    });

    let resp = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {resend_api_key}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Resend API error {status}: {text}"))
    }
}

// ── Branded HTML chrome ──
//
// Every transactional email (signin, invite, key-creation code) shares the
// same outer shell and brand footer. These helpers own that chrome so the
// wrapper, button color, and footer copy live in one place; callers supply
// only the inner content (`heading` + body + `fine_print`).

/// Wrap inner `body` HTML in Rift's standard email shell + footer.
pub fn branded_html(body: &str) -> String {
    format!(
        r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                {body}
                <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 32px 0;" />
                <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
            </div>"#
    )
}

/// Standard email heading (the `<h2>` at the top of the body).
pub fn heading(text: &str) -> String {
    format!(r#"<h2 style="margin-bottom: 24px;">{text}</h2>"#)
}

/// The primary call-to-action button (teal, links to `href`).
pub fn cta_button(label: &str, href: &str) -> String {
    format!(
        r#"<a href="{href}" style="display: inline-block; padding: 12px 24px; background: #0d9488; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0;">{label}</a>"#
    )
}

/// Muted fine-print line (expiry notice, safe-to-ignore copy).
pub fn fine_print(text: &str) -> String {
    format!(r#"<p style="color: #71717a; font-size: 13px; margin-top: 24px;">{text}</p>"#)
}

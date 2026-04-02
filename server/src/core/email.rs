use serde_json::json;

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

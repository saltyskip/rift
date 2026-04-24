use super::*;

fn sign(secret: &str, ts: i64, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(ts.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

#[test]
fn verifies_correct_signature() {
    let secret = "whsec_test";
    let body = b"{\"id\":\"evt_123\"}";
    let ts = 1_000_000;
    let sig = sign(secret, ts, body);
    let header = format!("t={ts},v1={sig}");
    assert!(verify_webhook_signature(secret, &header, body, ts).is_ok());
}

#[test]
fn rejects_bad_signature() {
    let secret = "whsec_test";
    let body = b"{\"id\":\"evt_123\"}";
    let ts = 1_000_000;
    let sig = sign("whsec_wrong", ts, body);
    let header = format!("t={ts},v1={sig}");
    let err = verify_webhook_signature(secret, &header, body, ts).unwrap_err();
    assert!(matches!(err, WebhookVerifyError::SignatureMismatch));
}

#[test]
fn rejects_altered_body() {
    let secret = "whsec_test";
    let body = b"{\"id\":\"evt_123\"}";
    let ts = 1_000_000;
    let sig = sign(secret, ts, body);
    let header = format!("t={ts},v1={sig}");
    let err = verify_webhook_signature(secret, &header, b"{\"id\":\"evt_456\"}", ts).unwrap_err();
    assert!(matches!(err, WebhookVerifyError::SignatureMismatch));
}

#[test]
fn rejects_stale_timestamp() {
    let secret = "whsec_test";
    let body = b"{\"id\":\"evt_123\"}";
    let ts = 1_000_000;
    let sig = sign(secret, ts, body);
    let header = format!("t={ts},v1={sig}");
    // 400 seconds later is outside the 5-min tolerance.
    let err = verify_webhook_signature(secret, &header, body, ts + 400).unwrap_err();
    assert!(matches!(err, WebhookVerifyError::TimestampTooOld));
}

#[test]
fn accepts_multiple_v1_signatures_any_valid() {
    let secret = "whsec_test";
    let body = b"body";
    let ts = 1_000_000;
    let good = sign(secret, ts, body);
    let header = format!("t={ts},v1=badsig000,v1={good}");
    assert!(verify_webhook_signature(secret, &header, body, ts).is_ok());
}

#[test]
fn malformed_header_rejected() {
    let err = verify_webhook_signature("s", "not_a_header", b"body", 0).unwrap_err();
    assert!(matches!(err, WebhookVerifyError::BadHeader));
}

use super::*;
use mongodb::bson::{oid::ObjectId, DateTime};

fn test_source() -> Source {
    Source {
        id: ObjectId::new(),
        tenant_id: ObjectId::new(),
        name: "test".to_string(),
        source_type: SourceType::Custom,
        url_token: "test_token".to_string(),
        signing_secret: None,
        config: Document::new(),
        created_at: DateTime::now(),
    }
}

#[test]
fn parses_minimal_payload() {
    let body = br#"{"user_id":"u1","type":"signup"}"#;
    let out = CustomParser.parse(body, &test_source()).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].user_id.as_deref(), Some("u1"));
    assert_eq!(out[0].conversion_type, "signup");
}

#[test]
fn parses_full_payload() {
    let body = br#"{
        "user_id": "u1",
        "type": "deposit",
        "idempotency_key": "tx_001",
        "metadata": {"tx_hash": "0xabc"}
    }"#;
    let out = CustomParser.parse(body, &test_source()).unwrap();
    let p = &out[0];
    assert_eq!(p.idempotency_key.as_deref(), Some("tx_001"));
    assert!(p.metadata.is_some());
}

#[test]
fn rejects_missing_user_id() {
    let body = br#"{"type":"deposit"}"#;
    assert!(matches!(
        CustomParser.parse(body, &test_source()),
        Err(ParseError::InvalidPayload(_))
    ));
}

#[test]
fn rejects_empty_user_id() {
    let body = br#"{"user_id":"","type":"deposit"}"#;
    assert!(matches!(
        CustomParser.parse(body, &test_source()),
        Err(ParseError::MissingField("user_id"))
    ));
}

#[test]
fn rejects_missing_type() {
    let body = br#"{"user_id":"u1"}"#;
    assert!(matches!(
        CustomParser.parse(body, &test_source()),
        Err(ParseError::InvalidPayload(_))
    ));
}

#[test]
fn rejects_oversized_idempotency_key() {
    let long_key = "a".repeat(257);
    let body = format!(r#"{{"user_id":"u1","type":"t","idempotency_key":"{long_key}"}}"#);
    assert!(matches!(
        CustomParser.parse(body.as_bytes(), &test_source()),
        Err(ParseError::IdempotencyKeyTooLong(_))
    ));
}

#[test]
fn rejects_oversized_metadata() {
    // Build a metadata object > 1KB.
    let big = "x".repeat(1100);
    let body = format!(r#"{{"user_id":"u1","type":"t","metadata":{{"blob":"{big}"}}}}"#);
    assert!(matches!(
        CustomParser.parse(body.as_bytes(), &test_source()),
        Err(ParseError::MetadataTooLarge(_))
    ));
}

use super::*;

#[test]
fn validate_partner_key_accepts_valid_slugs() {
    assert!(validate_partner_key("bcom").is_ok());
    assert!(validate_partner_key("bc").is_ok());
    assert!(validate_partner_key("partner-1").is_ok());
    assert!(validate_partner_key("a1b2c3").is_ok());
    assert!(validate_partner_key(&"a".repeat(32)).is_ok());
}

#[test]
fn validate_partner_key_rejects_too_short() {
    assert!(matches!(
        validate_partner_key("a"),
        Err(AffiliateError::InvalidPartnerKey(_))
    ));
}

#[test]
fn validate_partner_key_rejects_too_long() {
    assert!(matches!(
        validate_partner_key(&"a".repeat(33)),
        Err(AffiliateError::InvalidPartnerKey(_))
    ));
}

#[test]
fn validate_partner_key_rejects_uppercase() {
    assert!(matches!(
        validate_partner_key("Bcom"),
        Err(AffiliateError::InvalidPartnerKey(_))
    ));
}

#[test]
fn validate_partner_key_rejects_leading_or_trailing_hyphen() {
    assert!(matches!(
        validate_partner_key("-bcom"),
        Err(AffiliateError::InvalidPartnerKey(_))
    ));
    assert!(matches!(
        validate_partner_key("bcom-"),
        Err(AffiliateError::InvalidPartnerKey(_))
    ));
}

#[test]
fn validate_partner_key_rejects_special_chars() {
    assert!(matches!(
        validate_partner_key("b_com"),
        Err(AffiliateError::InvalidPartnerKey(_))
    ));
    assert!(matches!(
        validate_partner_key("b com"),
        Err(AffiliateError::InvalidPartnerKey(_))
    ));
}

#[test]
fn validate_name_rejects_empty_or_whitespace() {
    assert!(matches!(
        validate_name(""),
        Err(AffiliateError::InvalidName(_))
    ));
    assert!(matches!(
        validate_name("   "),
        Err(AffiliateError::InvalidName(_))
    ));
}

#[test]
fn validate_name_rejects_too_long() {
    assert!(matches!(
        validate_name(&"a".repeat(65)),
        Err(AffiliateError::InvalidName(_))
    ));
}

#[test]
fn validate_name_accepts_normal() {
    assert!(validate_name("Bcom").is_ok());
}

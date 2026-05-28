use mongodb::bson::oid::ObjectId;

use super::{AffiliateId, Id, ParseIdError, SourceId, TenantId, WebhookId, HEX_LEN};

#[test]
fn from_object_id_stores_hex() {
    let oid = ObjectId::new();
    let id: AffiliateId = AffiliateId::from_object_id(oid);
    assert_eq!(id.as_hex(), &oid.to_hex());
    assert_eq!(id.as_hex().len(), HEX_LEN);
}

#[test]
fn display_includes_prefix() {
    let oid = ObjectId::new();
    let id: AffiliateId = oid.into();
    assert_eq!(format!("{id}"), format!("aff_{}", oid.to_hex()));
}

#[test]
fn round_trip_to_object_id() {
    let oid = ObjectId::new();
    let id: WebhookId = oid.into();
    let back = id.to_object_id().unwrap();
    assert_eq!(oid, back);
}

#[test]
fn serialize_to_prefixed_string() {
    let oid = ObjectId::parse_str("665a1b2c3d4e5f6a7b8c9d0e").unwrap();
    let id: AffiliateId = oid.into();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"aff_665a1b2c3d4e5f6a7b8c9d0e\"");
}

#[test]
fn deserialize_from_prefixed_string() {
    let json = "\"aff_665a1b2c3d4e5f6a7b8c9d0e\"";
    let id: AffiliateId = serde_json::from_str(json).unwrap();
    assert_eq!(id.as_hex(), "665a1b2c3d4e5f6a7b8c9d0e");
}

#[test]
fn deserialize_rejects_wrong_prefix() {
    let json = "\"aff_665a1b2c3d4e5f6a7b8c9d0e\"";
    let err = serde_json::from_str::<WebhookId>(json).unwrap_err();
    assert!(err.to_string().contains("wrong prefix"), "got: {err}");
}

#[test]
fn deserialize_rejects_raw_hex_without_prefix() {
    let json = "\"665a1b2c3d4e5f6a7b8c9d0e\"";
    let err = serde_json::from_str::<AffiliateId>(json).unwrap_err();
    assert!(err.to_string().contains("missing"), "got: {err}");
}

#[test]
fn deserialize_rejects_uppercase_hex() {
    // ObjectId::to_hex() always produces lowercase; the wire format must match.
    let json = "\"aff_665A1B2C3D4E5F6A7B8C9D0E\"";
    let err = serde_json::from_str::<AffiliateId>(json).unwrap_err();
    assert!(err.to_string().contains("hex"), "got: {err}");
}

#[test]
fn parse_rejects_wrong_length() {
    let err = AffiliateId::parse("aff_665a1b2c3d4e").unwrap_err();
    assert!(matches!(
        err,
        ParseIdError::InvalidLength {
            expected: 24,
            got: 12
        }
    ));
}

#[test]
fn parse_rejects_non_hex_body() {
    let err = AffiliateId::parse("aff_zzzzzzzzzzzzzzzzzzzzzzzz").unwrap_err();
    assert!(matches!(err, ParseIdError::InvalidHex));
}

#[test]
fn parse_rejects_missing_separator() {
    let err = AffiliateId::parse("aff665a1b2c3d4e5f6a7b8c9d0e").unwrap_err();
    assert!(matches!(err, ParseIdError::MissingSeparator));
}

#[test]
fn fromstr_works() {
    let oid = ObjectId::new();
    let s = format!("tnt_{}", oid.to_hex());
    let id: TenantId = s.parse().unwrap();
    assert_eq!(id.as_hex(), &oid.to_hex());
}

#[test]
fn distinct_marker_types_have_distinct_schema_names() {
    use utoipa::ToSchema;
    assert_eq!(AffiliateId::name(), "AffiliateId");
    assert_eq!(TenantId::name(), "TenantId");
    assert_eq!(SourceId::name(), "SourceId");
    assert_eq!(WebhookId::name(), "WebhookId");
}

#[test]
fn utoipa_schema_has_lowercase_hex_pattern() {
    use utoipa::PartialSchema;
    let schema = AffiliateId::schema();
    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["pattern"], "^aff_[0-9a-f]{24}$");
}

#[cfg(feature = "mcp")]
#[test]
fn schemars_schema_has_lowercase_hex_pattern() {
    use schemars::JsonSchema;
    let mut gen = schemars::SchemaGenerator::default();
    let schema = AffiliateId::json_schema(&mut gen);
    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["pattern"], "^aff_[0-9a-f]{24}$");
}

#[test]
fn equality_within_same_type() {
    let oid = ObjectId::new();
    let a: AffiliateId = oid.into();
    let b: AffiliateId = oid.into();
    assert_eq!(a, b);
}

#[test]
fn ord_matches_hex_ord() {
    let mut ids: Vec<AffiliateId> = (0..5).map(|_| AffiliateId::from(ObjectId::new())).collect();
    let mut hexes: Vec<String> = ids.iter().map(|i| i.as_hex().to_string()).collect();
    ids.sort();
    hexes.sort();
    let after: Vec<String> = ids.iter().map(|i| i.as_hex().to_string()).collect();
    assert_eq!(after, hexes);
}

#[test]
fn hash_consistency() {
    use std::collections::HashSet;
    let oid = ObjectId::new();
    let id: AffiliateId = oid.into();
    let clone = id.clone();
    let mut set = HashSet::new();
    set.insert(id);
    assert!(set.contains(&clone));
}

// Compile-time: distinct marker types are not interchangeable.
// fn _no_cross_assignment() {
//     let a: AffiliateId = ObjectId::new().into();
//     let _w: WebhookId = a; // ERROR
// }

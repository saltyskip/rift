use super::{KeyScope, SecretKeyDoc};
use crate::core::public_id::{AffiliateId, SecretKeyId, TenantId, UserId};
use mongodb::bson;

fn doc_with(scope: Option<KeyScope>) -> SecretKeyDoc {
    SecretKeyDoc {
        id: SecretKeyId::new(),
        tenant_id: TenantId::new(),
        created_by: UserId::new(),
        key_hash: "deadbeef".to_string(),
        key_prefix: "rl_live_abc...".to_string(),
        created_at: bson::DateTime::now(),
        scope,
    }
}

// Faithful to the driver: `cursor.deserialize_current()` reads raw, NON-human-
// readable BSON. `bson::to_vec` / `from_slice` exercise that same path.
fn roundtrip_raw(doc: &SecretKeyDoc) -> Result<SecretKeyDoc, String> {
    let bytes = bson::to_vec(doc).map_err(|e| e.to_string())?;
    bson::from_slice(&bytes).map_err(|e| e.to_string())
}

#[test]
fn full_scope_roundtrips_through_raw_bson() {
    let doc = doc_with(Some(KeyScope::Full));
    assert!(roundtrip_raw(&doc).is_ok(), "full scope should round-trip");
}

#[test]
fn affiliate_scope_roundtrips_through_raw_bson() {
    let doc = doc_with(Some(KeyScope::Affiliate {
        affiliate_id: AffiliateId::new(),
    }));
    let result = roundtrip_raw(&doc);
    assert!(
        result.is_ok(),
        "affiliate scope failed to deserialize from raw BSON: {:?}",
        result.err()
    );
}

use super::*;
use crate::services::auth::secret_keys::repo::KeyScope;
use mongodb::bson::oid::ObjectId;

#[test]
fn require_full_accepts_full_scope() {
    assert!(require_full(Some(&KeyScope::Full)).is_ok());
}

#[test]
fn require_full_grandfathers_none() {
    // Migration-window concession. Remove once m004 is confirmed deployed
    // and the follow-up PR ships.
    assert!(require_full(None).is_ok());
}

#[test]
fn require_full_rejects_affiliate_scope() {
    let scope = KeyScope::Affiliate {
        affiliate_id: ObjectId::new(),
    };
    assert_eq!(require_full(Some(&scope)), Err(ScopeError::Forbidden));
}

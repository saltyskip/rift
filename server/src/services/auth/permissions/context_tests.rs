use super::super::models::{AuthContext, AuthzError, Permission, Principal, ResourceScope, Scopes};
use crate::core::public_id::{TenantId, UserId};
use crate::services::auth::secret_keys::repo::KeyScope;
use mongodb::bson::oid::ObjectId;

fn user_ctx() -> AuthContext {
    AuthContext::for_session(
        TenantId::new(),
        UserId::new(),
        crate::core::public_id::AuthSessionId::new(),
    )
}

#[test]
fn session_has_full_scope() {
    let ctx = user_ctx();
    assert!(ctx.require(Permission::LinksWrite).is_ok());
    assert!(ctx.require(Permission::TenantAdmin).is_ok());
    assert!(matches!(ctx.resource_scope, ResourceScope::Tenant));
}

#[test]
fn secret_key_full_has_full_scope() {
    let ctx = AuthContext::for_secret_key(TenantId::new(), ObjectId::new(), Some(&KeyScope::Full));
    assert!(ctx.require(Permission::AffiliatesWrite).is_ok());
}

#[test]
fn secret_key_missing_scope_grandfathered_to_full() {
    let ctx = AuthContext::for_secret_key(TenantId::new(), ObjectId::new(), None);
    assert!(ctx.require(Permission::WebhooksWrite).is_ok());
}

#[test]
fn secret_key_affiliate_has_only_links_scope() {
    let affiliate_id = ObjectId::new();
    let ctx = AuthContext::for_secret_key(
        TenantId::new(),
        ObjectId::new(),
        Some(&KeyScope::Affiliate {
            affiliate_id: crate::core::public_id::AffiliateId::from_object_id(affiliate_id),
        }),
    );
    assert!(ctx.require(Permission::LinksRead).is_ok());
    assert!(ctx.require(Permission::LinksWrite).is_ok());
    assert_eq!(
        ctx.require(Permission::AffiliatesWrite).unwrap_err(),
        AuthzError::MissingPermission(Permission::AffiliatesWrite)
    );
    assert!(matches!(
        ctx.resource_scope,
        ResourceScope::Affiliate { affiliate_id: a } if a.to_object_id() == affiliate_id
    ));
}

#[test]
fn require_any_succeeds_if_one_matches() {
    let ctx = AuthContext::for_secret_key(
        TenantId::new(),
        ObjectId::new(),
        Some(&KeyScope::Affiliate {
            affiliate_id: crate::core::public_id::AffiliateId::new(),
        }),
    );
    assert!(ctx
        .require_any(&[Permission::TenantAdmin, Permission::LinksRead])
        .is_ok());
}

#[test]
fn require_any_fails_when_none_match() {
    let ctx = AuthContext::for_secret_key(
        TenantId::new(),
        ObjectId::new(),
        Some(&KeyScope::Affiliate {
            affiliate_id: crate::core::public_id::AffiliateId::new(),
        }),
    );
    let err = ctx
        .require_any(&[Permission::TenantAdmin, Permission::DomainsWrite])
        .unwrap_err();
    assert!(matches!(err, AuthzError::AnyOfMissing(_)));
}

#[test]
fn principal_carries_correct_kind() {
    let session = user_ctx();
    assert!(matches!(session.principal, Principal::User { .. }));

    let key = AuthContext::for_secret_key(TenantId::new(), ObjectId::new(), Some(&KeyScope::Full));
    assert!(matches!(key.principal, Principal::SecretKey { .. }));
}

#[test]
fn scopes_full_contains_all_permissions() {
    let s = Scopes::full();
    // Sanity: a sampling that covers each resource family.
    for p in [
        Permission::LinksRead,
        Permission::LinksDelete,
        Permission::DomainsWrite,
        Permission::AppsRead,
        Permission::WebhooksRead,
        Permission::AffiliatesRead,
        Permission::ConversionsWrite,
        Permission::SecretKeysWrite,
        Permission::BillingRead,
        Permission::TenantAdmin,
    ] {
        assert!(s.contains(p), "Scopes::full missing {}", p.to_wire_str());
    }
}

#[test]
fn permission_wire_strings_are_unique() {
    use std::collections::HashSet;
    let perms = [
        Permission::LinksRead,
        Permission::LinksWrite,
        Permission::LinksDelete,
        Permission::DomainsRead,
        Permission::DomainsWrite,
        Permission::AppsRead,
        Permission::AppsWrite,
        Permission::WebhooksRead,
        Permission::WebhooksWrite,
        Permission::AffiliatesRead,
        Permission::AffiliatesWrite,
        Permission::ConversionsRead,
        Permission::ConversionsWrite,
        Permission::SecretKeysRead,
        Permission::SecretKeysWrite,
        Permission::BillingRead,
        Permission::BillingWrite,
        Permission::TenantAdmin,
    ];
    let strs: HashSet<&'static str> = perms.iter().map(|p| p.to_wire_str()).collect();
    assert_eq!(strs.len(), perms.len(), "duplicate wire string");
}

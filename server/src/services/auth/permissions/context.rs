//! Constructors + scope-check methods for `AuthContext`, plus `Scopes`
//! helpers. Implementation file; `pub` data types live in `models.rs`.

use super::models::{AuthContext, AuthzError, Permission, Principal, ResourceScope, Scopes};
use crate::services::auth::secret_keys::repo::KeyScope;
use mongodb::bson::oid::ObjectId;
use std::collections::BTreeSet;

impl AuthContext {
    /// Build context for a session-authenticated request. Sessions are always
    /// full tenant access — there's no affiliate-scoped human in Phase 1.
    pub fn for_session(tenant_id: ObjectId, user_id: ObjectId, session_id: ObjectId) -> Self {
        Self {
            tenant_id,
            principal: Principal::User {
                user_id,
                session_id,
            },
            permissions: Scopes::full(),
            resource_scope: ResourceScope::Tenant,
        }
    }

    /// Build context for a secret-key-authenticated request. `key_scope` is
    /// `None` for grandfathered pre-migration rows — treated as `Full`, same
    /// rule as `services/auth/scope::require_full`.
    pub fn for_secret_key(
        tenant_id: ObjectId,
        key_id: ObjectId,
        key_scope: Option<&KeyScope>,
    ) -> Self {
        let (permissions, resource_scope) = match key_scope {
            None | Some(KeyScope::Full) => (Scopes::full(), ResourceScope::Tenant),
            Some(KeyScope::Affiliate { affiliate_id }) => (
                Scopes::affiliate_partner(),
                ResourceScope::Affiliate {
                    affiliate_id: *affiliate_id,
                },
            ),
        };
        Self {
            tenant_id,
            principal: Principal::SecretKey { key_id },
            permissions,
            resource_scope,
        }
    }

    /// Guard the operation on a specific permission. Service methods call
    /// this at the top — usually injected by the `#[requires(...)]` macro.
    pub fn require(&self, perm: Permission) -> Result<(), AuthzError> {
        if self.permissions.contains(perm) {
            Ok(())
        } else {
            Err(AuthzError::MissingPermission(perm))
        }
    }

    /// Guard that the caller carries at least one of the listed permissions.
    /// Paired with the `#[requires_any(...)]` proc-macro. Currently unused
    /// by services but kept available for future multi-permission gates.
    #[allow(dead_code)]
    pub fn require_any(&self, perms: &[Permission]) -> Result<(), AuthzError> {
        if perms.iter().any(|p| self.permissions.contains(*p)) {
            Ok(())
        } else {
            Err(AuthzError::AnyOfMissing(perms.to_vec()))
        }
    }
}

impl Scopes {
    /// All permissions — used for session auth and pre-scope (`Full`) keys.
    pub fn full() -> Self {
        Self(all_permissions().collect())
    }

    /// Permissions a partner-scoped key carries. Pinning to a specific
    /// affiliate's resources is enforced at the repo layer via
    /// `ResourceScope::Affiliate`, not here.
    pub fn affiliate_partner() -> Self {
        Self(BTreeSet::from([
            Permission::LinksRead,
            Permission::LinksWrite,
        ]))
    }

    pub fn contains(&self, perm: Permission) -> bool {
        self.0.contains(&perm)
    }
}

// ── Helpers ──

fn all_permissions() -> impl Iterator<Item = Permission> {
    [
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
    ]
    .into_iter()
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;

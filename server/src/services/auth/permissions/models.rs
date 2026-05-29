//! Data types for service-layer authorization.

use std::collections::BTreeSet;
use std::fmt;

use crate::core::public_id::{AffiliateId, AuthSessionId, SecretKeyId, TenantId, UserId};

/// Closed set of operation types a caller can be authorized for. Wire
/// representation is `<resource>:<action>` (see `to_wire_str`) — used in
/// 403 error bodies and (future) OpenAPI security scope strings.
///
/// `Copy` so `require(Permission::X)` doesn't move; `Ord` so `Scopes` can
/// store them in a `BTreeSet` for deterministic iteration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Permission {
    LinksRead,
    LinksWrite,
    LinksDelete,
    DomainsRead,
    DomainsWrite,
    AppsRead,
    AppsWrite,
    WebhooksRead,
    WebhooksWrite,
    AffiliatesRead,
    AffiliatesWrite,
    ConversionsRead,
    ConversionsWrite,
    SecretKeysRead,
    SecretKeysWrite,
    BillingRead,
    BillingWrite,
    /// Team management, tenant-wide settings, tenant delete.
    TenantAdmin,
}

impl Permission {
    pub const fn to_wire_str(self) -> &'static str {
        match self {
            Self::LinksRead => "links:read",
            Self::LinksWrite => "links:write",
            Self::LinksDelete => "links:delete",
            Self::DomainsRead => "domains:read",
            Self::DomainsWrite => "domains:write",
            Self::AppsRead => "apps:read",
            Self::AppsWrite => "apps:write",
            Self::WebhooksRead => "webhooks:read",
            Self::WebhooksWrite => "webhooks:write",
            Self::AffiliatesRead => "affiliates:read",
            Self::AffiliatesWrite => "affiliates:write",
            Self::ConversionsRead => "conversions:read",
            Self::ConversionsWrite => "conversions:write",
            Self::SecretKeysRead => "secret_keys:read",
            Self::SecretKeysWrite => "secret_keys:write",
            Self::BillingRead => "billing:read",
            Self::BillingWrite => "billing:write",
            Self::TenantAdmin => "tenant:admin",
        }
    }
}

/// Set of permissions a caller carries. Backed by a `BTreeSet` for
/// deterministic ordering when surfacing the set (e.g. for diagnostics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scopes(pub(super) BTreeSet<Permission>);

/// Who is making the call. The orthogonal "what resources can they touch?"
/// dimension lives in `ResourceScope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Principal {
    /// Browser/dashboard session.
    User {
        user_id: UserId,
        session_id: AuthSessionId,
    },
    /// `rl_live_…` secret key.
    SecretKey { key_id: SecretKeyId },
}

/// Which subset of the tenant's resources the caller can act on. Distinct
/// from `Permission` (operation type) — a partner-scoped key may carry
/// `LinksWrite` but only on its own affiliate's links. Instance-level
/// filtering lives in the repos (`WHERE tenant_id = ? AND affiliate_id = ?`),
/// not in scope checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceScope {
    Tenant,
    Affiliate { affiliate_id: AffiliateId },
}

/// Unified identity injected into request extensions by the auth middleware.
/// Service methods take `&AuthContext` and call `ctx.require(...)?` (or the
/// `#[requires(...)]` proc-macro injects it for them).
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub tenant_id: TenantId,
    pub principal: Principal,
    pub permissions: Scopes,
    pub resource_scope: ResourceScope,
}

/// Caller is missing a permission required by the operation.
///
/// `AnyOfMissing` pairs with `#[requires_any(...)]` — currently no service
/// uses the multi-permission form, but it's part of the macro surface and
/// stays available for future use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthzError {
    MissingPermission(Permission),
    #[allow(dead_code)]
    AnyOfMissing(Vec<Permission>),
}

impl AuthzError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingPermission(_) | Self::AnyOfMissing(_) => "forbidden_permission",
        }
    }
}

impl fmt::Display for AuthzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPermission(p) => {
                write!(f, "Missing required permission: {}", p.to_wire_str())
            }
            Self::AnyOfMissing(ps) => {
                let names: Vec<&'static str> = ps.iter().map(|p| p.to_wire_str()).collect();
                write!(f, "Missing one of: {}", names.join(", "))
            }
        }
    }
}

impl std::error::Error for AuthzError {}

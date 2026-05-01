//! Axum extension types injected by `api/auth/middleware.rs` into request
//! extensions, then extracted by route handlers via `Extension<...>`.

use mongodb::bson::oid::ObjectId;

use crate::services::auth::secret_keys::repo::KeyScope;

/// Tenant identity injected by the auth middleware.
/// Handlers extract this via `Extension<TenantId>`.
#[derive(Debug, Clone)]
pub struct TenantId(pub ObjectId);

/// The ObjectId of the secret key used for authentication.
/// Handlers extract this via `Extension<AuthKeyId>`.
#[derive(Debug, Clone)]
pub struct AuthKeyId(pub ObjectId);

/// Domain associated with an SDK key, injected by `sdk_auth_gate`.
#[derive(Debug, Clone)]
pub struct SdkDomain(pub String);

/// Scope the calling key carries.
///
/// Always injected as `Extension<CallerScope>`, with `scope: None` for
/// pre-migration rows that haven't been backfilled yet (grandfathered to
/// `Full` per `services/auth/scope::require_full`). Affiliate-scoped keys
/// can only hit the path allowlist; everything else returns 403.
#[derive(Debug, Clone)]
pub struct CallerScope(pub Option<KeyScope>);

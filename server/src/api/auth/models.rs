//! Axum extension types injected by `api/auth/middleware.rs` into request
//! extensions, then extracted by route handlers via `Extension<...>`.
//!
//! `TenantId`, `UserId`, and `SessionId` (alias for `AuthSessionId`) are
//! re-exports of the typed identifiers from `core::public_id`. The middleware
//! constructs the typed value once at the auth boundary and the same type
//! flows all the way through services and repos.
//!
//! `AuthKeyId` and `SdkDomain` are still local newtypes — they aren't yet
//! migrated to typed `Id<P>` aliases (secret_keys migration pending).

use mongodb::bson::oid::ObjectId;

pub use crate::core::public_id::{AuthSessionId as SessionId, TenantId, UserId};

/// The ObjectId of the secret key used for authentication.
/// Handlers extract this via `Extension<AuthKeyId>`.
#[derive(Debug, Clone)]
pub struct AuthKeyId(pub ObjectId);

/// Domain associated with an SDK key, injected by `sdk_auth_gate`.
#[derive(Debug, Clone)]
pub struct SdkDomain(pub String);

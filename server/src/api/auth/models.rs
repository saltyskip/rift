//! Axum extension types injected by `api/auth/middleware.rs` into request
//! extensions, then extracted by route handlers via `Extension<...>`.
//!
//! `TenantId` and `UserId` are re-exports of the typed identifiers from
//! `core::public_id` — there's no separate axum-extension newtype. The
//! middleware constructs the typed value once at the auth boundary and the
//! same type flows all the way through services and repos.
//!
//! `AuthKeyId`, `SessionId`, `SdkDomain` are still local newtypes — they
//! aren't yet migrated to typed `Id<P>` aliases. Doing so is part of the
//! secret_keys / sessions migrations.

use mongodb::bson::oid::ObjectId;

pub use crate::core::public_id::{TenantId, UserId};

/// The ObjectId of the secret key used for authentication.
/// Handlers extract this via `Extension<AuthKeyId>`.
#[derive(Debug, Clone)]
pub struct AuthKeyId(pub ObjectId);

/// The active session's ObjectId — used by `POST /v1/auth/signout` to revoke
/// the exact session the caller arrived on.
#[derive(Debug, Clone)]
pub struct SessionId(pub ObjectId);

/// Domain associated with an SDK key, injected by `sdk_auth_gate`.
#[derive(Debug, Clone)]
pub struct SdkDomain(pub String);

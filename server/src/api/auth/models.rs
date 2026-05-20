//! Axum extension types injected by `api/auth/middleware.rs` into request
//! extensions, then extracted by route handlers via `Extension<...>`.
//!
//! Service-layer authorization travels through `AuthContext` (see
//! `services/auth/permissions/`). These extensions remain for the route
//! layer to use directly — `AuthKeyId` for the affiliate credential
//! provenance, `UserId`/`SessionId` for session-bound flows, `SdkDomain`
//! for the SDK path. `TenantId` is kept for routes that bypass the
//! service layer (e.g. webhook list/delete that call the repo directly).

use mongodb::bson::oid::ObjectId;

/// Tenant identity injected by the auth middleware.
/// Handlers extract this via `Extension<TenantId>`.
#[derive(Debug, Clone)]
pub struct TenantId(pub ObjectId);

/// The ObjectId of the secret key used for authentication.
/// Handlers extract this via `Extension<AuthKeyId>`.
#[derive(Debug, Clone)]
pub struct AuthKeyId(pub ObjectId);

/// Human identity for session-authenticated requests.
///
/// Only injected by `session_auth_gate` and `session_or_key_auth_gate` (when the
/// session path wins). Key-only routes never see this; session-only handlers
/// can extract it via `Extension<UserId>`. Handlers wrapped with `session_or_key_auth_gate`
/// should treat it as optional (`Option<Extension<UserId>>`).
#[derive(Debug, Clone)]
pub struct UserId(pub ObjectId);

/// The active session's ObjectId — used by `POST /v1/auth/signout` to revoke
/// the exact session the caller arrived on.
#[derive(Debug, Clone)]
pub struct SessionId(pub ObjectId);

/// Domain associated with an SDK key, injected by `sdk_auth_gate`.
#[derive(Debug, Clone)]
pub struct SdkDomain(pub String);

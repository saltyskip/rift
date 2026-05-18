//! Data types for `services/auth/sessions/` — DB document, error enum,
//! service config + return types.

use mongodb::bson::{self, oid::ObjectId};
use serde::{Deserialize, Serialize};
use std::fmt;

// ── DB Document ──

/// One row in the `sessions` collection. Represents a human signed into a
/// browser. Looked up by `token_hash` on every authenticated request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDoc {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub user_id: ObjectId,
    pub tenant_id: ObjectId,
    /// SHA-256 of the raw opaque token. The raw token only ever exists in the
    /// `Set-Cookie` header and in the client browser.
    pub token_hash: String,
    pub created_at: bson::DateTime,
    pub expires_at: bson::DateTime,
    pub last_seen_at: bson::DateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<bson::DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
}

// ── Service config ──

#[derive(Debug, Clone)]
pub struct SessionsConfig {
    /// Base URL used to build the magic-link callback (e.g. `https://api.riftl.ink`).
    pub public_url: String,
    pub resend_api_key: String,
    pub resend_from_email: String,
}

// ── Service return types ──

/// Resolved session lookup — what session middleware injects.
#[derive(Debug, Clone)]
pub struct ResolvedSession {
    pub session_id: ObjectId,
    pub user_id: ObjectId,
    pub tenant_id: ObjectId,
}

/// Returned from `consume_sign_in` — the raw cookie value to set + the resolved
/// identity for downstream logging.
pub struct SignInOutcome {
    pub raw_token: String,
    pub user_id: ObjectId,
    pub tenant_id: ObjectId,
}

// ── Errors ──

/// Errors surfaced from `SessionsService` to the transport layer.
///
/// Email-send failures inside `request_sign_in` are intentionally swallowed
/// (logged + return `Ok(())`) to preserve the always-200 enumeration defense.
/// They don't appear here.
#[derive(Debug)]
pub enum SessionError {
    InvalidEmail,
    RateLimited,
    InvalidToken,
    Internal(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEmail => write!(f, "Invalid email address"),
            Self::RateLimited => write!(f, "Too many requests. Try again later."),
            Self::InvalidToken => write!(f, "Sign-in link is invalid or has expired"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl SessionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidEmail => "invalid_email",
            Self::RateLimited => "rate_limited",
            Self::InvalidToken => "invalid_token",
            Self::Internal(_) => "db_error",
        }
    }
}

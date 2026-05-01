use mongodb::bson::{self, oid::ObjectId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Stored secret key (`rl_live_…`).
///
/// `scope` is optional only as a migration-window concession — pre-existing
/// rows in production deserialize as `None`. The auth middleware grandfathers
/// `None` to `KeyScope::Full` for one release cycle. Migration `m004` backfills
/// `Some(KeyScope::Full)` on all such rows; a follow-up PR will then make this
/// field required (non-Option) and flip middleware to reject `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKeyDoc {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub created_by: ObjectId,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: bson::DateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<KeyScope>,
}

/// Access level a secret key carries.
///
/// Tagged-enum serialization (`{ "type": "full" }` / `{ "type": "affiliate",
/// "affiliate_id": "…" }`) keeps the schema additive — future variants
/// (e.g. `ReadOnly`, `Webhook`) can be added without migrating old rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyScope {
    /// Full tenant access. The default for advertiser-issued keys —
    /// can mint links, manage affiliates, domains, webhooks, team, etc.
    Full,
    /// Partner-scoped access. Key can only operate on the named affiliate's
    /// links (mint pinned to this id, read its own links). Cannot manage
    /// tenant resources.
    Affiliate { affiliate_id: ObjectId },
}

// ── Errors ──

#[derive(Debug)]
pub enum SecretKeyError {
    UserNotMember,
    UserUnverified,
    KeyLimit,
    RequestPending,
    TooManyAttempts,
    InvalidCode,
    LastKey,
    SelfDelete,
    NotFound,
    EmailFailed(String),
    Internal(String),
}

impl fmt::Display for SecretKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserNotMember => write!(f, "Email is not a member of this team"),
            Self::UserUnverified => write!(f, "User has not verified their email"),
            Self::KeyLimit => write!(f, "Maximum of 5 secret keys per team"),
            Self::RequestPending => write!(
                f,
                "A key creation request is already pending. Check your email or wait 15 minutes."
            ),
            Self::TooManyAttempts => write!(f, "Too many attempts. Request a new code."),
            Self::InvalidCode => write!(f, "Invalid or expired confirmation code"),
            Self::LastKey => write!(f, "Cannot delete your only secret key"),
            Self::SelfDelete => {
                write!(
                    f,
                    "Cannot delete the key you are currently authenticated with"
                )
            }
            Self::NotFound => write!(f, "Secret key not found"),
            Self::EmailFailed(e) => write!(f, "Failed to send confirmation email: {e}"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl SecretKeyError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UserNotMember => "not_a_member",
            Self::UserUnverified => "user_unverified",
            Self::KeyLimit => "key_limit",
            Self::RequestPending => "request_pending",
            Self::TooManyAttempts => "too_many_attempts",
            Self::InvalidCode => "invalid_code",
            Self::LastKey => "last_key",
            Self::SelfDelete => "self_delete",
            Self::NotFound => "not_found",
            Self::EmailFailed(_) => "email_error",
            Self::Internal(_) => "db_error",
        }
    }
}

/// Caller's `KeyScope` is not authorized for this operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeError {
    Forbidden,
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forbidden => write!(f, "key scope forbids this operation"),
        }
    }
}

// ── Service return types ──

pub struct CreatedKey {
    pub id: ObjectId,
    pub key: String,
    pub key_prefix: String,
    pub created_at: bson::DateTime,
}

pub struct KeyDetail {
    pub id: ObjectId,
    pub key_prefix: String,
    pub created_by: ObjectId,
    pub created_at: bson::DateTime,
}

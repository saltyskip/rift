//! Data types for `services/auth/users/` — DB document, error enum, service
//! return types.

use mongodb::bson;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::core::public_id::{TenantId, UserId};
use crate::services::auth::permissions::AuthzError;
use crate::services::billing::quota::QuotaError;

// ── DB Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<UserId>,
    pub tenant_id: TenantId,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
    pub created_at: bson::DateTime,
    /// When the outstanding invite token expires, denormalized from the
    /// `tokens` collection so member status can be computed from the user row
    /// alone (no per-user token lookup on list). `None` for users who were
    /// never invited (verified owners) or invited before this field existed —
    /// the latter render as `Expired`, which a resend fixes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_expires_at: Option<bson::DateTime>,
}

/// Lifecycle of a team member, derived (not stored) from `verified` +
/// `invite_expires_at`. `verified` always wins — once accepted, expiry is moot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    Active,
    Pending,
    Expired,
}

impl MemberStatus {
    pub fn derive(
        verified: bool,
        invite_expires_at: Option<bson::DateTime>,
        now: bson::DateTime,
    ) -> Self {
        if verified {
            return Self::Active;
        }
        match invite_expires_at {
            Some(exp) if exp > now => Self::Pending,
            _ => Self::Expired,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
            Self::Expired => "expired",
        }
    }
}

// ── Errors ──

#[derive(Debug)]
pub enum UserError {
    InvalidEmail,
    UserExists,
    LastUser,
    NotFound,
    Forbidden(AuthzError),
    QuotaExceeded(QuotaError),
    EmailFailed(String),
    Internal(String),
}

impl From<QuotaError> for UserError {
    fn from(err: QuotaError) -> Self {
        UserError::QuotaExceeded(err)
    }
}

impl From<AuthzError> for UserError {
    fn from(err: AuthzError) -> Self {
        UserError::Forbidden(err)
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEmail => write!(f, "Invalid email address"),
            Self::UserExists => write!(f, "User already exists on this team"),
            Self::LastUser => write!(f, "Cannot remove the last verified user on this team"),
            Self::NotFound => write!(f, "User not found"),
            Self::Forbidden(e) => write!(f, "{e}"),
            Self::QuotaExceeded(e) => write!(f, "{e}"),
            Self::EmailFailed(e) => write!(f, "Failed to send email: {e}"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl UserError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidEmail => "invalid_email",
            Self::UserExists => "user_exists",
            Self::LastUser => "last_user",
            Self::NotFound => "not_found",
            Self::Forbidden(e) => e.code(),
            Self::QuotaExceeded(_) => "quota_exceeded",
            Self::EmailFailed(_) => "email_error",
            Self::Internal(_) => "db_error",
        }
    }
}

// ── Service return types ──

pub struct VerifyResult {
    pub tenant_id: TenantId,
    pub email: String,
}

pub struct InviteResult {
    pub user_id: UserId,
    pub email: String,
    /// True when this invite re-sent a link to an existing pending/expired
    /// member rather than creating a new one. Lets the transport tell the
    /// caller "resent" vs "sent".
    pub resent: bool,
}

pub struct UserDetail {
    pub id: UserId,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
    pub status: MemberStatus,
    pub created_at: bson::DateTime,
}

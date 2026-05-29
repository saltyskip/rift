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
}

pub struct UserDetail {
    pub id: UserId,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
    pub created_at: bson::DateTime,
}

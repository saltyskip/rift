//! Data types for `services/auth/users/` — DB document, error enum, service
//! return types.

use mongodb::bson::{self, oid::ObjectId};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::services::billing::quota::QuotaError;

// ── DB Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub tenant_id: ObjectId,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
    pub created_at: bson::DateTime,
}

// ── Errors ──

#[derive(Debug)]
pub enum UserError {
    InvalidEmail,
    EmailExists,
    UserExists,
    LastUser,
    NotFound,
    QuotaExceeded(QuotaError),
    EmailFailed(String),
    Internal(String),
}

impl From<QuotaError> for UserError {
    fn from(err: QuotaError) -> Self {
        UserError::QuotaExceeded(err)
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEmail => write!(f, "Invalid email address"),
            Self::EmailExists => write!(
                f,
                "Email already registered. Use key rotation to get a new key, or contact support."
            ),
            Self::UserExists => write!(f, "User already exists on this team"),
            Self::LastUser => write!(f, "Cannot remove the last verified user on this team"),
            Self::NotFound => write!(f, "User not found"),
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
            Self::EmailExists => "email_exists",
            Self::UserExists => "user_exists",
            Self::LastUser => "last_user",
            Self::NotFound => "not_found",
            Self::QuotaExceeded(_) => "quota_exceeded",
            Self::EmailFailed(_) => "email_error",
            Self::Internal(_) => "db_error",
        }
    }
}

// ── Service return types ──

pub struct SignupResult;

pub struct VerifyResult {
    pub tenant_id: ObjectId,
    pub email: String,
    /// Only set for owner verification — the full key shown once.
    pub key: Option<String>,
    pub key_prefix: Option<String>,
}

pub struct InviteResult {
    pub user_id: ObjectId,
    pub email: String,
}

pub struct UserDetail {
    pub id: ObjectId,
    pub email: String,
    pub verified: bool,
    pub is_owner: bool,
    pub created_at: bson::DateTime,
}

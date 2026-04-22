use mongodb::bson::oid::ObjectId;
use std::fmt;
use std::sync::Arc;

use super::repo::{SecretKeyCreateRequestDoc, SecretKeyDoc, SecretKeysRepository};
use crate::core::email;
use crate::services::auth::keys;
use crate::services::auth::users::repo::UsersRepository;

// ── Error ──

#[derive(Debug)]
pub enum SecretKeyError {
    UserNotMember,
    UserUnverified,
    KeyLimit,
    RequestPending,
    TooManyAttempts,
    NoPendingRequest,
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
            Self::NoPendingRequest => {
                write!(
                    f,
                    "No pending key creation request. Request a new code first."
                )
            }
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
            Self::NoPendingRequest => "no_pending_request",
            Self::InvalidCode => "invalid_code",
            Self::LastKey => "last_key",
            Self::SelfDelete => "self_delete",
            Self::NotFound => "not_found",
            Self::EmailFailed(_) => "email_error",
            Self::Internal(_) => "db_error",
        }
    }
}

// ── Response types ──

pub struct CreatedKey {
    pub id: ObjectId,
    pub key: String,
    pub key_prefix: String,
    pub created_at: mongodb::bson::DateTime,
}

pub struct KeyDetail {
    pub id: ObjectId,
    pub key_prefix: String,
    pub created_by: ObjectId,
    pub created_at: mongodb::bson::DateTime,
}

// ── Shared primitive: mint a new secret key for an existing tenant ──
//
// Used by the initial owner-key path (UsersService::verify) and the
// confirmation-code path (SecretKeysService::confirm_create). Billing paths
// in later phases can call this directly.
pub async fn mint_for_tenant(
    sk_repo: &dyn SecretKeysRepository,
    tenant_id: ObjectId,
    created_by: ObjectId,
) -> Result<CreatedKey, String> {
    let (full_key, key_hash, key_prefix) = keys::generate_api_key();
    let key_id = ObjectId::new();
    let now = mongodb::bson::DateTime::now();

    let key_doc = SecretKeyDoc {
        id: key_id,
        tenant_id,
        created_by,
        key_hash,
        key_prefix: key_prefix.clone(),
        created_at: now,
    };

    sk_repo.create_key(&key_doc).await?;

    Ok(CreatedKey {
        id: key_id,
        key: full_key,
        key_prefix,
        created_at: now,
    })
}

// ── Service ──

pub struct SecretKeysService {
    sk_repo: Arc<dyn SecretKeysRepository>,
    users_repo: Arc<dyn UsersRepository>,
}

impl SecretKeysService {
    pub fn new(
        sk_repo: Arc<dyn SecretKeysRepository>,
        users_repo: Arc<dyn UsersRepository>,
    ) -> Self {
        Self {
            sk_repo,
            users_repo,
        }
    }

    /// Request a new key creation. Sends a 6-char code to the specified email.
    pub async fn request_create(
        &self,
        tenant_id: ObjectId,
        email: &str,
        resend_api_key: &str,
        resend_from_email: &str,
    ) -> Result<(), SecretKeyError> {
        // Permission check
        let user = self
            .users_repo
            .find_by_tenant_and_email(&tenant_id, email)
            .await
            .map_err(SecretKeyError::Internal)?
            .ok_or(SecretKeyError::UserNotMember)?;

        if !user.verified {
            return Err(SecretKeyError::UserUnverified);
        }

        let user_id = user.id.unwrap_or_else(ObjectId::new);

        // Key limit
        let count = self
            .sk_repo
            .count_by_tenant(&tenant_id)
            .await
            .map_err(SecretKeyError::Internal)?;
        if count >= 5 {
            return Err(SecretKeyError::KeyLimit);
        }

        // Cooldown
        if self
            .sk_repo
            .find_pending_request(&tenant_id, &user_id)
            .await
            .map_err(SecretKeyError::Internal)?
            .is_some()
        {
            return Err(SecretKeyError::RequestPending);
        }

        // Generate code and store request
        let code = keys::generate_key_create_code();
        let token_hash = keys::hash_key(&code);
        let expires_at = mongodb::bson::DateTime::from_millis(
            chrono::Utc::now().timestamp_millis() + 15 * 60 * 1000,
        );

        let request_doc = SecretKeyCreateRequestDoc {
            id: None,
            tenant_id,
            user_id,
            token_hash,
            attempts: 0,
            expires_at,
            created_at: mongodb::bson::DateTime::now(),
        };

        self.sk_repo
            .create_request(&request_doc)
            .await
            .map_err(SecretKeyError::Internal)?;

        // Send email
        let html = format!(
            r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                <h2 style="margin-bottom: 24px;">Key creation confirmation</h2>
                <p>Use this code to confirm your new API key:</p>
                <code style="display: block; padding: 16px; background: #f4f4f5; border-radius: 6px; font-size: 24px; letter-spacing: 4px; text-align: center; margin: 20px 0;">{code}</code>
                <p style="color: #71717a; font-size: 13px; margin-top: 24px;">This code expires in 15 minutes. If you didn't request this, you can safely ignore this email.</p>
                <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 32px 0;" />
                <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
            </div>"#
        );

        email::send_email(
            resend_api_key,
            resend_from_email,
            email,
            &format!("Your Rift key creation code: {code}"),
            &html,
        )
        .await
        .map_err(SecretKeyError::EmailFailed)?;

        Ok(())
    }

    /// Confirm key creation with the 6-char code. Returns the new key (shown once).
    pub async fn confirm_create(
        &self,
        tenant_id: ObjectId,
        email: &str,
        token: &str,
    ) -> Result<CreatedKey, SecretKeyError> {
        let user = self
            .users_repo
            .find_by_tenant_and_email(&tenant_id, email)
            .await
            .map_err(SecretKeyError::Internal)?
            .ok_or(SecretKeyError::InvalidCode)?;

        let user_id = user.id.unwrap_or_else(ObjectId::new);

        // Rate limit
        let attempts = self
            .sk_repo
            .increment_request_attempts(&tenant_id, &user_id)
            .await
            .map_err(SecretKeyError::Internal)?;

        if attempts == 0 {
            return Err(SecretKeyError::NoPendingRequest);
        }
        if attempts > 5 {
            return Err(SecretKeyError::TooManyAttempts);
        }

        // Validate token
        let token_hash = keys::hash_key(&token.trim().to_uppercase());
        let consumed = self
            .sk_repo
            .validate_and_consume_request(&tenant_id, &user_id, &token_hash)
            .await
            .map_err(SecretKeyError::Internal)?;

        if !consumed {
            return Err(SecretKeyError::InvalidCode);
        }

        mint_for_tenant(&*self.sk_repo, tenant_id, user_id)
            .await
            .map_err(SecretKeyError::Internal)
    }

    /// List all secret keys for a tenant (prefix only).
    pub async fn list(&self, tenant_id: &ObjectId) -> Result<Vec<KeyDetail>, SecretKeyError> {
        let docs = self
            .sk_repo
            .list_by_tenant(tenant_id)
            .await
            .map_err(SecretKeyError::Internal)?;

        Ok(docs
            .into_iter()
            .map(|d| KeyDetail {
                id: d.id,
                key_prefix: d.key_prefix,
                created_by: d.created_by,
                created_at: d.created_at,
            })
            .collect())
    }

    /// Delete a secret key. Enforces guards: can't delete last key or self.
    pub async fn delete(
        &self,
        tenant_id: ObjectId,
        key_id: ObjectId,
        auth_key_id: ObjectId,
    ) -> Result<(), SecretKeyError> {
        if key_id == auth_key_id {
            return Err(SecretKeyError::SelfDelete);
        }

        let count = self
            .sk_repo
            .count_by_tenant(&tenant_id)
            .await
            .map_err(SecretKeyError::Internal)?;

        if count <= 1 {
            return Err(SecretKeyError::LastKey);
        }

        let deleted = self
            .sk_repo
            .delete_key(&tenant_id, &key_id)
            .await
            .map_err(SecretKeyError::Internal)?;

        if !deleted {
            return Err(SecretKeyError::NotFound);
        }

        Ok(())
    }
}

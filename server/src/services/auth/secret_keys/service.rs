use mongodb::bson::{doc, oid::ObjectId};
use std::fmt;
use std::sync::Arc;

use super::repo::{SecretKeyDoc, SecretKeysRepository};
use crate::core::email;
use crate::services::auth::keys;
use crate::services::auth::users::repo::UsersRepository;
use crate::services::tokens::{ConsumeOutcome, TokenKind, TokenPurpose, TokenService, TokenSpec};

// ── Error ──

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
    tokens: Arc<TokenService>,
}

impl SecretKeysService {
    /// 15min TTL matches the old `secret_key_create_requests` TTL exactly.
    const ROTATION_TTL_SECS: i64 = 15 * 60;

    /// 5 tries before the token is wiped and the user must re-request.
    /// Real security work — 36^6 = 2.2B code space is guessable at network
    /// speed without this cap.
    const ROTATION_MAX_ATTEMPTS: i32 = 5;

    pub fn new(
        sk_repo: Arc<dyn SecretKeysRepository>,
        users_repo: Arc<dyn UsersRepository>,
        tokens: Arc<TokenService>,
    ) -> Self {
        Self {
            sk_repo,
            users_repo,
            tokens,
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
        // Permission check: caller's email must be a verified member of this tenant.
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

        // Key limit.
        let count = self
            .sk_repo
            .count_by_tenant(&tenant_id)
            .await
            .map_err(SecretKeyError::Internal)?;
        if count >= 5 {
            return Err(SecretKeyError::KeyLimit);
        }

        // Cooldown: bail if a pending rotation is already live. TokenService
        // supersedes on re-issue, but we don't want repeat emails filling
        // the user's inbox.
        if self
            .tokens
            .pending_exists(TokenPurpose::KeyRotation, email)
            .await
            .map_err(SecretKeyError::Internal)?
        {
            return Err(SecretKeyError::RequestPending);
        }

        let code = self
            .tokens
            .issue(TokenSpec {
                purpose: TokenPurpose::KeyRotation,
                kind: TokenKind::TupleKeyed {
                    max_attempts: Self::ROTATION_MAX_ATTEMPTS,
                },
                ttl_secs: Self::ROTATION_TTL_SECS,
                email: email.to_string(),
                metadata: doc! {
                    "tenant_id": tenant_id,
                    "user_id": user_id,
                },
            })
            .await
            .map_err(SecretKeyError::Internal)?;

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
    ///
    /// `tenant_id` and `email` are only used for transport-layer auth sanity —
    /// the token metadata is the authoritative source of (tenant_id, user_id)
    /// because that's what was bound to the code when it was issued.
    pub async fn confirm_create(
        &self,
        tenant_id: ObjectId,
        email: &str,
        token: &str,
    ) -> Result<CreatedKey, SecretKeyError> {
        match self
            .tokens
            .consume_tuple(token, TokenPurpose::KeyRotation, email)
            .await
            .map_err(SecretKeyError::Internal)?
        {
            ConsumeOutcome::AttemptsExhausted => Err(SecretKeyError::TooManyAttempts),
            ConsumeOutcome::NotFound => Err(SecretKeyError::InvalidCode),
            ConsumeOutcome::Ok {
                purpose: TokenPurpose::KeyRotation,
                metadata,
                ..
            } => {
                let meta_tenant = metadata
                    .get_object_id("tenant_id")
                    .map_err(|e| SecretKeyError::Internal(format!("missing tenant_id: {e}")))?;
                let meta_user = metadata
                    .get_object_id("user_id")
                    .map_err(|e| SecretKeyError::Internal(format!("missing user_id: {e}")))?;

                // Belt-and-suspenders: the token is bound to a tenant; the
                // HTTP caller also claims a tenant via API key. They must
                // match, otherwise someone's crossing sessions.
                if meta_tenant != tenant_id {
                    return Err(SecretKeyError::InvalidCode);
                }

                mint_for_tenant(&*self.sk_repo, meta_tenant, meta_user)
                    .await
                    .map_err(SecretKeyError::Internal)
            }
            ConsumeOutcome::Ok { .. } => Err(SecretKeyError::InvalidCode),
        }
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

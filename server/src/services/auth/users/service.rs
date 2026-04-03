use mongodb::bson::oid::ObjectId;
use std::fmt;
use std::sync::Arc;

use super::repo::{UserDoc, UsersRepository};
use crate::core::email;
use crate::services::auth::keys;
use crate::services::auth::secret_keys::repo::{SecretKeyDoc, SecretKeysRepository};
use crate::services::auth::tenants::repo::{TenantDoc, TenantsRepository};

// ── Error ──

#[derive(Debug)]
pub enum UserError {
    InvalidEmail,
    EmailExists,
    UserExists,
    LastUser,
    NotFound,
    EmailFailed(String),
    Internal(String),
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
            Self::EmailFailed(_) => "email_error",
            Self::Internal(_) => "db_error",
        }
    }
}

// ── Response types ──

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
    pub created_at: mongodb::bson::DateTime,
}

// ── Service ──

pub struct UsersService {
    tenants_repo: Arc<dyn TenantsRepository>,
    users_repo: Arc<dyn UsersRepository>,
    sk_repo: Arc<dyn SecretKeysRepository>,
}

impl UsersService {
    pub fn new(
        tenants_repo: Arc<dyn TenantsRepository>,
        users_repo: Arc<dyn UsersRepository>,
        sk_repo: Arc<dyn SecretKeysRepository>,
    ) -> Self {
        Self {
            tenants_repo,
            users_repo,
            sk_repo,
        }
    }

    /// Sign up a new account: creates tenant + owner user, sends verify email.
    pub async fn signup(
        &self,
        email: &str,
        public_url: &str,
        resend_api_key: &str,
        resend_from_email: &str,
    ) -> Result<SignupResult, UserError> {
        let email = email.trim().to_lowercase();

        if !email.contains('@') || email.len() < 5 {
            return Err(UserError::InvalidEmail);
        }

        // Check if already exists
        if let Some(existing) = self
            .users_repo
            .find_by_email(&email)
            .await
            .map_err(UserError::Internal)?
        {
            if existing.verified {
                return Err(UserError::EmailExists);
            }
        }

        let verify_token = keys::generate_verify_token();
        let expires_at = mongodb::bson::DateTime::from_millis(
            chrono::Utc::now().timestamp_millis() + 24 * 60 * 60 * 1000,
        );

        let tenant_id = ObjectId::new();
        let tenant_doc = TenantDoc {
            id: Some(tenant_id),
            monthly_quota: 100,
            created_at: mongodb::bson::DateTime::now(),
        };

        self.tenants_repo
            .create(&tenant_doc)
            .await
            .map_err(UserError::Internal)?;

        let user_doc = UserDoc {
            id: Some(ObjectId::new()),
            tenant_id,
            email: email.clone(),
            verified: false,
            is_owner: true,
            verify_token: Some(verify_token.clone()),
            verify_token_expires_at: Some(expires_at),
            created_at: mongodb::bson::DateTime::now(),
        };

        self.users_repo
            .upsert_by_email(&user_doc)
            .await
            .map_err(UserError::Internal)?;

        let verify_url = format!("{public_url}/v1/auth/verify?token={verify_token}");
        let html = format!(
            r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                <h2 style="margin-bottom: 24px;">Verify your email</h2>
                <p>Click the button below to activate your Rift API key:</p>
                <a href="{verify_url}" style="display: inline-block; padding: 12px 24px; background: #0d9488; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0;">Verify Email</a>
                <p style="color: #71717a; font-size: 13px; margin-top: 24px;">Your API key will be shown once after verification. Save it — we can't show it again.</p>
                <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 32px 0;" />
                <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
            </div>"#
        );

        email::send_email(
            resend_api_key,
            resend_from_email,
            &email,
            "Verify your Rift API key",
            &html,
        )
        .await
        .map_err(UserError::EmailFailed)?;

        Ok(SignupResult)
    }

    /// Verify a user's email. For owners, generates the first key.
    pub async fn verify(&self, token: &str) -> Result<VerifyResult, UserError> {
        let user = self
            .users_repo
            .verify_user(token)
            .await
            .map_err(UserError::Internal)?
            .ok_or(UserError::NotFound)?;

        if user.is_owner {
            let (full_key, key_hash, key_prefix) = keys::generate_api_key();
            let user_id = user.id.unwrap_or_else(ObjectId::new);

            let key_doc = SecretKeyDoc {
                id: ObjectId::new(),
                tenant_id: user.tenant_id,
                created_by: user_id,
                key_hash,
                key_prefix: key_prefix.clone(),
                created_at: mongodb::bson::DateTime::now(),
            };

            self.sk_repo
                .create_key(&key_doc)
                .await
                .map_err(UserError::Internal)?;

            Ok(VerifyResult {
                tenant_id: user.tenant_id,
                email: user.email,
                key: Some(full_key),
                key_prefix: Some(key_prefix),
            })
        } else {
            Ok(VerifyResult {
                tenant_id: user.tenant_id,
                email: user.email,
                key: None,
                key_prefix: None,
            })
        }
    }

    /// Invite a user to a tenant. Sends verification email.
    pub async fn invite(
        &self,
        tenant_id: ObjectId,
        email: &str,
        public_url: &str,
        resend_api_key: &str,
        resend_from_email: &str,
    ) -> Result<InviteResult, UserError> {
        let email = email.trim().to_lowercase();

        if !email.contains('@') || email.len() < 5 {
            return Err(UserError::InvalidEmail);
        }

        if self
            .users_repo
            .find_by_tenant_and_email(&tenant_id, &email)
            .await
            .map_err(UserError::Internal)?
            .is_some()
        {
            return Err(UserError::UserExists);
        }

        let verify_token = keys::generate_verify_token();
        let expires_at = mongodb::bson::DateTime::from_millis(
            chrono::Utc::now().timestamp_millis() + 24 * 60 * 60 * 1000,
        );

        let user_id = ObjectId::new();
        let user_doc = UserDoc {
            id: Some(user_id),
            tenant_id,
            email: email.clone(),
            verified: false,
            is_owner: false,
            verify_token: Some(verify_token.clone()),
            verify_token_expires_at: Some(expires_at),
            created_at: mongodb::bson::DateTime::now(),
        };

        self.users_repo
            .create(&user_doc)
            .await
            .map_err(UserError::Internal)?;

        let verify_url = format!("{public_url}/v1/auth/verify?token={verify_token}");
        let html = format!(
            r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
                <h2 style="margin-bottom: 24px;">You've been invited</h2>
                <p>Click the button below to join the team on Rift:</p>
                <a href="{verify_url}" style="display: inline-block; padding: 12px 24px; background: #0d9488; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0;">Accept Invitation</a>
                <p style="color: #71717a; font-size: 13px; margin-top: 24px;">This link expires in 24 hours.</p>
                <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 32px 0;" />
                <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
            </div>"#
        );

        email::send_email(
            resend_api_key,
            resend_from_email,
            &email,
            "You've been invited to a Rift team",
            &html,
        )
        .await
        .map_err(UserError::EmailFailed)?;

        Ok(InviteResult { user_id, email })
    }

    /// List all users on a tenant.
    pub async fn list(&self, tenant_id: &ObjectId) -> Result<Vec<UserDetail>, UserError> {
        let docs = self
            .users_repo
            .list_by_tenant(tenant_id)
            .await
            .map_err(UserError::Internal)?;

        Ok(docs
            .into_iter()
            .map(|d| UserDetail {
                id: d.id.unwrap_or_else(ObjectId::new),
                email: d.email,
                verified: d.verified,
                is_owner: d.is_owner,
                created_at: d.created_at,
            })
            .collect())
    }

    /// Delete a user. Guard: can't remove last verified user.
    pub async fn delete(&self, tenant_id: ObjectId, user_id: ObjectId) -> Result<(), UserError> {
        let count = self
            .users_repo
            .count_verified_by_tenant(&tenant_id)
            .await
            .map_err(UserError::Internal)?;

        if count <= 1 {
            return Err(UserError::LastUser);
        }

        let deleted = self
            .users_repo
            .delete(&tenant_id, &user_id)
            .await
            .map_err(UserError::Internal)?;

        if !deleted {
            return Err(UserError::NotFound);
        }

        Ok(())
    }
}

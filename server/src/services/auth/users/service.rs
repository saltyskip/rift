use mongodb::bson::{doc, oid::ObjectId};
use std::sync::Arc;

use super::models::{InviteResult, SignupResult, UserDetail, UserDoc, UserError, VerifyResult};
use super::repo::UsersRepository;
use crate::core::email;
use crate::core::validation::validate_email;
use crate::services::auth::secret_keys::repo::SecretKeysRepository;
use crate::services::auth::secret_keys::service::mint_for_tenant;
use crate::services::auth::tenants::service::TenantsService;
use crate::services::billing::quota::{QuotaChecker, Resource};
use crate::services::tokens::{ConsumeOutcome, TokenKind, TokenPurpose, TokenService, TokenSpec};

// ── Service ──

crate::impl_container!(UsersService);
pub struct UsersService {
    tenants_service: Arc<TenantsService>,
    users_repo: Arc<dyn UsersRepository>,
    sk_repo: Arc<dyn SecretKeysRepository>,
    tokens: Arc<TokenService>,
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl UsersService {
    /// 24h TTL for email verification — users might not check their inbox
    /// same-day. After expiry they re-request.
    const EMAIL_VERIFY_TTL_SECS: i64 = 24 * 60 * 60;

    pub fn new(
        tenants_service: Arc<TenantsService>,
        users_repo: Arc<dyn UsersRepository>,
        sk_repo: Arc<dyn SecretKeysRepository>,
        tokens: Arc<TokenService>,
        quota: Option<Arc<dyn QuotaChecker>>,
    ) -> Self {
        Self {
            tenants_service,
            users_repo,
            sk_repo,
            tokens,
            quota,
        }
    }

    /// Create an unverified owner user on a tenant and return the raw verify
    /// token. Idempotent on the user row (upsert by email) — a retry from an
    /// unverified state supersedes the previous token via TokenService.
    pub async fn attach_email_owner(
        &self,
        tenant_id: ObjectId,
        email: &str,
    ) -> Result<String, UserError> {
        let user_doc = UserDoc {
            id: Some(ObjectId::new()),
            tenant_id,
            email: email.to_string(),
            verified: false,
            is_owner: true,
            created_at: mongodb::bson::DateTime::now(),
        };

        self.users_repo
            .upsert_by_email(&user_doc)
            .await
            .map_err(UserError::Internal)?;

        self.tokens
            .issue(TokenSpec {
                purpose: TokenPurpose::EmailVerify,
                kind: TokenKind::HashKeyed,
                ttl_secs: Self::EMAIL_VERIFY_TTL_SECS,
                email: email.to_string(),
                metadata: doc! {},
            })
            .await
            .map_err(UserError::Internal)
    }

    /// Create a tenant with an already-verified email owner. Used by the
    /// billing webhook path when a new customer completes Stripe Checkout —
    /// payment success is itself proof-of-email, so no verification round
    /// trip is needed.
    ///
    /// Returns `(tenant_id, user_id)`. Bubbles up a db error if the email is
    /// already present; callers should only reach this path after a
    /// `TenantsRepo::find_by_owner_email` miss.
    pub async fn create_tenant_with_verified_owner(
        &self,
        email: &str,
    ) -> Result<(ObjectId, ObjectId), UserError> {
        let email = validate_email(email).map_err(|_| UserError::InvalidEmail)?;

        let tenant_id = self
            .tenants_service
            .create_blank()
            .await
            .map_err(UserError::Internal)?;

        let user_id = ObjectId::new();
        let user_doc = UserDoc {
            id: Some(user_id),
            tenant_id,
            email: email.clone(),
            verified: true,
            is_owner: true,
            created_at: mongodb::bson::DateTime::now(),
        };

        self.users_repo
            .create(&user_doc)
            .await
            .map_err(UserError::Internal)?;

        Ok((tenant_id, user_id))
    }

    /// Sign up a new account: creates tenant + owner user, sends verify email.
    pub async fn signup(
        &self,
        email: &str,
        public_url: &str,
        resend_api_key: &str,
        resend_from_email: &str,
    ) -> Result<SignupResult, UserError> {
        let email = validate_email(email).map_err(|_| UserError::InvalidEmail)?;

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

        let tenant_id = self
            .tenants_service
            .create_blank()
            .await
            .map_err(UserError::Internal)?;

        let verify_token = self.attach_email_owner(tenant_id, &email).await?;

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
        let outcome = self
            .tokens
            .consume_hash(token)
            .await
            .map_err(UserError::Internal)?;

        let email = match outcome {
            ConsumeOutcome::Ok {
                purpose: TokenPurpose::EmailVerify,
                email,
                ..
            } => email,
            // Token valid but for a different flow — treat as not found so
            // we don't leak purpose info.
            ConsumeOutcome::Ok { .. } => return Err(UserError::NotFound),
            ConsumeOutcome::NotFound | ConsumeOutcome::AttemptsExhausted => {
                return Err(UserError::NotFound)
            }
        };

        let user = self
            .users_repo
            .mark_verified(&email)
            .await
            .map_err(UserError::Internal)?
            .ok_or(UserError::NotFound)?;

        if user.is_owner {
            let user_id = user.id.unwrap_or_else(ObjectId::new);
            let created = mint_for_tenant(&*self.sk_repo, user.tenant_id, user_id)
                .await
                .map_err(UserError::Internal)?;

            Ok(VerifyResult {
                tenant_id: user.tenant_id,
                email: user.email,
                key: Some(created.key),
                key_prefix: Some(created.key_prefix),
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
        let email = validate_email(email).map_err(|_| UserError::InvalidEmail)?;

        if self
            .users_repo
            .find_by_tenant_and_email(&tenant_id, &email)
            .await
            .map_err(UserError::Internal)?
            .is_some()
        {
            return Err(UserError::UserExists);
        }

        // Service-layer quota enforcement (applies to every transport).
        if let Some(q) = &self.quota {
            q.check(&tenant_id, Resource::InviteTeamMember).await?;
        }

        let user_id = ObjectId::new();
        let user_doc = UserDoc {
            id: Some(user_id),
            tenant_id,
            email: email.clone(),
            verified: false,
            is_owner: false,
            created_at: mongodb::bson::DateTime::now(),
        };

        self.users_repo
            .create(&user_doc)
            .await
            .map_err(UserError::Internal)?;

        let verify_token = self
            .tokens
            .issue(TokenSpec {
                purpose: TokenPurpose::EmailVerify,
                kind: TokenKind::HashKeyed,
                ttl_secs: Self::EMAIL_VERIFY_TTL_SECS,
                email: email.clone(),
                metadata: doc! {},
            })
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

use mongodb::bson::{doc, oid::ObjectId};
use rift_macros::requires;
use std::sync::Arc;

use super::models::{InviteResult, UserDetail, UserDoc, UserError, VerifyResult};
use super::repo::UsersRepository;
use crate::core::email;
use crate::core::validation::validate_email;
use crate::services::auth::permissions::{AuthContext, Permission};
use crate::services::auth::tenants::service::TenantsService;
use crate::services::billing::quota::{QuotaChecker, Resource};
use crate::services::tokens::{ConsumeOutcome, TokenKind, TokenPurpose, TokenService, TokenSpec};

// ── Service ──

crate::impl_container!(UsersService);
pub struct UsersService {
    tenants_service: Arc<TenantsService>,
    users_repo: Arc<dyn UsersRepository>,
    tokens: Arc<TokenService>,
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl UsersService {
    /// 24h TTL for team invite emails — invited members might not check their
    /// inbox same-day. After expiry they re-request.
    const INVITE_EMAIL_TTL_SECS: i64 = 24 * 60 * 60;

    pub fn new(
        tenants_service: Arc<TenantsService>,
        users_repo: Arc<dyn UsersRepository>,
        tokens: Arc<TokenService>,
        quota: Option<Arc<dyn QuotaChecker>>,
    ) -> Self {
        Self {
            tenants_service,
            users_repo,
            tokens,
            quota,
        }
    }

    /// Create a tenant with an already-verified email owner. Used by the
    /// billing webhook path when a new customer completes Stripe Checkout
    /// (payment success is itself proof-of-email) and by the magic-link
    /// signin flow on first-ever signin for an email.
    ///
    /// Returns `(tenant_id, user_id)`. Bubbles up a db error if the email is
    /// already present; callers should only reach this path after a
    /// `users_repo.find_by_email` miss.
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

        let user_id = crate::core::public_id::UserId::new();
        let user_doc = UserDoc {
            id: Some(user_id),
            tenant_id: crate::core::public_id::TenantId::from_object_id(tenant_id),
            email: email.clone(),
            verified: true,
            is_owner: true,
            created_at: mongodb::bson::DateTime::now(),
        };

        self.users_repo
            .create(&user_doc)
            .await
            .map_err(UserError::Internal)?;

        Ok((tenant_id, user_id.to_object_id()))
    }

    /// Accept a team-invite verification token: mark the invited user as
    /// verified and return their identity. Owner-signup tokens used to flow
    /// through here too; that path is gone (replaced by /v1/auth/signin),
    /// but the endpoint stays alive for in-flight team-invite emails.
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
            // Token valid but for a different flow — refuse so signin /
            // billing tokens can't be redeemed here.
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

        Ok(VerifyResult {
            tenant_id: user.tenant_id,
            email: user.email,
        })
    }

    /// Invite a user to a tenant. Sends verification email.
    #[requires(Permission::TenantAdmin)]
    pub async fn invite(
        &self,
        ctx: &AuthContext,
        email: &str,
        public_url: &str,
        resend_api_key: &str,
        resend_from_email: &str,
    ) -> Result<InviteResult, UserError> {
        let email = validate_email(email).map_err(|_| UserError::InvalidEmail)?;

        if self
            .users_repo
            .find_by_tenant_and_email(ctx.tenant_id.as_object_id(), &email)
            .await
            .map_err(UserError::Internal)?
            .is_some()
        {
            return Err(UserError::UserExists);
        }

        // Service-layer quota enforcement (applies to every transport).
        if let Some(q) = &self.quota {
            q.check(ctx.tenant_id.as_object_id(), Resource::InviteTeamMember)
                .await?;
        }

        let user_id = crate::core::public_id::UserId::new();
        let user_doc = UserDoc {
            id: Some(user_id),
            tenant_id: ctx.tenant_id,
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
                ttl_secs: Self::INVITE_EMAIL_TTL_SECS,
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
    #[requires(Permission::TenantAdmin)]
    pub async fn list(&self, ctx: &AuthContext) -> Result<Vec<UserDetail>, UserError> {
        let docs = self
            .users_repo
            .list_by_tenant(ctx.tenant_id.as_object_id())
            .await
            .map_err(UserError::Internal)?;

        Ok(docs
            .into_iter()
            .map(|d| UserDetail {
                id: d.id.unwrap_or_else(crate::core::public_id::UserId::new),
                email: d.email,
                verified: d.verified,
                is_owner: d.is_owner,
                created_at: d.created_at,
            })
            .collect())
    }

    /// Delete a user. Guard: can't remove last verified user.
    #[requires(Permission::TenantAdmin)]
    pub async fn delete(&self, ctx: &AuthContext, user_id: ObjectId) -> Result<(), UserError> {
        let count = self
            .users_repo
            .count_verified_by_tenant(ctx.tenant_id.as_object_id())
            .await
            .map_err(UserError::Internal)?;

        if count <= 1 {
            return Err(UserError::LastUser);
        }

        let deleted = self
            .users_repo
            .delete(ctx.tenant_id.as_object_id(), &user_id)
            .await
            .map_err(UserError::Internal)?;

        if !deleted {
            return Err(UserError::NotFound);
        }

        Ok(())
    }
}

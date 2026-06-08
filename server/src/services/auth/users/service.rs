use mongodb::bson::doc;
use rift_macros::requires;
use std::sync::Arc;

use super::models::{InviteResult, MemberStatus, UserDetail, UserDoc, UserError, VerifyResult};
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
    ) -> Result<
        (
            crate::core::public_id::TenantId,
            crate::core::public_id::UserId,
        ),
        UserError,
    > {
        let email = validate_email(email).map_err(|_| UserError::InvalidEmail)?;

        let tenant_id = self
            .tenants_service
            .create_blank()
            .await
            .map_err(UserError::Internal)?;

        let user_id = crate::core::public_id::UserId::new();
        let user_doc = UserDoc {
            id: Some(user_id),
            tenant_id,
            email: email.clone(),
            verified: true,
            is_owner: true,
            created_at: mongodb::bson::DateTime::now(),
            invite_expires_at: None,
        };

        self.users_repo
            .create(&user_doc)
            .await
            .map_err(UserError::Internal)?;

        Ok((tenant_id, user_id))
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

    /// Invite a user to a tenant, or re-send the invite if they're already a
    /// pending/expired member. Sends the verification email either way.
    ///
    /// A member whose 24h invite link has lapsed is stranded otherwise: the
    /// link is dead but a fresh `invite` used to 409 with `user_exists`. We
    /// treat any *unverified* row as a resend target (rotating the token), and
    /// only reject when the email belongs to an *already-verified* member.
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

        let existing = self
            .users_repo
            .find_by_tenant_and_email(&ctx.tenant_id, &email)
            .await
            .map_err(UserError::Internal)?;

        let (user_id, resent) = match classify_invite(existing) {
            InviteAction::AlreadyMember => return Err(UserError::UserExists),
            InviteAction::Resend(user_id) => {
                // No quota check: the member row already exists, so a resend
                // doesn't consume a new seat. Supersede the stale link first.
                self.tokens
                    .revoke_pending(TokenPurpose::EmailVerify, &email)
                    .await
                    .map_err(UserError::Internal)?;
                self.users_repo
                    .set_invite_expiry(&ctx.tenant_id, &email, invite_expiry_from_now())
                    .await
                    .map_err(UserError::Internal)?;
                (user_id, true)
            }
            InviteAction::CreateNew => {
                // Service-layer quota enforcement (applies to every transport).
                if let Some(q) = &self.quota {
                    q.check(&ctx.tenant_id, Resource::InviteTeamMember).await?;
                }
                let user_id = crate::core::public_id::UserId::new();
                let user_doc = UserDoc {
                    id: Some(user_id),
                    tenant_id: ctx.tenant_id,
                    email: email.clone(),
                    verified: false,
                    is_owner: false,
                    created_at: mongodb::bson::DateTime::now(),
                    invite_expires_at: Some(invite_expiry_from_now()),
                };
                self.users_repo
                    .create(&user_doc)
                    .await
                    .map_err(UserError::Internal)?;
                (user_id, false)
            }
        };

        self.issue_and_send_invite(&email, public_url, resend_api_key, resend_from_email)
            .await?;

        Ok(InviteResult {
            user_id,
            email,
            resent,
        })
    }

    /// List all users on a tenant.
    #[requires(Permission::TenantAdmin)]
    pub async fn list(&self, ctx: &AuthContext) -> Result<Vec<UserDetail>, UserError> {
        let docs = self
            .users_repo
            .list_by_tenant(&ctx.tenant_id)
            .await
            .map_err(UserError::Internal)?;

        let now = mongodb::bson::DateTime::now();
        Ok(docs
            .into_iter()
            .map(|d| UserDetail {
                id: d.id.unwrap_or_else(crate::core::public_id::UserId::new),
                status: MemberStatus::derive(d.verified, d.invite_expires_at, now),
                email: d.email,
                verified: d.verified,
                is_owner: d.is_owner,
                created_at: d.created_at,
            })
            .collect())
    }

    /// Delete a user. Guard: can't remove last verified user.
    #[requires(Permission::TenantAdmin)]
    pub async fn delete(
        &self,
        ctx: &AuthContext,
        user_id: crate::core::public_id::UserId,
    ) -> Result<(), UserError> {
        let count = self
            .users_repo
            .count_verified_by_tenant(&ctx.tenant_id)
            .await
            .map_err(UserError::Internal)?;

        if count <= 1 {
            return Err(UserError::LastUser);
        }

        let deleted = self
            .users_repo
            .delete(&ctx.tenant_id, &user_id)
            .await
            .map_err(UserError::Internal)?;

        if !deleted {
            return Err(UserError::NotFound);
        }

        Ok(())
    }

    // ── Helpers ──

    /// Mint a fresh single-use invite token and email the accept link. Shared
    /// by the create-new and resend branches of `invite`.
    async fn issue_and_send_invite(
        &self,
        email: &str,
        public_url: &str,
        resend_api_key: &str,
        resend_from_email: &str,
    ) -> Result<(), UserError> {
        let verify_token = self
            .tokens
            .issue(TokenSpec {
                purpose: TokenPurpose::EmailVerify,
                kind: TokenKind::HashKeyed,
                ttl_secs: Self::INVITE_EMAIL_TTL_SECS,
                email: email.to_string(),
                metadata: doc! {},
            })
            .await
            .map_err(UserError::Internal)?;

        let verify_url = format!("{public_url}/v1/auth/verify?token={verify_token}");
        let html = email::branded_html(&format!(
            "{}<p>Click the button below to join the team on Rift:</p>{}{}",
            email::heading("You've been invited"),
            email::cta_button("Accept Invitation", &verify_url),
            email::fine_print("This link expires in 24 hours."),
        ));

        email::send_email(
            resend_api_key,
            resend_from_email,
            email,
            "You've been invited to a Rift team",
            &html,
        )
        .await
        .map_err(UserError::EmailFailed)
    }
}

/// What `invite` should do given the existing (or absent) row for the email.
#[derive(Debug, PartialEq, Eq)]
enum InviteAction {
    /// No row — create a fresh pending member (consumes quota).
    CreateNew,
    /// Unverified row (pending or expired) — rotate the link, no new seat.
    Resend(crate::core::public_id::UserId),
    /// Verified member — reject with `UserExists`.
    AlreadyMember,
}

fn classify_invite(existing: Option<UserDoc>) -> InviteAction {
    match existing {
        None => InviteAction::CreateNew,
        Some(u) if u.verified => InviteAction::AlreadyMember,
        Some(u) => InviteAction::Resend(u.id.unwrap_or_else(crate::core::public_id::UserId::new)),
    }
}

fn invite_expiry_from_now() -> mongodb::bson::DateTime {
    let now_ms = mongodb::bson::DateTime::now().timestamp_millis();
    mongodb::bson::DateTime::from_millis(now_ms + UsersService::INVITE_EMAIL_TTL_SECS * 1000)
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

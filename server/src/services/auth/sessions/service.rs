//! SessionsService — magic-link signin → durable browser session lifecycle.
//!
//! Mirrors `services/billing/handoff.rs` for the request/redeem shape.
//! Unlike billing, the redeem here:
//!   1. Creates the User + Tenant on first signin (auto-signup),
//!   2. Mints a long-lived opaque session token (30d) stored in the `sessions`
//!      collection — returned as the `Set-Cookie` value by the callback route.
//!
//! Throughout, `email` flows through `validate_email` and is lowercased. The
//! signin token (15 min) carries the email in its metadata, so the redeem
//! path is stateless and doesn't need a separate "pending signin" row.

use std::sync::Arc;

use mongodb::bson::doc;
use rand::Rng;
use sha2::{Digest, Sha256};

use super::models::{ResolvedSession, SessionDoc, SessionError, SessionsConfig, SignInOutcome};
use super::repo::SessionsRepository;
use crate::core::email;
use crate::core::rate_limit::RateLimiter;
use crate::core::validation::validate_email;
use crate::services::auth::users::repo::UsersRepository;
use crate::services::auth::users::service::UsersService;
use crate::services::tokens::{ConsumeOutcome, TokenKind, TokenPurpose, TokenService, TokenSpec};

crate::impl_container!(SessionsService);
pub struct SessionsService {
    tokens: Arc<TokenService>,
    sessions_repo: Arc<dyn SessionsRepository>,
    users_repo: Arc<dyn UsersRepository>,
    users_service: Arc<UsersService>,
    config: SessionsConfig,
    ip_limiter: RateLimiter,
}

impl SessionsService {
    /// 15min — short because the link lives in inboxes / access logs.
    const SIGNIN_TOKEN_TTL_SECS: i64 = 15 * 60;

    /// 30 days — typical "remember me" default. Sessions are cheap to revoke;
    /// the bigger the surface, the more annoying re-auth becomes.
    ///
    /// `pub` so cookie-issuing route handlers (`api/auth/sessions/routes.rs`,
    /// `api/auth/oauth/routes.rs`) can set `Max-Age` consistent with the
    /// server-side session row's `expires_at`. Drift here silently produces
    /// cookies that expire before the row, or vice versa.
    pub const SESSION_TTL_SECS: i64 = 30 * 24 * 60 * 60;

    /// Per-email cap: 5 signin emails per hour. Stops an attacker from
    /// spamming an inbox by hitting `/v1/auth/signin` from rotating IPs.
    const PER_EMAIL_WINDOW_SECS: i64 = 3600;
    const PER_EMAIL_MAX: u64 = 5;

    /// Only write `last_seen_at` when stale by more than 60s — sessions are
    /// read on every authed request, but we don't want to write that often.
    const TOUCH_INTERVAL_SECS: i64 = 60;

    pub fn new(
        tokens: Arc<TokenService>,
        sessions_repo: Arc<dyn SessionsRepository>,
        users_repo: Arc<dyn UsersRepository>,
        users_service: Arc<UsersService>,
        config: SessionsConfig,
    ) -> Self {
        // Per-IP: 10 req/min burst 10. Looser than billing because legitimate
        // users may retry across tabs; per-email cap is the real ceiling.
        let ip_limiter = RateLimiter::new(10, 10);
        Self {
            tokens,
            sessions_repo,
            users_repo,
            users_service,
            config,
            ip_limiter,
        }
    }

    // ── Public API ──

    /// Issue a signin magic link to `email`. Auto-signup happens on redeem,
    /// not here — this is a pure "emit token + send email" operation.
    ///
    /// `origin` is the request's `Origin` header value, **already validated**
    /// by the route handler against the `OriginMatcher`. When provided, it
    /// gets stored in the token's metadata so the callback can redirect the
    /// user back to wherever they started — letting Vercel preview URLs
    /// (which change per branch / per commit) work without env-var updates.
    /// Callers MUST validate before passing — the service trusts what it
    /// gets here.
    ///
    /// Always returns `Ok` on validation success to prevent email enumeration;
    /// downstream failures (send error, per-email cap hit) are logged and
    /// swallowed. The only error surfaces are invalid-email and IP rate-limit.
    pub async fn request_sign_in(
        &self,
        email_raw: &str,
        client_ip: &str,
        origin: Option<&str>,
        next: Option<&str>,
    ) -> Result<(), SessionError> {
        if !self.ip_limiter.check(client_ip) {
            return Err(SessionError::RateLimited);
        }

        let email = validate_email(email_raw).map_err(|_| SessionError::InvalidEmail)?;

        // Per-email cap. Silently skip sending when over cap — the 200
        // response prevents callers from distinguishing "you hit the cap"
        // from "we sent it." Combined with InvalidEmail being the only
        // negative validation signal, this prevents enumeration.
        let recent = self
            .tokens
            .count_recent(TokenPurpose::Signin, &email, Self::PER_EMAIL_WINDOW_SECS)
            .await
            .unwrap_or(0);
        if recent >= Self::PER_EMAIL_MAX {
            tracing::info!(email = %email, "signin_email_rate_limited");
            return Ok(());
        }

        let mut metadata = doc! {};
        if let Some(o) = origin {
            metadata.insert("origin", o);
        }
        if let Some(n) = next {
            metadata.insert("next", n);
        }

        let raw_token = match self
            .tokens
            .issue(TokenSpec {
                purpose: TokenPurpose::Signin,
                kind: TokenKind::HashKeyed,
                ttl_secs: Self::SIGNIN_TOKEN_TTL_SECS,
                email: email.clone(),
                metadata,
            })
            .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "signin_token_issue_failed");
                return Ok(());
            }
        };

        let link_url = format!(
            "{}/v1/auth/callback?token={}",
            self.config.public_url, raw_token
        );

        let html = signin_email_html(&link_url);
        if let Err(e) = email::send_email(
            &self.config.resend_api_key,
            &self.config.resend_from_email,
            &email,
            "Sign in to Rift",
            &html,
        )
        .await
        {
            tracing::error!(error = %e, email = %email, "signin_email_send_failed");
        }

        Ok(())
    }

    /// Consume a signin token: resolve / create the user, mint a fresh
    /// session, and return the raw cookie value plus the resolved identity.
    /// Called from `GET /v1/auth/callback?token=` once.
    ///
    /// `origin` (when present in the token's metadata) is returned via
    /// `SignInOutcome::origin` so the callback can redirect the browser back
    /// to the same origin that started the flow. Callers MUST re-validate
    /// the origin against the current allowlist before using it — what was
    /// allowed at signin time may not be allowed at callback time if env
    /// vars changed.
    pub async fn consume_sign_in(
        &self,
        raw_token: &str,
        client_ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<SignInOutcome, SessionError> {
        let outcome = self
            .tokens
            .consume_hash(raw_token)
            .await
            .map_err(SessionError::Internal)?;

        let (email, origin, next) = match outcome {
            ConsumeOutcome::Ok {
                purpose: TokenPurpose::Signin,
                email,
                metadata,
            } => {
                let origin = metadata.get_str("origin").ok().map(|s| s.to_string());
                let next = metadata.get_str("next").ok().map(|s| s.to_string());
                (email, origin, next)
            }
            // Token valid but for a different flow — refuse so we don't let
            // billing magic-links or key-rotation codes mint sessions.
            ConsumeOutcome::Ok { .. } => return Err(SessionError::InvalidToken),
            ConsumeOutcome::NotFound | ConsumeOutcome::AttemptsExhausted => {
                return Err(SessionError::InvalidToken);
            }
        };

        // Resolve or create the user. The find_by_email lookup is a single
        // index hit; create_tenant_with_verified_owner is only hit on the
        // first signin per email.
        let (user_id, tenant_id) = match self
            .users_repo
            .find_by_email(&email)
            .await
            .map_err(SessionError::Internal)?
        {
            Some(user) => {
                let user_id = user.id.unwrap_or_else(crate::core::public_id::UserId::new);
                // Email click is proof of ownership — bump verified if it
                // wasn't already. We don't surface a failure if mark_verified
                // returns None here because the find_by_email above just
                // succeeded; the only way it'd miss is a race, which is fine.
                if !user.verified {
                    let _ = self.users_repo.mark_verified(&email).await;
                }
                (user_id, user.tenant_id)
            }
            None => self
                .users_service
                .create_tenant_with_verified_owner(&email)
                .await
                .map(|(tenant_id, user_id)| {
                    (
                        crate::core::public_id::UserId::from_object_id(user_id),
                        crate::core::public_id::TenantId::from_object_id(tenant_id),
                    )
                })
                .map_err(|e| SessionError::Internal(e.to_string()))?,
        };

        let raw_session = self
            .issue_session(user_id, tenant_id, client_ip, user_agent)
            .await?;

        // Surface `email` only for logging — callers don't need it beyond
        // sentry/tracing context, which we set at the route handler layer.
        tracing::info!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            email = %email,
            "sign_in_consumed"
        );

        Ok(SignInOutcome {
            raw_token: raw_session,
            user_id,
            tenant_id,
            origin,
            next,
        })
    }

    /// Mint a fresh session for an already-resolved `(user_id, tenant_id)` and
    /// return the raw opaque cookie value. Shared between the magic-link
    /// `consume_sign_in` path and the OAuth-federation callback path — both
    /// arrive at "we have a verified user, mint a session" through different
    /// upstreams but produce the same session row + cookie shape.
    pub async fn issue_session(
        &self,
        user_id: crate::core::public_id::UserId,
        tenant_id: crate::core::public_id::TenantId,
        client_ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<String, SessionError> {
        let raw_session = generate_session_token();
        let token_hash = hash_session_token(&raw_session);
        let now = mongodb::bson::DateTime::now();
        let expires_at = mongodb::bson::DateTime::from_millis(
            now.timestamp_millis() + Self::SESSION_TTL_SECS * 1000,
        );

        let session_doc = SessionDoc {
            id: crate::core::public_id::AuthSessionId::new(),
            user_id,
            tenant_id,
            token_hash,
            created_at: now,
            expires_at,
            last_seen_at: now,
            revoked_at: None,
            user_agent: user_agent.map(|s| truncate(s, 256)),
            ip: client_ip.map(|s| s.to_string()),
        };

        self.sessions_repo
            .insert(&session_doc)
            .await
            .map_err(SessionError::Internal)?;

        Ok(raw_session)
    }

    /// Resolve a raw session token to an identity for middleware. Returns
    /// `Ok(None)` if the session is missing, expired, or revoked. Also
    /// debounces a `last_seen_at` write.
    pub async fn lookup(&self, raw_token: &str) -> Result<Option<ResolvedSession>, SessionError> {
        let token_hash = hash_session_token(raw_token);
        let session = match self
            .sessions_repo
            .find_active_by_hash(&token_hash)
            .await
            .map_err(SessionError::Internal)?
        {
            Some(s) => s,
            None => return Ok(None),
        };

        // Debounced touch — fire-and-forget. A failure here doesn't fail the
        // request; worst case `last_seen_at` lags.
        let staleness_secs = mongodb::bson::DateTime::now().timestamp_millis() / 1000
            - session.last_seen_at.timestamp_millis() / 1000;
        if staleness_secs > Self::TOUCH_INTERVAL_SECS {
            let _ = self
                .sessions_repo
                .touch_last_seen(&session.id.to_object_id())
                .await;
        }

        Ok(Some(ResolvedSession {
            session_id: session.id,
            user_id: session.user_id,
            tenant_id: session.tenant_id,
        }))
    }

    /// Revoke a session by id (called from `POST /v1/auth/signout`). Idempotent.
    pub async fn revoke(
        &self,
        session_id: &crate::core::public_id::AuthSessionId,
    ) -> Result<(), SessionError> {
        self.sessions_repo
            .revoke(&session_id.to_object_id())
            .await
            .map(|_| ())
            .map_err(SessionError::Internal)
    }
}

// ── Helpers ──

/// 32 random bytes hex-encoded = 64-char opaque token. Matches the existing
/// verify-token shape and gives us 256 bits of entropy, far above the brute
/// force ceiling on any reasonable DB.
fn generate_session_token() -> String {
    let bytes: Vec<u8> = rand::rng().random_iter::<u8>().take(32).collect();
    hex::encode(&bytes)
}

fn hash_session_token(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

fn signin_email_html(link_url: &str) -> String {
    format!(
        r#"<div style="font-family: system-ui, sans-serif; max-width: 480px; margin: 0 auto; padding: 40px 20px;">
            <h2 style="margin-bottom: 24px;">Sign in to Rift</h2>
            <p>Click the button below to sign in:</p>
            <a href="{link_url}" style="display: inline-block; padding: 12px 24px; background: #0d9488; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0;">Sign in to Rift</a>
            <p style="color: #71717a; font-size: 13px; margin-top: 24px;">This link expires in 15 minutes and can only be used once.</p>
            <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 32px 0;" />
            <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
        </div>"#
    )
}

//! OauthService — orchestrates the GitHub / Google federation handshake.
//!
//! Two operations:
//!   1. `start` — issue a state token (carries `provider`, PKCE
//!      `code_verifier`, sanitized `next`, validated `origin`) and return the
//!      provider authorize URL.
//!   2. `consume_callback` — validate state, exchange code with the provider,
//!      apply verified-email rules, resolve-or-create the user. Returns the
//!      `(user_id, tenant_id, next, origin)` triple the route handler needs
//!      to mint a session via `SessionsService::issue_session`.
//!
//! Email is the identity — GitHub / Google / magic-link all hit
//! `UsersRepo::find_by_email` and converge to the same `User` row.

use std::sync::Arc;
use std::time::Duration;

use mongodb::bson::doc;
use rand::Rng;
use reqwest::Client;
use sha2::{Digest, Sha256};

use super::models::{
    OauthCallbackOutcome, OauthConfig, OauthError, OauthProvider, OauthStartOutcome,
};
use super::providers::{GithubClient, GoogleClient, OauthProviderClient};
use crate::core::rate_limit::RateLimiter;
use crate::services::auth::users::repo::UsersRepository;
use crate::services::auth::users::service::UsersService;
use crate::services::tokens::{ConsumeOutcome, TokenKind, TokenPurpose, TokenService, TokenSpec};

crate::impl_container!(OauthService);
pub struct OauthService {
    tokens: Arc<TokenService>,
    users_repo: Arc<dyn UsersRepository>,
    users_service: Arc<UsersService>,
    github: Option<Box<dyn OauthProviderClient>>,
    google: Option<Box<dyn OauthProviderClient>>,
    api_base_url: String,
    http: Client,
    /// Per-IP rate limit on `/start`. OAuth state issuance is unauthenticated
    /// and amplifies into outbound HTTPS calls to providers — without this
    /// gate, an attacker could spam state-token rows and provider RPS.
    ip_limiter: RateLimiter,
}

impl OauthService {
    /// 5 min — long enough for users to approve at GitHub/Google, short
    /// enough that a stolen state token is mostly useless. Single-use via
    /// `consume_hash` semantics.
    const STATE_TTL_SECS: i64 = 5 * 60;

    pub fn new(
        config: OauthConfig,
        tokens: Arc<TokenService>,
        users_repo: Arc<dyn UsersRepository>,
        users_service: Arc<UsersService>,
    ) -> Self {
        let github: Option<Box<dyn OauthProviderClient>> = if config.github.is_configured() {
            Some(Box::new(GithubClient::new(
                config.github.client_id,
                config.github.client_secret,
            )))
        } else {
            None
        };
        let google: Option<Box<dyn OauthProviderClient>> = if config.google.is_configured() {
            Some(Box::new(GoogleClient::new(
                config.google.client_id,
                config.google.client_secret,
            )))
        } else {
            None
        };

        Self {
            tokens,
            users_repo,
            users_service,
            github,
            google,
            api_base_url: config.api_base_url,
            // Timeouts bound how long a wedged GitHub/Google response can
            // hold the callback handler open. The callback runs synchronously
            // inside the user's browser navigation — without these, a slow
            // upstream pegs an Axum worker until the request is dropped.
            http: Client::builder()
                .user_agent("rift-oauth/1.0")
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client builds with static config"),
            // 30/min — looser than magic-link (10/min) because OAuth flows
            // can involve more legitimate retries (back-button, account
            // switcher, "wrong account" → start over).
            ip_limiter: RateLimiter::new(30, 30),
        }
    }

    /// True if at least one provider is configured. Used by `main.rs` to
    /// decide whether to wrap the service in `Some(Arc<...>)` on AppState.
    pub fn any_configured(&self) -> bool {
        self.github.is_some() || self.google.is_some()
    }

    // ── Public API ──

    /// Start the OAuth flow. Mints a state token carrying everything the
    /// callback needs (`provider`, PKCE verifier, `next`, `origin`) and
    /// returns the provider's authorize URL.
    ///
    /// `next` and `origin` are accepted as already-validated by the caller
    /// (route handler runs them through `sanitize_next` and `OriginMatcher`
    /// respectively before this call).
    pub async fn start(
        &self,
        provider: OauthProvider,
        client_ip: &str,
        next: Option<&str>,
        origin: Option<&str>,
    ) -> Result<OauthStartOutcome, OauthError> {
        if !self.ip_limiter.check(client_ip) {
            return Err(OauthError::RateLimited);
        }

        let client = self.client_for(provider)?;

        // PKCE: random verifier, S256-hashed challenge.
        let code_verifier = generate_pkce_verifier();
        let code_challenge = pkce_s256_challenge(&code_verifier);

        let mut metadata = doc! {
            "provider": provider.as_str(),
            "code_verifier": &code_verifier,
        };
        if let Some(n) = next {
            metadata.insert("next", n);
        }
        if let Some(o) = origin {
            metadata.insert("origin", o);
        }

        // `email` field is required on the token doc but irrelevant for OAuth
        // state — we don't know the email yet. Use a placeholder; rate-limit
        // counting is per-IP at the route layer instead of per-email.
        let raw_state = self
            .tokens
            .issue(TokenSpec {
                purpose: TokenPurpose::OauthState,
                kind: TokenKind::HashKeyed,
                ttl_secs: Self::STATE_TTL_SECS,
                email: String::new(),
                metadata,
            })
            .await
            .map_err(OauthError::Internal)?;

        let redirect_uri = self.redirect_uri_for(provider);
        let authorize_url = client.authorize_url(&raw_state, &code_challenge, &redirect_uri);

        Ok(OauthStartOutcome { authorize_url })
    }

    /// Consume the provider's callback. Validates state, exchanges code,
    /// resolves the verified email, and returns the identity to issue a
    /// session for.
    pub async fn consume_callback(
        &self,
        provider: OauthProvider,
        code: &str,
        state: &str,
    ) -> Result<OauthCallbackOutcome, OauthError> {
        let client = self.client_for(provider)?;

        // 1. Validate state token (atomic single-use).
        let outcome = self
            .tokens
            .consume_hash(state)
            .await
            .map_err(OauthError::Internal)?;

        let metadata = match outcome {
            ConsumeOutcome::Ok {
                purpose: TokenPurpose::OauthState,
                metadata,
                ..
            } => metadata,
            // Token valid but for a different flow — refuse so signin /
            // billing tokens can't be redeemed here.
            ConsumeOutcome::Ok { .. } => return Err(OauthError::InvalidState),
            ConsumeOutcome::NotFound | ConsumeOutcome::AttemptsExhausted => {
                return Err(OauthError::InvalidState)
            }
        };

        // 2. Cross-check provider — URL segment vs. token metadata.
        let token_provider = metadata
            .get_str("provider")
            .map_err(|_| OauthError::InvalidState)?;
        if token_provider != provider.as_str() {
            return Err(OauthError::ProviderMismatch);
        }

        let code_verifier = metadata
            .get_str("code_verifier")
            .map_err(|_| OauthError::InvalidState)?
            .to_string();
        let next = metadata
            .get_str("next")
            .ok()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/account".to_string());
        let origin = metadata.get_str("origin").ok().map(|s| s.to_string());

        // 3. Exchange code, fetch + verify email.
        let redirect_uri = self.redirect_uri_for(provider);
        let info = client
            .fetch_user_info(code, &code_verifier, &redirect_uri, &self.http)
            .await?;

        // 4. Resolve user by verified email — email-as-identity. Same path
        // as magic-link signin's consume_sign_in.
        let (user_id, tenant_id) = match self
            .users_repo
            .find_by_email(&info.email)
            .await
            .map_err(OauthError::Internal)?
        {
            Some(user) => {
                let user_id = user.id.unwrap_or_else(crate::core::public_id::UserId::new);
                if !user.verified {
                    let _ = self.users_repo.mark_verified(&info.email).await;
                }
                (user_id, user.tenant_id)
            }
            None => self
                .users_service
                .create_tenant_with_verified_owner(&info.email)
                .await
                .map(|(tenant_id, user_id)| {
                    (
                        crate::core::public_id::UserId::from_object_id(user_id),
                        crate::core::public_id::TenantId::from_object_id(tenant_id),
                    )
                })
                .map_err(|e| OauthError::Internal(e.to_string()))?,
        };

        tracing::info!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            provider = %provider,
            email = %info.email,
            "oauth_callback_consumed"
        );

        Ok(OauthCallbackOutcome {
            user_id,
            tenant_id,
            next,
            origin,
        })
    }
}

// ── Helpers ──

impl OauthService {
    fn client_for(&self, provider: OauthProvider) -> Result<&dyn OauthProviderClient, OauthError> {
        let opt = match provider {
            OauthProvider::Github => self.github.as_deref(),
            OauthProvider::Google => self.google.as_deref(),
        };
        opt.ok_or(OauthError::NotConfigured)
    }

    fn redirect_uri_for(&self, provider: OauthProvider) -> String {
        format!(
            "{}/v1/auth/oauth/{}/callback",
            self.api_base_url.trim_end_matches('/'),
            provider.as_str()
        )
    }
}

/// RFC 7636 §4.1: code_verifier is 43-128 chars from the unreserved set
/// `[A-Z][a-z][0-9]-._~`. Hex of 32 random bytes (= 64 chars, all in set)
/// gives us 256 bits of entropy and stays within the spec.
fn generate_pkce_verifier() -> String {
    let bytes: Vec<u8> = rand::rng().random_iter::<u8>().take(32).collect();
    hex::encode(&bytes)
}

/// RFC 7636 §4.2: `code_challenge = BASE64URL-ENCODE(SHA256(code_verifier))`,
/// no padding.
fn pkce_s256_challenge(verifier: &str) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

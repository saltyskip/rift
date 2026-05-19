//! Provider abstraction: one trait, one impl per provider. GitHub and Google
//! diverge enough at the userinfo step that branching on enum gets ugly —
//! the trait isolates each provider's quirks in its own struct.
//!
//! Verified-email rules are baked into each impl (GitHub filters `verified`
//! and prefers `primary`; Google requires the `email_verified` claim) so the
//! service layer can treat them uniformly.

use super::models::{OauthError, OauthUserInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use url::Url;

#[async_trait]
pub trait OauthProviderClient: Send + Sync {
    /// Build the URL we redirect the browser to. Caller has already minted
    /// the state token (carries CSRF defense + PKCE verifier in metadata)
    /// and the code_challenge.
    fn authorize_url(&self, state: &str, code_challenge: &str, redirect_uri: &str) -> String;

    /// Exchange `code` for an access token, then fetch the userinfo, apply
    /// the verified-email rules, and return a single canonical email.
    async fn fetch_user_info(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        http: &Client,
    ) -> Result<OauthUserInfo, OauthError>;
}

// ── GitHub ──

crate::impl_container!(GithubClient);
/// GitHub OAuth client. Scopes: `read:user user:email` — the second is what
/// lets `/user/emails` return private emails.
pub struct GithubClient {
    client_id: String,
    client_secret: String,
}

impl GithubClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
        }
    }
}

#[async_trait]
impl OauthProviderClient for GithubClient {
    fn authorize_url(&self, state: &str, code_challenge: &str, redirect_uri: &str) -> String {
        let mut url = Url::parse("https://github.com/login/oauth/authorize")
            .expect("github authorize URL is static");
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", "read:user user:email")
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256");
        url.into()
    }

    async fn fetch_user_info(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        http: &Client,
    ) -> Result<OauthUserInfo, OauthError> {
        // 1. Exchange code for access token.
        let token_resp = http
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await
            .map_err(|e| OauthError::ProviderError(format!("github token request: {e}")))?;

        if !token_resp.status().is_success() {
            let status = token_resp.status();
            let body = token_resp.text().await.unwrap_or_default();
            return Err(OauthError::ProviderError(format!(
                "github token endpoint {status}: {body}"
            )));
        }

        let token: GithubTokenResponse = token_resp
            .json()
            .await
            .map_err(|e| OauthError::ProviderError(format!("github token parse: {e}")))?;

        if let Some(err) = token.error {
            return Err(OauthError::ProviderError(format!("github token: {err}")));
        }
        let access_token = token
            .access_token
            .ok_or_else(|| OauthError::ProviderError("github token: empty".into()))?;

        // 2. Fetch /user/emails (private list, requires user:email scope).
        let emails_resp = http
            .get("https://api.github.com/user/emails")
            .bearer_auth(&access_token)
            .header("User-Agent", "rift-oauth")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| OauthError::ProviderError(format!("github emails request: {e}")))?;

        if !emails_resp.status().is_success() {
            let status = emails_resp.status();
            return Err(OauthError::ProviderError(format!(
                "github emails endpoint {status}"
            )));
        }

        let emails: Vec<GithubEmail> = emails_resp
            .json()
            .await
            .map_err(|e| OauthError::ProviderError(format!("github emails parse: {e}")))?;

        // Pick: primary + verified first, then any verified, then nothing.
        let chosen = emails
            .iter()
            .find(|e| e.primary && e.verified)
            .or_else(|| emails.iter().find(|e| e.verified))
            .cloned();

        match chosen {
            Some(e) => Ok(OauthUserInfo {
                email: e.email.to_lowercase(),
            }),
            None => {
                // Distinguish "user has emails but none verified" from "user
                // gave us no emails at all" for a better toast.
                if emails.is_empty() {
                    Err(OauthError::NoEmail)
                } else {
                    Err(OauthError::EmailUnverified)
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct GithubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

// ── Google ──

crate::impl_container!(GoogleClient);
/// Google OAuth client. Scopes: `openid email profile`. The token endpoint
/// returns an `id_token` (signed JWT) that carries `email` + `email_verified`
/// directly — no second HTTP call needed.
///
/// We don't currently verify the ID token's JWT signature against Google's
/// JWKS — we trust the HTTPS-fetched token-endpoint response, which is the
/// confidential-client norm. If we ever switch to a public-client model or
/// expose this to untrusted contexts, add JWKS verification.
pub struct GoogleClient {
    client_id: String,
    client_secret: String,
}

impl GoogleClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
        }
    }
}

#[async_trait]
impl OauthProviderClient for GoogleClient {
    fn authorize_url(&self, state: &str, code_challenge: &str, redirect_uri: &str) -> String {
        let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
            .expect("google authorize URL is static");
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "openid email profile")
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256")
            // access_type=online + prompt=select_account = no refresh token,
            // user can pick an account each time. We don't need offline access.
            .append_pair("access_type", "online")
            .append_pair("prompt", "select_account");
        url.into()
    }

    async fn fetch_user_info(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        http: &Client,
    ) -> Result<OauthUserInfo, OauthError> {
        let token_resp = http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("grant_type", "authorization_code"),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await
            .map_err(|e| OauthError::ProviderError(format!("google token request: {e}")))?;

        if !token_resp.status().is_success() {
            let status = token_resp.status();
            let body = token_resp.text().await.unwrap_or_default();
            return Err(OauthError::ProviderError(format!(
                "google token endpoint {status}: {body}"
            )));
        }

        let token: GoogleTokenResponse = token_resp
            .json()
            .await
            .map_err(|e| OauthError::ProviderError(format!("google token parse: {e}")))?;

        let id_token = token
            .id_token
            .ok_or_else(|| OauthError::ProviderError("google token: no id_token".into()))?;

        // ID token is a JWT: `header.payload.signature`. We trust the
        // HTTPS-fetched response and just decode the payload — no JWKS
        // verification (confidential-client norm).
        let claims = parse_google_id_token_unverified(&id_token)?;

        if !claims.email_verified.unwrap_or(false) {
            return Err(OauthError::EmailUnverified);
        }
        let email = claims.email.ok_or(OauthError::NoEmail)?;

        Ok(OauthUserInfo {
            email: email.to_lowercase(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleIdTokenClaims {
    email: Option<String>,
    email_verified: Option<bool>,
}

/// Decode the payload of a Google ID token JWT without verifying the
/// signature. Safe in confidential-client OAuth flows where the token came
/// directly from `https://oauth2.googleapis.com/token` (TLS-authenticated).
fn parse_google_id_token_unverified(jwt: &str) -> Result<GoogleIdTokenClaims, OauthError> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    let mut parts = jwt.split('.');
    let _header = parts
        .next()
        .ok_or_else(|| OauthError::ProviderError("google id_token: missing header".into()))?;
    let payload_b64 = parts
        .next()
        .ok_or_else(|| OauthError::ProviderError("google id_token: missing payload".into()))?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| OauthError::ProviderError(format!("google id_token b64: {e}")))?;
    serde_json::from_slice(&payload_bytes)
        .map_err(|e| OauthError::ProviderError(format!("google id_token json: {e}")))
}

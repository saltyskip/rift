//! Targeted unit tests for the OAuth service's internal helpers and provider
//! URL construction. Full end-to-end flow (state token → code exchange →
//! session mint) is exercised via integration tests / manual QA against real
//! GitHub + Google OAuth apps — wiring a faithful HTTP mock for both
//! providers is more code than it's worth at v1.

use super::*;
use crate::services::auth::oauth::providers::{GithubClient, GoogleClient, OauthProviderClient};
use sha2::{Digest, Sha256};

#[test]
fn pkce_challenge_matches_rfc_7636() {
    // RFC 7636 §B.1: code_verifier =
    //   "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    // code_challenge =
    //   "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let expected = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    assert_eq!(pkce_s256_challenge(verifier), expected);
}

#[test]
fn pkce_verifier_is_high_entropy_and_url_safe() {
    let v1 = generate_pkce_verifier();
    let v2 = generate_pkce_verifier();
    assert_ne!(v1, v2, "verifiers must not repeat");
    assert_eq!(v1.len(), 64, "hex-encoded 32 bytes = 64 chars");
    // Hex chars are a strict subset of the RFC 7636 unreserved set.
    for c in v1.chars() {
        assert!(
            c.is_ascii_hexdigit(),
            "verifier must be url-safe (hex only): {c}"
        );
    }
}

#[test]
fn pkce_challenge_round_trips_with_verifier() {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    let verifier = generate_pkce_verifier();
    let challenge = pkce_s256_challenge(&verifier);
    let decoded = URL_SAFE_NO_PAD.decode(&challenge).unwrap();
    assert_eq!(decoded, Sha256::digest(verifier.as_bytes()).to_vec());
}

#[test]
fn provider_from_path_segment_rejects_unknown() {
    assert_eq!(
        OauthProvider::from_path_segment("github"),
        Some(OauthProvider::Github)
    );
    assert_eq!(
        OauthProvider::from_path_segment("google"),
        Some(OauthProvider::Google)
    );
    assert_eq!(OauthProvider::from_path_segment("apple"), None);
    assert_eq!(OauthProvider::from_path_segment(""), None);
    assert_eq!(OauthProvider::from_path_segment("GITHUB"), None);
}

#[test]
fn github_authorize_url_includes_required_params() {
    let client = GithubClient::new("client123".into(), "secret".into());
    let url = client.authorize_url(
        "state-abc",
        "challenge-xyz",
        "https://api.example.com/v1/auth/oauth/github/callback",
    );

    assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
    assert!(url.contains("client_id=client123"));
    assert!(url.contains("scope=read%3Auser+user%3Aemail"));
    assert!(url.contains("state=state-abc"));
    assert!(url.contains("code_challenge=challenge-xyz"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains(
        "redirect_uri=https%3A%2F%2Fapi.example.com%2Fv1%2Fauth%2Foauth%2Fgithub%2Fcallback"
    ));
}

#[test]
fn google_authorize_url_includes_required_params() {
    let client = GoogleClient::new("client456".into(), "secret".into());
    let url = client.authorize_url(
        "state-def",
        "challenge-pqr",
        "https://api.example.com/v1/auth/oauth/google/callback",
    );

    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
    assert!(url.contains("client_id=client456"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("scope=openid+email+profile"));
    assert!(url.contains("state=state-def"));
    assert!(url.contains("code_challenge=challenge-pqr"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("prompt=select_account"));
    assert!(url.contains("access_type=online"));
}

#[test]
fn provider_credentials_configured_check() {
    use crate::services::auth::oauth::models::ProviderCredentials;
    let empty = ProviderCredentials {
        client_id: "".into(),
        client_secret: "".into(),
    };
    let partial = ProviderCredentials {
        client_id: "x".into(),
        client_secret: "".into(),
    };
    let full = ProviderCredentials {
        client_id: "x".into(),
        client_secret: "y".into(),
    };
    assert!(!empty.is_configured());
    assert!(!partial.is_configured());
    assert!(full.is_configured());
}

#[test]
fn oauth_error_codes_are_stable() {
    use crate::services::auth::oauth::models::OauthError;
    assert_eq!(OauthError::InvalidState.code(), "oauth_state_invalid");
    assert_eq!(OauthError::ProviderMismatch.code(), "oauth_state_invalid");
    assert_eq!(OauthError::EmailUnverified.code(), "oauth_email_unverified");
    assert_eq!(OauthError::NoEmail.code(), "oauth_no_email");
    assert_eq!(
        OauthError::ProviderError("x".into()).code(),
        "oauth_provider_error"
    );
    assert_eq!(OauthError::NotConfigured.code(), "oauth_not_configured");
}

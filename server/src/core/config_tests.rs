//! Cookie-domain derivation is the critical path: setting the wrong Domain
//! either silently drops the cookie (no Domain match → no auth) or leaks a
//! sandbox session into prod. Every supported deployment topology is pinned
//! here.

use super::{resolve_cookie_domain, CookieSameSite};

#[test]
fn prod_marketing_yields_riftl_ink_parent() {
    assert_eq!(
        resolve_cookie_domain(None, "https://riftl.ink"),
        Some(".riftl.ink".to_string())
    );
}

#[test]
fn sandbox_marketing_yields_sandbox_subtree_no_prod_leak() {
    // The whole point: sandbox cookies stay inside .sandbox.riftl.ink and
    // are not sent to https://riftl.ink. If this returned ".riftl.ink",
    // sandbox sessions would leak into prod.
    assert_eq!(
        resolve_cookie_domain(None, "https://sandbox.riftl.ink"),
        Some(".sandbox.riftl.ink".to_string())
    );
}

#[test]
fn deeper_subdomain_yields_its_own_subtree() {
    // `team-x.staging.riftl.ink` → `.team-x.staging.riftl.ink`. We
    // deliberately do not climb to the registrable suffix; each
    // subdomain owns its own cookie scope.
    assert_eq!(
        resolve_cookie_domain(None, "https://team-x.staging.riftl.ink"),
        Some(".team-x.staging.riftl.ink".to_string())
    );
}

#[test]
fn localhost_returns_none() {
    // Browsers refuse Domain= attributes that don't match the response
    // host; on localhost the only safe answer is "no Domain attribute"
    // so the cookie sticks to the host that set it.
    assert_eq!(resolve_cookie_domain(None, "http://localhost:3000"), None);
    assert_eq!(resolve_cookie_domain(None, "http://localhost"), None);
}

#[test]
fn ip_addresses_return_none() {
    // Same reasoning as localhost. Both v4 and v6.
    assert_eq!(resolve_cookie_domain(None, "http://127.0.0.1:3000"), None);
    assert_eq!(resolve_cookie_domain(None, "http://192.168.1.5"), None);
    assert_eq!(resolve_cookie_domain(None, "http://[::1]:3000"), None);
}

#[test]
fn invalid_marketing_url_returns_none() {
    // Fail closed rather than panic if MARKETING_URL is misconfigured.
    assert_eq!(resolve_cookie_domain(None, "not a url"), None);
    assert_eq!(resolve_cookie_domain(None, ""), None);
}

#[test]
fn override_with_leading_dot_passes_through() {
    assert_eq!(
        resolve_cookie_domain(Some(".custom.example"), "https://anything.com"),
        Some(".custom.example".to_string())
    );
}

#[test]
fn override_without_leading_dot_gets_normalised() {
    assert_eq!(
        resolve_cookie_domain(Some("custom.example"), "https://anything.com"),
        Some(".custom.example".to_string())
    );
}

#[test]
fn override_empty_or_whitespace_falls_back_to_none() {
    // Empty string explicitly disables the Domain attribute. Useful for
    // single-host deployments where you don't want subdomain scoping —
    // including the sandbox/Vercel-preview setup where the cookie must
    // stick to the exact API host because marketing lives on a different
    // parent domain.
    assert_eq!(resolve_cookie_domain(Some(""), "https://riftl.ink"), None);
    assert_eq!(
        resolve_cookie_domain(Some("   "), "https://riftl.ink"),
        None
    );
}

#[test]
fn same_site_defaults_to_lax() {
    // Unset, empty, garbage, and "lax" all land on Lax — typo doesn't
    // crash the server at startup.
    assert_eq!(CookieSameSite::from_env_str(None), CookieSameSite::Lax);
    assert_eq!(CookieSameSite::from_env_str(Some("")), CookieSameSite::Lax);
    assert_eq!(
        CookieSameSite::from_env_str(Some("garbage")),
        CookieSameSite::Lax
    );
    assert_eq!(
        CookieSameSite::from_env_str(Some("lax")),
        CookieSameSite::Lax
    );
}

#[test]
fn same_site_parses_none_and_strict_case_insensitive() {
    // Cross-origin sandbox/preview testing sets None explicitly.
    assert_eq!(
        CookieSameSite::from_env_str(Some("None")),
        CookieSameSite::None
    );
    assert_eq!(
        CookieSameSite::from_env_str(Some("NONE")),
        CookieSameSite::None
    );
    assert_eq!(
        CookieSameSite::from_env_str(Some("Strict")),
        CookieSameSite::Strict
    );
    assert_eq!(
        CookieSameSite::from_env_str(Some("strict")),
        CookieSameSite::Strict
    );
}

#[test]
fn same_site_as_str_round_trips_into_set_cookie_form() {
    // The `Set-Cookie` attribute value the browser expects: title-case.
    assert_eq!(CookieSameSite::Lax.as_str(), "Lax");
    assert_eq!(CookieSameSite::Strict.as_str(), "Strict");
    assert_eq!(CookieSameSite::None.as_str(), "None");
}

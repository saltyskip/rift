//! `sanitize_next` is the open-redirect gate for the signin callback. Every
//! input listed here either redirects somewhere the browser ultimately
//! resolves off-origin (rejected) or stays on the marketing site (accepted).

use super::{build_cookie, sanitize_next};
use crate::core::config::CookieSameSite;

const BASE: &str = "https://riftl.ink";

#[test]
fn accepts_plain_relative_path() {
    assert_eq!(
        sanitize_next(BASE, "/account"),
        Some("/account".to_string())
    );
}

#[test]
fn accepts_nested_path() {
    assert_eq!(
        sanitize_next(BASE, "/account/keys"),
        Some("/account/keys".to_string())
    );
}

#[test]
fn accepts_path_with_query() {
    assert_eq!(
        sanitize_next(BASE, "/account?tab=keys"),
        Some("/account?tab=keys".to_string())
    );
}

#[test]
fn drops_fragment_silently() {
    // Fragments never reach the server anyway; the redirect target carries
    // only path + query.
    assert_eq!(
        sanitize_next(BASE, "/account#tokens"),
        Some("/account".to_string())
    );
}

#[test]
fn rejects_empty() {
    assert_eq!(sanitize_next(BASE, ""), None);
}

#[test]
fn rejects_protocol_relative() {
    // `//evil.com` joined against the base produces `https://evil.com/`
    // (different origin) — the canonical open-redirect bypass.
    assert_eq!(sanitize_next(BASE, "//evil.com"), None);
    assert_eq!(sanitize_next(BASE, "//evil.com/path"), None);
}

#[test]
fn rejects_absolute_http_url() {
    assert_eq!(sanitize_next(BASE, "https://evil.com"), None);
    assert_eq!(sanitize_next(BASE, "http://evil.com/path"), None);
}

#[test]
fn rejects_scheme_only() {
    assert_eq!(sanitize_next(BASE, "javascript:alert(1)"), None);
    assert_eq!(sanitize_next(BASE, "data:text/html,<script>"), None);
}

#[test]
fn rejects_backslash() {
    // Browsers normalize `\` to `/` in path context; defense in depth.
    assert_eq!(sanitize_next(BASE, "/\\evil.com"), None);
    assert_eq!(sanitize_next(BASE, "\\evil.com"), None);
}

#[test]
fn rejects_control_characters() {
    // Browsers strip whitespace/CR/LF/tab before parsing URLs, so a payload
    // the URL parser treats as same-origin may navigate elsewhere after
    // browser preprocessing.
    assert_eq!(sanitize_next(BASE, "/\tevil.com"), None);
    assert_eq!(sanitize_next(BASE, "/\nevil.com"), None);
    assert_eq!(sanitize_next(BASE, "/\revil.com"), None);
    // NUL and other C0 control codes.
    assert_eq!(sanitize_next(BASE, "/\x00account"), None);
    assert_eq!(sanitize_next(BASE, "/\x07account"), None);
}

#[test]
fn rejects_when_base_url_is_invalid() {
    // Pathological config — fail closed rather than panic.
    assert_eq!(sanitize_next("not a url", "/account"), None);
}

#[test]
fn url_join_handles_dot_segments() {
    // `..` traversal stays same-origin (Url::join normalises), so this is
    // structurally safe — included so we know what to expect.
    assert_eq!(sanitize_next(BASE, "/a/../b"), Some("/b".to_string()));
}

#[test]
fn build_cookie_prod_shape() {
    // Production profile: scoped to parent, Secure, Lax.
    let c = build_cookie(
        "abc",
        2592000,
        Some(".riftl.ink"),
        true,
        CookieSameSite::Lax,
    );
    assert_eq!(
        c,
        "session_token=abc; Domain=.riftl.ink; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=2592000"
    );
}

#[test]
fn build_cookie_sandbox_cross_origin_shape() {
    // Sandbox-with-Vercel-preview profile: no Domain (sticks to exact host),
    // Secure (required by SameSite=None), SameSite=None for cross-origin.
    let c = build_cookie("abc", 2592000, None, true, CookieSameSite::None);
    assert_eq!(
        c,
        "session_token=abc; Path=/; HttpOnly; Secure; SameSite=None; Max-Age=2592000"
    );
}

#[test]
fn build_cookie_dev_shape() {
    // Localhost: no Domain, no Secure, Lax.
    let c = build_cookie("abc", 2592000, None, false, CookieSameSite::Lax);
    assert_eq!(
        c,
        "session_token=abc; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000"
    );
}

#[test]
fn clearing_cookie_matches_issuing_attrs() {
    // The signout cookie must mirror the issuing cookie's attrs (minus the
    // value + Max-Age) or browsers won't scrub the original. M4 in the
    // review pinned this as an invariant.
    let issued = build_cookie(
        "abc",
        2592000,
        Some(".riftl.ink"),
        true,
        CookieSameSite::Lax,
    );
    let cleared = build_cookie("", 0, Some(".riftl.ink"), true, CookieSameSite::Lax);
    // Strip the value and Max-Age, compare the rest.
    let strip = |s: &str| {
        s.replace("session_token=abc; ", "")
            .replace("session_token=; ", "")
            .replace("Max-Age=2592000", "Max-Age=N")
            .replace("Max-Age=0", "Max-Age=N")
    };
    assert_eq!(strip(&issued), strip(&cleared));
}

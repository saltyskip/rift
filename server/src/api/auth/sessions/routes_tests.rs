//! `sanitize_next` is the open-redirect gate for the signin callback. Every
//! input listed here either redirects somewhere the browser ultimately
//! resolves off-origin (rejected) or stays on the marketing site (accepted).

use super::sanitize_next;

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

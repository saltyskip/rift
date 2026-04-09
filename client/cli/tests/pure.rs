use rift_cli::commands::init::looks_like_email;
use rift_cli::commands::setup_domain::{display_record_name, normalize_host};
use rift_cli::util::normalize_web_url;

// ── normalize_web_url ──

#[test]
fn normalize_web_url_adds_https() {
    assert_eq!(normalize_web_url("bitcoin.com"), "https://bitcoin.com");
}

#[test]
fn normalize_web_url_preserves_https() {
    assert_eq!(
        normalize_web_url("https://bitcoin.com"),
        "https://bitcoin.com"
    );
}

#[test]
fn normalize_web_url_preserves_http() {
    assert_eq!(
        normalize_web_url("http://bitcoin.com"),
        "http://bitcoin.com"
    );
}

#[test]
fn normalize_web_url_preserves_other_schemes() {
    assert_eq!(normalize_web_url("ftp://host"), "ftp://host");
}

#[test]
fn normalize_web_url_trims_whitespace() {
    assert_eq!(normalize_web_url("  bitcoin.com  "), "https://bitcoin.com");
}

// ── looks_like_email ──

#[test]
fn email_valid() {
    assert!(looks_like_email("alice@example.com"));
}

#[test]
fn email_trimmed() {
    assert!(looks_like_email("  alice@example.com  "));
}

#[test]
fn email_missing_at() {
    assert!(!looks_like_email("alice"));
}

#[test]
fn email_empty() {
    assert!(!looks_like_email(""));
}

#[test]
fn email_no_dot_in_domain() {
    assert!(!looks_like_email("alice@example"));
}

#[test]
fn email_empty_local() {
    assert!(!looks_like_email("@example.com"));
}

#[test]
fn email_domain_starts_with_dot() {
    assert!(!looks_like_email("alice@.example.com"));
}

#[test]
fn email_domain_ends_with_dot() {
    assert!(!looks_like_email("alice@example.com."));
}

#[test]
fn email_contains_whitespace() {
    assert!(!looks_like_email("alice @example.com"));
}

// ── normalize_host ──

#[test]
fn normalize_host_strips_https() {
    assert_eq!(normalize_host("https://bitcoin.com"), "bitcoin.com");
}

#[test]
fn normalize_host_strips_http() {
    assert_eq!(normalize_host("http://bitcoin.com"), "bitcoin.com");
}

#[test]
fn normalize_host_strips_trailing_slash() {
    assert_eq!(normalize_host("bitcoin.com/"), "bitcoin.com");
}

#[test]
fn normalize_host_strips_scheme_and_slash() {
    assert_eq!(normalize_host("https://bitcoin.com/"), "bitcoin.com");
}

#[test]
fn normalize_host_trims_whitespace() {
    assert_eq!(normalize_host("  bitcoin.com  "), "bitcoin.com");
}

#[test]
fn normalize_host_bare_domain() {
    assert_eq!(normalize_host("bitcoin.com"), "bitcoin.com");
}

// ── display_record_name ──

#[test]
fn display_record_name_strips_root_suffix() {
    assert_eq!(
        display_record_name("_rift-verify.go.bitcoin.com", "bitcoin.com"),
        "_rift-verify.go"
    );
}

#[test]
fn display_record_name_strips_direct_subdomain() {
    assert_eq!(
        display_record_name("_rift-verify.bitcoin.com", "bitcoin.com"),
        "_rift-verify"
    );
}

#[test]
fn display_record_name_no_match_returns_full() {
    assert_eq!(
        display_record_name("unrelated.example.com", "bitcoin.com"),
        "unrelated.example.com"
    );
}

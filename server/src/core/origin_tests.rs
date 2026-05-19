//! `OriginMatcher` is the single source of truth for "is this origin
//! allowed?" — both CORS and signin route through it.

use super::OriginMatcher;
use axum::http::HeaderValue;
use std::sync::Arc;

fn matcher_with(exact: &[&str], regex: Option<&str>) -> Arc<OriginMatcher> {
    let exact: Vec<HeaderValue> = exact
        .iter()
        .map(|s| HeaderValue::from_str(s).unwrap())
        .collect();
    let regex = regex.map(|pat| regex::Regex::new(&format!("^(?:{pat})$")).unwrap());
    Arc::new(OriginMatcher { exact, regex })
}

#[test]
fn exact_match_accepts() {
    let m = matcher_with(&["https://riftl.ink"], None);
    assert!(m.matches(&HeaderValue::from_static("https://riftl.ink")));
}

#[test]
fn exact_mismatch_with_no_regex_rejects() {
    let m = matcher_with(&["https://riftl.ink"], None);
    assert!(!m.matches(&HeaderValue::from_static("https://evil.com")));
}

#[test]
fn regex_matches_vercel_preview_per_commit() {
    // Per-commit Vercel URL: rift-{hash}-{team}.vercel.app
    let m = matcher_with(
        &[],
        Some(r"https://rift-[a-z0-9-]+-andrei-bitcoincoms-projects\.vercel\.app"),
    );
    assert!(m.matches(&HeaderValue::from_static(
        "https://rift-6wtuldt9k-andrei-bitcoincoms-projects.vercel.app"
    )));
}

#[test]
fn regex_matches_vercel_preview_per_branch() {
    // Per-branch Vercel URL: rift-git-{branch}-{team}.vercel.app — same
    // permissive pattern covers both shapes.
    let m = matcher_with(
        &[],
        Some(r"https://rift-[a-z0-9-]+-andrei-bitcoincoms-projects\.vercel\.app"),
    );
    assert!(m.matches(&HeaderValue::from_static(
        "https://rift-git-feat-sessions-auth-phase1-andrei-bitcoincoms-projects.vercel.app"
    )));
}

#[test]
fn anchoring_rejects_trailing_junk() {
    // `from_env` wraps the pattern in `^(?:...)$`; replicate here so the
    // assertion exercises the same shape.
    let m = matcher_with(&[], Some(r"https://riftl\.ink"));
    assert!(m.matches(&HeaderValue::from_static("https://riftl.ink")));
    assert!(!m.matches(&HeaderValue::from_static("https://riftl.ink.evil.com")));
    assert!(!m.matches(&HeaderValue::from_static(
        "https://evil.com/https://riftl.ink"
    )));
}

#[test]
fn exact_takes_precedence_over_regex() {
    let m = matcher_with(&["https://riftl.ink"], Some(r"https://never\.matches"));
    assert!(m.matches(&HeaderValue::from_static("https://riftl.ink")));
}

#[test]
fn malformed_header_rejected() {
    let m = matcher_with(&[], Some(r"https://.*"));
    let bad = HeaderValue::from_bytes(b"\xff\xfe").unwrap();
    assert!(!m.matches(&bad));
}

#[test]
fn matches_str_round_trips_to_matches() {
    let m = matcher_with(&["https://riftl.ink"], None);
    assert!(m.matches_str("https://riftl.ink"));
    assert!(!m.matches_str("https://evil.com"));
    // Invalid header strings (control chars, etc.) reject cleanly.
    assert!(!m.matches_str("not a url\n"));
}

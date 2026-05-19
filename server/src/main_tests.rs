//! `origin_matches` is the CORS decision function. Exact-match wins; the
//! regex (when configured) is the dynamic-origin escape hatch for Vercel
//! preview URLs and similar.

use super::origin_matches;
use axum::http::HeaderValue;

fn exact(values: &[&str]) -> Vec<HeaderValue> {
    values
        .iter()
        .map(|s| HeaderValue::from_str(s).unwrap())
        .collect()
}

fn anchored_regex(pattern: &str) -> regex::Regex {
    // Mirror the anchoring `build_cors_layer` applies, so tests cover the
    // actual semantics.
    regex::Regex::new(&format!("^(?:{pattern})$")).unwrap()
}

#[test]
fn exact_match_accepts() {
    let allowed = exact(&["https://riftl.ink"]);
    assert!(origin_matches(
        &HeaderValue::from_static("https://riftl.ink"),
        &allowed,
        None,
    ));
}

#[test]
fn exact_mismatch_with_no_regex_rejects() {
    let allowed = exact(&["https://riftl.ink"]);
    assert!(!origin_matches(
        &HeaderValue::from_static("https://evil.com"),
        &allowed,
        None,
    ));
}

#[test]
fn regex_matches_vercel_preview() {
    // Real Vercel preview pattern: project-git-branch-team.vercel.app
    let re =
        anchored_regex(r"https://rift-git-[a-z0-9-]+-andrei-bitcoincoms-projects\.vercel\.app");
    let origin = HeaderValue::from_static(
        "https://rift-git-feat-sessions-auth-phase1-andrei-bitcoincoms-projects.vercel.app",
    );
    assert!(origin_matches(&origin, &[], Some(&re)));
}

#[test]
fn regex_admits_team_name_impersonation_documented_limitation() {
    // **Documented limitation:** Vercel preview URLs put the team slug
    // inside the *same* DNS label as the project/branch (`{proj}-git-{branch}-{team}.vercel.app`).
    // No regex can safely distinguish "the team I trust" from "an attacker
    // who registered a Vercel team named `evil-<my-team>`" — both URLs
    // satisfy the same `-<my-team>.vercel.app` suffix pattern.
    //
    // The regex below would let an impersonator through. The mitigation is
    // out-of-band: either list specific preview URLs in `ALLOWED_ORIGINS`,
    // accept the (narrow) risk for non-production environments, or move
    // previews onto a custom domain (`*.preview.riftl.ink`) where DNS-label
    // boundaries can be enforced safely.
    let re =
        anchored_regex(r"https://rift-git-[a-z0-9-]+-andrei-bitcoincoms-projects\.vercel\.app");
    let attacker =
        HeaderValue::from_static("https://rift-git-x-evil-andrei-bitcoincoms-projects.vercel.app");
    assert!(
        origin_matches(&attacker, &[], Some(&re)),
        "Regex matched as expected; the impersonation risk is documented, not prevented at this layer"
    );
}

#[test]
fn regex_is_fully_anchored() {
    // Even if the caller forgets `^...$`, build_cors_layer adds them.
    // Verify a pattern without anchors doesn't allow trailing junk.
    let re = anchored_regex(r"https://riftl\.ink");
    assert!(origin_matches(
        &HeaderValue::from_static("https://riftl.ink"),
        &[],
        Some(&re),
    ));
    assert!(!origin_matches(
        &HeaderValue::from_static("https://riftl.ink.evil.com"),
        &[],
        Some(&re),
    ));
    assert!(!origin_matches(
        &HeaderValue::from_static("https://evil.com/https://riftl.ink"),
        &[],
        Some(&re),
    ));
}

#[test]
fn exact_takes_precedence_over_regex() {
    // Exact list is checked first; an exact hit doesn't bother the regex.
    let allowed = exact(&["https://riftl.ink"]);
    let re = anchored_regex(r"https://never\.matches");
    assert!(origin_matches(
        &HeaderValue::from_static("https://riftl.ink"),
        &allowed,
        Some(&re),
    ));
}

#[test]
fn rejects_malformed_origin_header() {
    let re = anchored_regex(r"https://.*");
    // Non-ASCII bytes (shouldn't happen — browsers send valid Origin
    // values) shouldn't panic or be allowed.
    let bad = HeaderValue::from_bytes(b"\xff\xfe").unwrap();
    assert!(!origin_matches(&bad, &[], Some(&re)));
}

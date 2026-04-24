use super::*;

#[test]
fn normalize_strips_fragment_and_trailing_slash() {
    assert_eq!(normalize_url("https://example.com/"), "https://example.com");
    assert_eq!(
        normalize_url("https://example.com/path#frag"),
        "https://example.com/path"
    );
    assert_eq!(normalize_url("HTTPS://EXAMPLE.COM"), "https://example.com");
}

#[tokio::test]
async fn check_exact_url_match() {
    let feed = ThreatFeed::new();
    feed.urls
        .write()
        .await
        .insert("https://evil.com/malware.exe".to_string());

    assert!(feed
        .check_url("https://evil.com/malware.exe")
        .await
        .is_some());
    assert!(feed.check_url("https://safe.com").await.is_none());
}

#[tokio::test]
async fn check_domain_match() {
    let feed = ThreatFeed::new();
    feed.domains
        .write()
        .await
        .insert("phishing-site.com".to_string());

    assert!(feed
        .check_url("https://phishing-site.com/login")
        .await
        .is_some());
    assert!(feed
        .check_url("https://sub.phishing-site.com/fake")
        .await
        .is_some());
    assert!(feed
        .check_url("https://legit-site.com/page")
        .await
        .is_none());
}

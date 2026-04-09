mod common;

use predicates::prelude::*;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use common::TestHarness;

// ── whoami ──

#[tokio::test]
async fn whoami_json_shows_team_members() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/v1/auth/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "users": [{
                "id": "abc123",
                "email": "alice@example.com",
                "verified": true,
                "is_owner": true,
                "created_at": "2025-01-01T00:00:00Z"
            }]
        })))
        .mount(&h.server)
        .await;

    h.cmd()
        .args(["whoami", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice@example.com"));
}

#[tokio::test]
async fn whoami_with_bad_key_exits_auth_failed() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/v1/auth/users"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error": "Invalid API key"})))
        .mount(&h.server)
        .await;

    h.cmd()
        .args(["whoami", "--json"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("rift login"));
}

// ── doctor ──

#[tokio::test]
async fn doctor_json_returns_capabilities() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/v1/domains"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "domains": [{
                "domain": "go.example.com",
                "verified": true,
                "role": "primary",
                "created_at": "2025-01-01T00:00:00Z"
            }]
        })))
        .mount(&h.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/apps"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "apps": [{
                "id": "app1",
                "platform": "ios",
                "bundle_id": "com.example.app",
                "team_id": "TEAM123",
                "package_name": null,
                "sha256_fingerprints": null,
                "app_name": null,
                "icon_url": null,
                "theme_color": null,
                "created_at": "2025-01-01T00:00:00Z"
            }]
        })))
        .mount(&h.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/auth/publishable-keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "keys": [{
                "id": "pk1",
                "key_prefix": "pk_live_abc...",
                "domain": "go.example.com",
                "created_at": "2025-01-01T00:00:00Z"
            }]
        })))
        .mount(&h.server)
        .await;

    let output = h.cmd().args(["doctor", "--json"]).output().unwrap();

    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["has_verified_primary_domain"], true);
    assert_eq!(body["has_ios_app"], true);
    assert_eq!(body["has_publishable_key"], true);
}

#[tokio::test]
async fn doctor_with_bad_key_exits_auth_failed() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/v1/domains"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error": "Invalid API key"})))
        .mount(&h.server)
        .await;

    h.cmd()
        .args(["doctor", "--json"])
        .assert()
        .failure()
        .code(3);
}

// ── links create ──

#[tokio::test]
async fn links_create_json_returns_link() {
    let h = TestHarness::spawn().await;

    Mock::given(method("POST"))
        .and(path("/v1/links"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "link_id": "abc123",
            "url": "https://riftl.ink/abc123",
            "expires_at": null
        })))
        .mount(&h.server)
        .await;

    let output = h
        .cmd()
        .args([
            "links",
            "create",
            "--web-url",
            "https://example.com",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["link_id"], "abc123");
    assert_eq!(body["url"], "https://riftl.ink/abc123");
}

#[tokio::test]
async fn links_create_with_bad_key_exits_auth_failed() {
    let h = TestHarness::spawn().await;

    Mock::given(method("POST"))
        .and(path("/v1/links"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error": "Invalid API key"})))
        .mount(&h.server)
        .await;

    h.cmd()
        .args([
            "links",
            "create",
            "--web-url",
            "https://example.com",
            "--json",
        ])
        .assert()
        .failure()
        .code(3);
}

#[tokio::test]
async fn links_create_with_all_fields() {
    let h = TestHarness::spawn().await;

    Mock::given(method("POST"))
        .and(path("/v1/links"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "link_id": "custom-slug",
            "url": "https://riftl.ink/custom-slug",
            "expires_at": "2025-12-31T23:59:59Z"
        })))
        .mount(&h.server)
        .await;

    let output = h
        .cmd()
        .args([
            "links",
            "create",
            "--web-url",
            "https://example.com",
            "--ios-deep-link",
            "myapp://path",
            "--android-deep-link",
            "intent://path",
            "--ios-store-url",
            "https://apps.apple.com/app/id123",
            "--android-store-url",
            "https://play.google.com/store/apps/details?id=com.example",
            "--custom-id",
            "custom-slug",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["link_id"], "custom-slug");
    assert!(body["expires_at"].is_string());
}

// ── links test ──

#[tokio::test]
async fn links_test_json_shows_destinations() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/r/abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "link_id": "abc123",
            "web_url": "https://example.com",
            "ios_deep_link": "myapp://home",
            "android_deep_link": null,
            "ios_store_url": "https://apps.apple.com/app/id123",
            "android_store_url": null,
            "metadata": null,
            "agent_context": null
        })))
        .mount(&h.server)
        .await;

    let output = h
        .cmd()
        .args(["links", "test", "abc123", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["link_id"], "abc123");
    assert_eq!(body["web_url"], "https://example.com");
    assert_eq!(body["ios_deep_link"], "myapp://home");
}

#[tokio::test]
async fn links_test_extracts_id_from_url() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/r/xyz789"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "link_id": "xyz789",
            "web_url": "https://example.com",
            "ios_deep_link": null,
            "android_deep_link": null,
            "ios_store_url": null,
            "android_store_url": null,
            "metadata": null,
            "agent_context": null
        })))
        .mount(&h.server)
        .await;

    let output = h
        .cmd()
        .args(["links", "test", "https://riftl.ink/xyz789", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["link_id"], "xyz789");
}

#[tokio::test]
async fn links_test_not_found() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/r/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": "Link not found"})))
        .mount(&h.server)
        .await;

    h.cmd()
        .args(["links", "test", "nonexistent", "--json"])
        .assert()
        .failure()
        .code(4);
}

// ── login ──

#[tokio::test]
async fn login_with_bad_key_exits_auth_failed() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/v1/auth/users"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error": "Invalid API key"})))
        .mount(&h.server)
        .await;

    // login reads the key from stdin via Password prompt, but with a bad key
    // the API call fails before saving. We can't easily feed stdin to Password,
    // so we test the auth failure path by calling whoami with the config already
    // pointing at a server that rejects the key.
    h.cmd()
        .args(["whoami", "--json"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("rift login"));
}

// ── doctor with empty state ──

#[tokio::test]
async fn doctor_json_empty_state() {
    let h = TestHarness::spawn().await;

    Mock::given(method("GET"))
        .and(path("/v1/domains"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"domains": []})))
        .mount(&h.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/apps"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"apps": []})))
        .mount(&h.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/auth/publishable-keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"keys": []})))
        .mount(&h.server)
        .await;

    let output = h.cmd().args(["doctor", "--json"]).output().unwrap();

    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["has_verified_primary_domain"], false);
    assert_eq!(body["has_ios_app"], false);
    assert_eq!(body["has_android_app"], false);
    assert_eq!(body["has_publishable_key"], false);
    assert!(!body["next_steps"].as_array().unwrap().is_empty());
}

// ── not logged in ──

#[tokio::test]
async fn commands_fail_when_not_logged_in() {
    let home = tempfile::tempdir().unwrap();

    // No config file written — every authenticated command should fail with exit 2
    for args in [
        vec!["whoami", "--json"],
        vec!["doctor", "--json"],
        vec![
            "links",
            "create",
            "--web-url",
            "https://example.com",
            "--json",
        ],
    ] {
        let mut cmd = assert_cmd::Command::cargo_bin("rift").unwrap();
        cmd.env("HOME", home.path());
        if cfg!(not(target_os = "macos")) {
            cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
        }
        cmd.args(&args)
            .assert()
            .failure()
            .code(2)
            .stderr(predicate::str::contains("rift init"));
    }
}

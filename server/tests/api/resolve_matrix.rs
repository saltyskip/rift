use mongodb::bson::oid::ObjectId;
use serde_json::json;

use crate::common;
use common::seed_verified_domain;

const IOS_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
const ANDROID_UA: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";
const DESKTOP_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Create a fully-configured test link with all platform destinations.
async fn create_test_link(app: &common::TestApp, key: &str, tenant_id: &ObjectId) {
    seed_verified_domain(app, tenant_id, "go.example.com").await;
    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&json!({
            "custom_id": "test-link",
            "web_url": "https://example.com",
            "ios_store_url": "https://apps.apple.com/app/id123",
            "android_store_url": "https://play.google.com/store/apps/details?id=com.example",
            "agent_context": {
                "action": "download",
                "cta": "Get the App",
                "description": "Test app description"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
}

fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

// ── redirect=1 tests ──

#[tokio::test]
async fn redirect_ios_goes_to_app_store() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link?redirect=1"))
        .header("User-Agent", IOS_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("apps.apple.com"),
        "body should contain store URL"
    );
    assert!(
        body.contains("navigator.clipboard"),
        "body should contain clipboard JS"
    );

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

#[tokio::test]
async fn redirect_android_goes_to_play_store() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link?redirect=1"))
        .header("User-Agent", ANDROID_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 307);
    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(
        location.contains("play.google.com"),
        "location should contain Play Store"
    );
    assert!(
        location.contains("referrer="),
        "location should contain referrer"
    );
    assert!(
        location.contains("rift_link%3Dtest-link"),
        "location should contain encoded rift_link=test-link"
    );

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

#[tokio::test]
async fn redirect_desktop_goes_to_web_url() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link?redirect=1"))
        .header("User-Agent", DESKTOP_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 307);
    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, "https://example.com");

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

#[tokio::test]
async fn redirect_no_store_url_falls_back_to_landing() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create link with NO ios_store_url.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&json!({
            "custom_id": "no-ios-store",
            "web_url": "https://example.com",
            "android_store_url": "https://play.google.com/store/apps/details?id=com.example",
        }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url("/r/no-ios-store?redirect=1"))
        .header("User-Agent", IOS_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    // Should be the landing page, not a redirect page.
    assert!(
        body.contains("split"),
        "should fall back to landing page layout"
    );

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

// ── Landing page tests (no redirect) ──

#[tokio::test]
async fn landing_page_ios() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("User-Agent", IOS_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("split"), "should contain split layout");
    assert!(
        body.contains("apps.apple.com"),
        "should contain App Store link"
    );
    assert!(
        body.contains("navigator.clipboard"),
        "should contain clipboard JS"
    );
    assert!(
        body.contains("Machine-Readable Link"),
        "should contain agent panel"
    );
    assert!(
        body.contains("Verified by Rift"),
        "should contain trust section"
    );
    assert!(
        body.contains("Test app description"),
        "should contain agent_context description"
    );
    assert!(
        body.contains("application/ld+json"),
        "should contain JSON-LD"
    );

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

#[tokio::test]
async fn landing_page_android() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("User-Agent", ANDROID_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("split"), "should contain landing page layout");
    assert!(
        body.contains("play.google.com"),
        "should contain Play Store link"
    );
    assert!(
        body.contains("referrer"),
        "should contain referrer in store URL"
    );
    assert!(
        body.contains("Machine-Readable Link"),
        "should contain agent panel"
    );
    assert!(
        body.contains("Verified by Rift"),
        "should contain trust section"
    );
    assert!(
        body.contains("Test app description"),
        "should contain agent_context description"
    );
    assert!(
        body.contains("application/ld+json"),
        "should contain JSON-LD"
    );

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

#[tokio::test]
async fn landing_page_desktop() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("User-Agent", DESKTOP_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("split"), "should contain landing page layout");

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

// ── JSON (agent) tests ──

#[tokio::test]
async fn json_resolve_ignores_redirect() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link?redirect=1"))
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["agent_context"].is_object(),
        "should have agent_context"
    );
    assert!(body["_rift_meta"].is_object(), "should have _rift_meta");

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

#[tokio::test]
async fn json_resolve_normal() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "test-link");
    assert_eq!(body["web_url"], "https://example.com");
    assert_eq!(body["ios_store_url"], "https://apps.apple.com/app/id123");
    assert_eq!(
        body["android_store_url"],
        "https://play.google.com/store/apps/details?id=com.example"
    );
    assert!(body["agent_context"].is_object());
    assert!(body["_rift_meta"].is_object());

    let clicks = app.links_repo.clicks.lock().unwrap();
    assert_eq!(clicks.len(), 1, "click should be recorded");
}

// ── Edge case tests ──

#[tokio::test]
async fn redirect_expired_link() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    // Manually set the link as expired.
    {
        let mut links = app.links_repo.links.lock().unwrap();
        let link = links.iter_mut().find(|l| l.link_id == "test-link").unwrap();
        link.expires_at = Some(mongodb::bson::DateTime::from_millis(1000)); // far in the past
    }

    let resp = app
        .client
        .get(app.url("/r/test-link?redirect=1"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 410);
}

#[tokio::test]
async fn redirect_nonexistent_link() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/r/DOESNOTEXIST?redirect=1"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn landing_page_includes_json_ld() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("User-Agent", IOS_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("application/ld+json"),
        "should contain JSON-LD script tag"
    );
    assert!(
        body.contains("DownloadAction"),
        "should contain DownloadAction (action=download)"
    );
}

#[tokio::test]
async fn landing_page_includes_agent_panel() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("User-Agent", DESKTOP_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("Machine-Readable Link"),
        "should contain Machine-Readable Link badge"
    );
    assert!(
        body.contains("Verified by Rift"),
        "should contain Verified by Rift"
    );
    assert!(
        body.contains("Test app description"),
        "should contain agent_context description text"
    );
}

#[tokio::test]
async fn landing_page_includes_rift_meta_tag() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_test_link(&app, &key, &tenant_id).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("User-Agent", DESKTOP_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains(r#"<meta name="description"#),
        "should contain meta description tag"
    );
    assert!(
        body.contains("Test app description"),
        "meta description should contain agent description"
    );
}

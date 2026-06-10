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

    assert_eq!(resp.status(), 307);
    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(
        location.contains("apps.apple.com"),
        "location should contain App Store URL"
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
        body.contains("clipboard"),
        "should contain clipboard JS in button handler"
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

// ════════════════════════════════════════════════════════════════════════
// Platform-aware auto-redirect matrix (issue #195).
//
// This section is the executable spec for "how a link should work": every
// OS × redirect_mode × Sec-Fetch-User × visitor × config cell. The delivery
// model is: redirect_mode=Auto + a Tier-1 desktop target + Sec-Fetch-User: ?1
// → zero-flash 307; everything else → landing page (preserves unfurls +
// clipboard tap). Mobile always lands. macOS → Mac App Store is Tier-2 (lands).
// ════════════════════════════════════════════════════════════════════════

const MAC_PLATFORM: &str = "\"macOS\"";
const WIN_PLATFORM: &str = "\"Windows\"";
const LINUX_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const SLACKBOT_UA: &str = "Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots/)";
const ACTIVATION: &str = "?1";

/// Create a link with ALL destinations (mobile + desktop stores + web) and an
/// optional `redirect_mode`. Seeds the tenant's verified domain once.
async fn create_matrix_link(
    app: &common::TestApp,
    key: &str,
    tenant_id: &ObjectId,
    custom_id: &str,
    redirect_mode: Option<&str>,
) {
    seed_verified_domain(app, tenant_id, "go.example.com").await;
    let mut body = json!({
        "custom_id": custom_id,
        "web_url": "https://example.com",
        "ios_store_url": "https://apps.apple.com/app/id123",
        "android_store_url": "https://play.google.com/store/apps/details?id=com.example",
        "macos_store_url": "https://apps.apple.com/app/mac/id999",
        "windows_store_url": "https://apps.microsoft.com/detail/9NBLGGH4NNS1",
    });
    if let Some(m) = redirect_mode {
        body["redirect_mode"] = json!(m);
    }
    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "link create should succeed");
}

// ── Block A: Auto + human, the 307 fast path is desktop Tier-1 only ──

#[tokio::test]
async fn auto_windows_with_activation_307s_to_ms_store_with_cid() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link"))
        .header("Sec-CH-UA-Platform", WIN_PLATFORM)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 307, "Windows + activation → 307");
    let loc = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(
        loc.contains("apps.microsoft.com"),
        "→ Microsoft Store: {loc}"
    );
    assert!(
        loc.contains("cid=test-link"),
        "carries cid attribution: {loc}"
    );
    // Headers that keep a shared cache from cross-serving the 307.
    assert_eq!(
        resp.headers().get("cache-control").unwrap(),
        "no-store",
        "307 must not be cached"
    );
    assert!(resp
        .headers()
        .get("vary")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("Sec-Fetch-User"));
}

#[tokio::test]
async fn auto_other_desktop_with_activation_307s_to_web_with_rift_link() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link"))
        .header("User-Agent", LINUX_UA)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 307, "Linux/other desktop + activation → 307");
    let loc = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.starts_with("https://example.com"), "→ web_url: {loc}");
    assert!(
        loc.contains("rift_link=test-link"),
        "carries rift_link: {loc}"
    );
}

#[tokio::test]
async fn auto_macos_lands_even_with_activation() {
    // macOS → Mac App Store is Tier-2 (clipboard) → must land for the tap.
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("Sec-CH-UA-Platform", MAC_PLATFORM)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "macOS Tier-2 always lands");
    let body = resp.text().await.unwrap();
    assert!(body.contains("split"), "landing page");
    assert!(
        body.contains("apps.apple.com/app/mac"),
        "Mac App Store button"
    );
    // iPad-in-desktop-mode correction must be present and carry the iOS store.
    assert!(
        body.contains("maxTouchPoints"),
        "iPad touch-detection correction present"
    );
    assert!(
        body.contains("apps.apple.com/app/id123"),
        "iOS App Store URL available for iPad correction"
    );
}

#[tokio::test]
async fn auto_macos_with_ios_target_lands_even_without_mac_store() {
    // An iPad reports as Mac; when the link has an iOS target it must land so
    // the page can route iPad → iOS App Store (never 307 to web/Mac).
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&json!({
            "custom_id": "mac-ios",
            "web_url": "https://example.com",
            "ios_store_url": "https://apps.apple.com/app/id123",
        }))
        .send()
        .await
        .unwrap();

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/mac-ios"))
        .header("Sec-CH-UA-Platform", MAC_PLATFORM)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        200,
        "Mac request with an iOS target lands (iPad protection), never 307"
    );
}

#[tokio::test]
async fn auto_mobile_never_307s() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;

    let client = no_redirect_client();
    for ua in [IOS_UA, ANDROID_UA] {
        let resp = client
            .get(app.url("/r/test-link"))
            .header("User-Agent", ua)
            .header("Sec-Fetch-User", ACTIVATION)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "mobile always lands (ua={ua})");
    }
}

#[tokio::test]
async fn auto_desktop_without_activation_lands() {
    // No Sec-Fetch-User (old/privacy client) → fall through to landing page.
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link"))
        .header("Sec-CH-UA-Platform", WIN_PLATFORM)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "Windows without activation → landing");
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("apps.microsoft.com"),
        "MS Store button present"
    );
}

// ── Block B: Off mode never 307s, even with a Tier-1 target + activation ──

#[tokio::test]
async fn off_mode_never_307s() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", Some("off")).await;

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link"))
        .header("Sec-CH-UA-Platform", WIN_PLATFORM)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "redirect_mode=off never auto-redirects");
}

// ── Block C: visitor type ──

#[tokio::test]
async fn crawler_gets_og_landing_not_redirect() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;

    let client = no_redirect_client();
    // Crawler: a Windows-ish UA but NO Sec-Fetch-User (bots don't set it).
    let resp = client
        .get(app.url("/r/test-link"))
        .header("User-Agent", SLACKBOT_UA)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "crawler never gets a 307");
    let body = resp.text().await.unwrap();
    assert!(body.contains("og:title"), "OG tags present for unfurl");
}

#[tokio::test]
async fn agent_json_includes_desktop_store_fields() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;

    let resp = app
        .client
        .get(app.url("/r/test-link"))
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body["macos_store_url"], "https://apps.apple.com/app/mac/id999",
        "JSON exposes macos_store_url"
    );
    assert_eq!(
        body["windows_store_url"], "https://apps.microsoft.com/detail/9NBLGGH4NNS1",
        "JSON exposes windows_store_url"
    );
}

// ── Block D: explicit ?redirect=1 covers desktop targets (wins over mode) ──

#[tokio::test]
async fn explicit_redirect_desktop_targets() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    // Off mode — explicit redirect must still win.
    create_matrix_link(&app, &key, &tenant_id, "test-link", Some("off")).await;

    let client = no_redirect_client();

    let win = client
        .get(app.url("/r/test-link?redirect=1"))
        .header("Sec-CH-UA-Platform", WIN_PLATFORM)
        .send()
        .await
        .unwrap();
    assert_eq!(win.status(), 307);
    let loc = win.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.contains("apps.microsoft.com") && loc.contains("cid=test-link"));

    let mac = client
        .get(app.url("/r/test-link?redirect=1"))
        .header("Sec-CH-UA-Platform", MAC_PLATFORM)
        .send()
        .await
        .unwrap();
    assert_eq!(mac.status(), 307);
    assert!(mac
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("apps.apple.com/app/mac"));
}

// ── Block E: fallback chains ──

#[tokio::test]
async fn auto_windows_without_store_falls_through_to_web() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&json!({
            "custom_id": "win-noweb",
            "web_url": "https://example.com",
            "android_store_url": "https://play.google.com/store/apps/details?id=com.example",
        }))
        .send()
        .await
        .unwrap();

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/win-noweb"))
        .header("Sec-CH-UA-Platform", WIN_PLATFORM)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 307, "no MS Store → Tier-1 web fallthrough");
    let loc = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.starts_with("https://example.com") && loc.contains("rift_link="));
}

#[tokio::test]
async fn legacy_link_without_redirect_mode_never_307s() {
    // Existing rows predating the feature have no `redirect_mode` → resolves to
    // Off → unchanged behavior, even on a Tier-1 desktop target with activation.
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    create_matrix_link(&app, &key, &tenant_id, "test-link", None).await;
    // Simulate a legacy row: clear the stamped redirect_mode.
    {
        let mut links = app.links_repo.links.lock().unwrap();
        let link = links.iter_mut().find(|l| l.link_id == "test-link").unwrap();
        link.redirect_mode = None;
    }

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/test-link"))
        .header("Sec-CH-UA-Platform", WIN_PLATFORM)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "legacy link (None) stays on landing");
}

#[tokio::test]
async fn auto_macos_pure_web_link_307s_to_web() {
    // A pure web link (no Mac store, no iOS target) is the only macOS case that
    // 307s — there's no iPad-vs-Mac ambiguity to protect (iPad → web is fine).
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&json!({
            "custom_id": "mac-web",
            "web_url": "https://example.com",
            "android_store_url": "https://play.google.com/store/apps/details?id=com.example",
        }))
        .send()
        .await
        .unwrap();

    let client = no_redirect_client();
    let resp = client
        .get(app.url("/r/mac-web"))
        .header("Sec-CH-UA-Platform", MAC_PLATFORM)
        .header("Sec-Fetch-User", ACTIVATION)
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        307,
        "macOS → web (Tier-1) for a pure web link"
    );
    let loc = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.starts_with("https://example.com") && loc.contains("rift_link="));
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

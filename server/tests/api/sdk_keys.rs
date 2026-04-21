use rift::services::domains::repo::DomainsRepository;

use crate::common;

// ── SDK Key CRUD Tests ──

#[tokio::test]
async fn create_sdk_key_returns_key() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/auth/publishable-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "go.example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["key"].as_str().unwrap().starts_with("pk_live_"));
    assert_eq!(body["domain"], "go.example.com");
    assert!(body["id"].as_str().is_some());
    assert!(body["created_at"].as_str().is_some());
}

#[tokio::test]
async fn create_sdk_key_rejects_unverified_domain() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;

    // Create an unverified domain.
    app.domains_repo
        .create_domain(
            tenant_id,
            "unverified.example.com".to_string(),
            "tok".to_string(),
            rift::services::domains::models::DomainRole::Primary,
        )
        .await
        .unwrap();

    let resp = app
        .client
        .post(app.url("/v1/auth/publishable-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "unverified.example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "domain_not_verified");
}

#[tokio::test]
async fn list_sdk_keys_omits_full_key() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create an SDK key.
    app.client
        .post(app.url("/v1/auth/publishable-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "go.example.com" }))
        .send()
        .await
        .unwrap();

    // List keys.
    let resp = app
        .client
        .get(app.url("/v1/auth/publishable-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let keys = body["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 1);
    // Should have key_prefix (truncated) not the full key.
    assert!(keys[0]["key_prefix"].as_str().unwrap().ends_with("..."));
    assert!(keys[0].get("key").is_none());
}

#[tokio::test]
async fn revoke_sdk_key_returns_204() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create an SDK key.
    let resp = app
        .client
        .post(app.url("/v1/auth/publishable-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "go.example.com" }))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    let key_id = body["id"].as_str().unwrap();

    let resp = app
        .client
        .delete(app.url(&format!("/v1/auth/publishable-keys/{key_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);

    // Verify it no longer appears in list.
    let resp = app
        .client
        .get(app.url("/v1/auth/publishable-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["keys"].as_array().unwrap().is_empty());
}

// ── Attribution Endpoint Tests ──

#[tokio::test]
async fn attribution_click_with_valid_key() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create a link.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "attr-click",
            "ios_deep_link": "myapp://test",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    // Seed an SDK key.
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/click"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .header(
            "User-Agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
        )
        .json(&serde_json::json!({ "link_id": "attr-click" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "attr-click");
    assert_eq!(body["platform"], "ios");
    assert_eq!(body["ios_deep_link"], "myapp://test");
    assert_eq!(body["web_url"], "https://example.com");
}

#[tokio::test]
async fn attribution_click_without_key_returns_401() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/click"))
        .json(&serde_json::json!({ "link_id": "any" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn attribution_report_with_valid_key() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create a link.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "attr-report",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/install"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "link_id": "attr-report",
            "install_id": "test-install",
            "app_version": "1.0.0"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn attribution_report_without_key_returns_401() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/install"))
        .json(&serde_json::json!({
            "link_id": "any",
            "install_id": "test",
            "app_version": "1.0"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ── Attribution Link Tests ──
//
// PUT /v1/attribution/identify lives on the SDK-auth path (pk_live_) because
// install_id is opaque and only lives in the mobile SDK. These tests verify
// the auth move and preserve the existing behavior of the handler.

async fn report_attribution_for_test(
    app: &common::TestApp,
    sdk_key: &str,
    link_id: &str,
    install_id: &str,
) {
    let resp = app
        .client
        .post(app.url("/v1/attribution/install"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "link_id": link_id,
            "install_id": install_id,
            "app_version": "1.0.0"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn attribution_link_binds_user_with_sdk_key() {
    let app = common::spawn_app().await;
    let (api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create a link and report an attribution we can later bind.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "custom_id": "bind-me",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;
    report_attribution_for_test(&app, &sdk_key, "bind-me", "install-abc").await;

    // Bind the install to a user with the SDK key.
    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-abc",
            "user_id": "usr_123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn attribution_link_is_idempotent_with_sdk_key() {
    let app = common::spawn_app().await;
    let (api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "custom_id": "idem",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;
    report_attribution_for_test(&app, &sdk_key, "idem", "install-idem").await;

    // First bind.
    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-idem",
            "user_id": "usr_same"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Second bind with the same pair — still succeeds.
    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-idem",
            "user_id": "usr_same"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn attribution_link_rejects_rebind_to_different_user() {
    let app = common::spawn_app().await;
    let (api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "custom_id": "rebind",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;
    report_attribution_for_test(&app, &sdk_key, "rebind", "install-rebind").await;

    // First bind to user A.
    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-rebind",
            "user_id": "usr_a"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Attempt to rebind to user B — rejected as not found (the underlying
    // repo update filter doesn't match because user_id is already set).
    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-rebind",
            "user_id": "usr_b"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn attribution_link_returns_404_for_missing_install() {
    let app = common::spawn_app().await;
    let (_api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "does-not-exist",
            "user_id": "usr_whatever"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn attribution_link_rejects_secret_key() {
    // The route moved from auth_gate to sdk_auth_gate. Calling with a
    // secret key should now return 401 — sdk_auth_gate only accepts pk_live_.
    let app = common::spawn_app().await;
    let (api_key, _tenant_id) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "install_id": "any",
            "user_id": "usr_any"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn attribution_link_rejects_no_auth() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .put(app.url("/v1/attribution/identify"))
        .json(&serde_json::json!({
            "install_id": "any",
            "user_id": "usr_any"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ── SDK Conversion Endpoint Tests ──

/// Helper: set up a full attribution chain so conversion events can be attributed.
async fn setup_attribution_chain(
    app: &common::TestApp,
    api_key: &str,
    sdk_key: &str,
    link_id: &str,
    install_id: &str,
    user_id: &str,
) {
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "custom_id": link_id,
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    app.client
        .post(app.url("/v1/attribution/install"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "link_id": link_id,
            "install_id": install_id,
            "app_version": "1.0.0"
        }))
        .send()
        .await
        .unwrap();

    app.client
        .put(app.url("/v1/attribution/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": install_id,
            "user_id": user_id
        }))
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn sdk_convert_happy_path() {
    let app = common::spawn_app().await;
    let (api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    setup_attribution_chain(
        &app,
        &api_key,
        &sdk_key,
        "conv-test",
        "inst-conv",
        "usr-conv",
    )
    .await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "user_id": "usr-conv",
            "type": "spot_trade",
            "idempotency_key": "order-001"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["accepted"], 1);
    assert_eq!(body["deduped"], 0);
    assert_eq!(body["unattributed"], 0);
}

#[tokio::test]
async fn sdk_convert_dedupes_by_idempotency_key() {
    let app = common::spawn_app().await;
    let (api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    setup_attribution_chain(
        &app,
        &api_key,
        &sdk_key,
        "dedup-test",
        "inst-dedup",
        "usr-dedup",
    )
    .await;

    let payload = serde_json::json!({
        "user_id": "usr-dedup",
        "type": "swap",
        "idempotency_key": "tx-same"
    });

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["accepted"], 1);

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["deduped"], 1);
    assert_eq!(body["accepted"], 0);
}

#[tokio::test]
async fn sdk_convert_unattributed_user() {
    let app = common::spawn_app().await;
    let (_api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "user_id": "unknown-user",
            "type": "trade",
            "idempotency_key": "order-ghost"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["unattributed"], 1);
    assert_eq!(body["accepted"], 0);
}

#[tokio::test]
async fn sdk_convert_rejects_empty_user_id() {
    let app = common::spawn_app().await;
    let (_api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "user_id": "",
            "type": "trade"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn sdk_convert_rejects_empty_type() {
    let app = common::spawn_app().await;
    let (_api_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "user_id": "usr-test",
            "type": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn sdk_convert_rejects_no_auth() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .json(&serde_json::json!({
            "user_id": "usr-test",
            "type": "trade"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn sdk_convert_rejects_secret_key() {
    let app = common::spawn_app().await;
    let (api_key, _tenant_id) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/attribution/convert"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "user_id": "usr-test",
            "type": "trade"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

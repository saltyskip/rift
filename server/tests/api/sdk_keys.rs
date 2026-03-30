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
        .post(app.url("/v1/attribution/report"))
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
        .post(app.url("/v1/attribution/report"))
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

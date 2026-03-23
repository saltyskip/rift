use crate::common;

#[tokio::test]
async fn create_link_returns_201() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["link_id"].as_str().is_some());
    assert!(body["url"].as_str().unwrap().contains("/r/"));
}

#[tokio::test]
async fn create_link_with_custom_id() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "my-link",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "my-link");
}

#[tokio::test]
async fn create_link_with_platform_destinations() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "platform-test",
            "ios_deep_link": "myapp://product/123",
            "android_deep_link": "myapp://product/123",
            "web_url": "https://example.com/product/123",
            "ios_store_url": "https://apps.apple.com/app/id123",
            "android_store_url": "https://play.google.com/store/apps/details?id=com.example"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);

    // Verify all fields are returned via JSON resolve.
    let resp = app
        .client
        .get(app.url("/r/platform-test"))
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ios_deep_link"], "myapp://product/123");
    assert_eq!(body["android_deep_link"], "myapp://product/123");
    assert_eq!(body["web_url"], "https://example.com/product/123");
    assert_eq!(body["ios_store_url"], "https://apps.apple.com/app/id123");
}

#[tokio::test]
async fn duplicate_custom_id_returns_409() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let payload = serde_json::json!({
        "custom_id": "taken",
        "web_url": "https://example.com"
    });

    let resp1 = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), 201);

    let resp2 = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), 409);
    let body: serde_json::Value = resp2.json().await.unwrap();
    assert_eq!(body["code"], "link_id_taken");
}

#[tokio::test]
async fn list_links_returns_created_links() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Create two links.
    for url in ["https://a.com", "https://b.com"] {
        app.client
            .post(app.url("/v1/links"))
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({ "web_url": url }))
            .send()
            .await
            .unwrap();
    }

    let resp = app
        .client
        .get(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["links"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_link_stats() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "stats-test",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Resolve it to generate a click.
    app.client
        .get(app.url("/r/stats-test"))
        .send()
        .await
        .unwrap();

    // Get stats.
    let resp = app
        .client
        .get(app.url("/v1/links/stats-test/stats"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "stats-test");
    assert_eq!(body["click_count"], 1);
}

#[tokio::test]
async fn create_link_custom_id_without_domain_returns_400() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "vanity",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "no_verified_domain");
}

#[tokio::test]
async fn two_tenants_same_custom_id_succeeds() {
    let app = common::spawn_app().await;

    // Tenant A
    let (key_a, tenant_a) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_a, "go.tenant-a.com").await;

    // Tenant B
    let (key_b, tenant_b) = common::seed_api_key_with(
        &app,
        "rl_live_test_b_234567890abcdef1234567890abcdef12345678",
    )
    .await;
    common::seed_verified_domain(&app, &tenant_b, "go.tenant-b.com").await;

    // Both create a link with the same custom_id.
    let resp_a = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key_a}"))
        .json(&serde_json::json!({ "custom_id": "download", "web_url": "https://a.com" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_a.status(), 201);

    let resp_b = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key_b}"))
        .json(&serde_json::json!({ "custom_id": "download", "web_url": "https://b.com" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_b.status(), 201);
}

#[tokio::test]
async fn create_link_invalid_custom_id() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "ab",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_custom_id");
}

#[tokio::test]
async fn timeseries_returns_daily_clicks() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "ts-test",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    // Generate 3 clicks.
    for _ in 0..3 {
        app.client
            .get(app.url("/r/ts-test"))
            .header("Accept", "application/json")
            .send()
            .await
            .unwrap();
    }

    let resp = app
        .client
        .get(app.url("/v1/links/ts-test/timeseries"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "ts-test");
    assert_eq!(body["granularity"], "daily");

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["clicks"], 3);
}

#[tokio::test]
async fn timeseries_link_not_found_returns_404() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .get(app.url("/v1/links/nonexistent/timeseries"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn timeseries_invalid_granularity_returns_400() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "ts-gran",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url("/v1/links/ts-gran/timeseries?granularity=hourly"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_granularity");
}

#[tokio::test]
async fn timeseries_empty_returns_empty_data() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Create link with auto-generated ID (no custom domain needed).
    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "web_url": "https://example.com" }))
        .send()
        .await
        .unwrap();
    let link_id = resp.json::<serde_json::Value>().await.unwrap()["link_id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .client
        .get(app.url(&format!("/v1/links/{link_id}/timeseries")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].as_array().unwrap().is_empty());
}

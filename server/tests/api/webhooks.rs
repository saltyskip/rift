use crate::common;

#[tokio::test]
async fn create_webhook_returns_201() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "url": "https://example.com/webhook",
            "events": ["click", "attribution"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["url"], "https://example.com/webhook");
    assert!(body["secret"].as_str().is_some());
    assert_eq!(body["secret"].as_str().unwrap().len(), 64); // 32 bytes hex
    assert_eq!(body["events"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn create_webhook_rejects_http_url() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "url": "http://example.com/webhook",
            "events": ["click"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_url");
}

#[tokio::test]
async fn create_webhook_rejects_empty_events() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "url": "https://example.com/webhook",
            "events": []
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "empty_events");
}

#[tokio::test]
async fn create_webhook_rejects_invalid_url() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "url": "not-a-url",
            "events": ["click"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_url");
}

#[tokio::test]
async fn create_webhook_enforces_limit() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Create 2 webhooks (the max).
    for i in 0..2 {
        let resp = app
            .client
            .post(app.url("/v1/webhooks"))
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({
                "url": format!("https://example.com/webhook/{i}"),
                "events": ["click"]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201);
    }

    // Third should be rejected.
    let resp = app
        .client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "url": "https://example.com/webhook/3",
            "events": ["click"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "webhook_limit");
}

#[tokio::test]
async fn list_webhooks_omits_secret() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "url": "https://example.com/webhook",
            "events": ["click"]
        }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let webhooks = body["webhooks"].as_array().unwrap();
    assert_eq!(webhooks.len(), 1);
    assert!(webhooks[0].get("secret").is_none());
    assert_eq!(webhooks[0]["url"], "https://example.com/webhook");
}

#[tokio::test]
async fn delete_webhook_returns_204() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "url": "https://example.com/webhook",
            "events": ["click"]
        }))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    let webhook_id = body["id"].as_str().unwrap();

    let resp = app
        .client
        .delete(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);

    // Verify it's gone.
    let resp = app
        .client
        .get(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["webhooks"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn delete_nonexistent_webhook_returns_404() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let fake_id = mongodb::bson::oid::ObjectId::new().to_hex();
    let resp = app
        .client
        .delete(app.url(&format!("/v1/webhooks/{fake_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn click_dispatches_webhook() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create a link.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "webhook-click",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    // Resolve link (triggers click).
    app.client
        .get(app.url("/r/webhook-click"))
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    // Check the mock dispatcher captured the click.
    let clicks = app.webhook_dispatcher.click_payloads.lock().unwrap();
    assert_eq!(clicks.len(), 1);
    assert_eq!(clicks[0].link_id, "webhook-click");
    assert_eq!(clicks[0].tenant_id, tenant_id.to_hex());
}

#[tokio::test]
async fn attribution_dispatches_webhook() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create a link.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "webhook-attr",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    // Report attribution.
    app.client
        .post(app.url("/v1/attribution"))
        .json(&serde_json::json!({
            "link_id": "webhook-attr",
            "install_id": "install-123",
            "app_version": "1.0.0"
        }))
        .send()
        .await
        .unwrap();

    // Check the mock dispatcher captured the attribution.
    let attrs = app.webhook_dispatcher.attribution_payloads.lock().unwrap();
    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].link_id, "webhook-attr");
    assert_eq!(attrs[0].install_id, "install-123");
    assert_eq!(attrs[0].tenant_id, tenant_id.to_hex());
}

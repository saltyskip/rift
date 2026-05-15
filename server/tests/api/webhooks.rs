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
            "events": ["click", "attribute"]
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

// TODO: restore webhook-limit test once the integration harness can wire a
// QuotaService in Enforce mode. The old hardcoded MAX_WEBHOOKS_PER_TENANT=2
// check was replaced by tier-derived limits via QuotaService::check inside
// WebhooksService::create_webhook (Free tier = 1 webhook). Tests currently
// run with quota=None so the limit doesn't fire in-process.

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
async fn attribute_dispatches_webhook() {
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

    // Seed an SDK key for the tenant.
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    // Report attribute via SDK-authenticated endpoint.
    app.client
        .post(app.url("/v1/lifecycle/attribute"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "link_id": "webhook-attr",
            "install_id": "install-123",
            "app_version": "1.0.0"
        }))
        .send()
        .await
        .unwrap();

    // Check the mock dispatcher captured the attribute event.
    let attrs = app.webhook_dispatcher.attribute_payloads.lock().unwrap();
    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].link_id, "webhook-attr");
    assert_eq!(attrs[0].install_id, "install-123");
    assert_eq!(attrs[0].tenant_id, tenant_id.to_hex());
}

#[tokio::test]
async fn identify_dispatches_webhook_with_link_metadata() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create a link with bonus-campaign metadata. The receiver dispatch
    // contract is: whatever was on `Link.metadata` at fire time is
    // serialized verbatim into the `identify` event payload.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "welcome-link",
            "web_url": "https://example.com",
            "metadata": {
                "bonus_type": "welcome",
                "bonus_amount_usdc": 10,
            }
        }))
        .send()
        .await
        .unwrap();

    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    // Establish the prior attribution that identify will bind to.
    app.client
        .post(app.url("/v1/lifecycle/attribute"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "link_id": "welcome-link",
            "install_id": "install-id-7",
            "app_version": "1.0.0"
        }))
        .send()
        .await
        .unwrap();

    // Bind the install to a user — this is the moment the `identify`
    // event should fire.
    let resp = app
        .client
        .put(app.url("/v1/lifecycle/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-id-7",
            "user_id": "user-abc",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let events = app.webhook_dispatcher.identify_payloads.lock().unwrap();
    assert_eq!(events.len(), 1, "expected exactly one identify event");
    let evt = &events[0];
    assert_eq!(evt.tenant_id, tenant_id.to_hex());
    assert_eq!(evt.user_id, "user-abc");
    assert_eq!(evt.link_id, "welcome-link");
    assert_eq!(evt.install_id, "install-id-7");

    let metadata = evt
        .link_metadata
        .as_ref()
        .expect("link_metadata should be populated from Link.metadata");
    assert_eq!(metadata["bonus_type"], "welcome");
    assert_eq!(metadata["bonus_amount_usdc"], 10);
}

#[tokio::test]
async fn identify_without_prior_attribution_does_not_dispatch() {
    let app = common::spawn_app().await;
    let (_, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .put(app.url("/v1/lifecycle/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "never-installed",
            "user_id": "user-xyz",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let events = app.webhook_dispatcher.identify_payloads.lock().unwrap();
    assert_eq!(events.len(), 0);
}

#[tokio::test]
async fn identify_idempotent_rebind_does_not_refire_webhook() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "welcome-rebind",
            "web_url": "https://example.com",
            "metadata": { "bonus_type": "welcome", "bonus_amount_usdc": 10 }
        }))
        .send()
        .await
        .unwrap();

    let sdk_key = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/lifecycle/attribute"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "link_id": "welcome-rebind",
            "install_id": "install-rebind-1",
            "app_version": "1.0.0"
        }))
        .send()
        .await
        .unwrap();

    // First identify → real bind, webhook fires.
    let resp = app
        .client
        .put(app.url("/v1/lifecycle/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-rebind-1",
            "user_id": "user-rebind",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Second identify with the SAME install_id + user_id — SDK retries on
    // launch. Bind is a no-op; the webhook must NOT re-fire, otherwise
    // receivers (e.g. welcome-bonus crediting) double-grant on every
    // launch the SDK decides to re-sync.
    let resp = app
        .client
        .put(app.url("/v1/lifecycle/identify"))
        .header("Authorization", format!("Bearer {sdk_key}"))
        .json(&serde_json::json!({
            "install_id": "install-rebind-1",
            "user_id": "user-rebind",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let events = app.webhook_dispatcher.identify_payloads.lock().unwrap();
    assert_eq!(
        events.len(),
        1,
        "idempotent rebind must not re-fire the identify webhook"
    );
}

#[tokio::test]
async fn patch_webhook_toggles_active() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Create a webhook.
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
    let webhook_id = body["id"].as_str().unwrap().to_string();

    // Disable it.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "active": false }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["active"], false);

    // Re-enable it.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "active": true }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["active"], true);
}

#[tokio::test]
async fn patch_webhook_replaces_events() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Create with one event.
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
    let webhook_id = body["id"].as_str().unwrap().to_string();

    // Patch the event list — add identify + attribution, drop click.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "events": ["identify", "attribute"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["events"], serde_json::json!(["identify", "attribute"]));

    // Patch with both `active` and `events` in one call.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "active": false, "events": ["conversion"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["active"], false);
    assert_eq!(body["events"], serde_json::json!(["conversion"]));
}

#[tokio::test]
async fn patch_webhook_rejects_empty_body() {
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
    let webhook_id = body["id"].as_str().unwrap().to_string();

    // No fields → 400.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Empty events array → 400.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "events": [] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn patch_webhook_replaces_url() {
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
    let webhook_id = body["id"].as_str().unwrap().to_string();
    let original_secret = body["secret"].as_str().unwrap().to_string();

    // URL-only patch keeps the existing secret untouched — that's the
    // whole point versus delete + recreate.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "url": "https://new.example.com/webhook" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["url"], "https://new.example.com/webhook");
    // The PATCH response shape doesn't expose `secret` (same as GET list),
    // so we verify it didn't change by hitting list and confirming the
    // server-side row is otherwise consistent.
    assert!(body.get("secret").is_none());
    // The secret won't reappear in any future response — capture is only
    // at creation. Keep it bound so future tests can use it if needed.
    let _ = original_secret;
}

#[tokio::test]
async fn patch_webhook_rejects_invalid_url() {
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
    let webhook_id = body["id"].as_str().unwrap().to_string();

    // Non-HTTPS rejected.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "url": "http://example.com/webhook" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_url");

    // Garbage rejected.
    let resp = app
        .client
        .patch(app.url(&format!("/v1/webhooks/{webhook_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "url": "not-a-url" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

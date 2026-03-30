use crate::common;

#[tokio::test]
async fn signup_rejects_invalid_email() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .post(app.url("/v1/auth/signup"))
        .json(&serde_json::json!({ "email": "bad" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_email");
}

#[tokio::test]
async fn signup_rejects_short_email() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .post(app.url("/v1/auth/signup"))
        .json(&serde_json::json!({ "email": "a@b" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn verify_rejects_invalid_token() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/v1/auth/verify?token=bogus"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_token");
}

#[tokio::test]
async fn anonymous_requests_are_rate_limited() {
    let app = common::spawn_app().await;
    let (_key, _) = common::seed_api_key(&app).await;

    // Create a link so resolve succeeds (which records usage through the middleware
    // on the authenticated link-creation side, but clicks are public).
    // Instead, we'll seed a link directly and test the protected endpoint.
    // Since anonymous requests that fail at the handler (500) don't record usage,
    // we need to test with an endpoint that actually succeeds anonymously through auth_gate.
    //
    // Strategy: create a link via API key, then hit the protected GET /v1/links endpoint
    // anonymously. The anonymous path passes auth_gate but fails at the handler (no TenantId).
    // Usage is only recorded on success, so the rate limit won't trigger.
    //
    // Better approach: verify the rate limit response directly by pre-filling usage records.
    use rift::services::auth::secret_keys::repo::{AuthRepository, UsageDoc};

    // Pre-fill 5 anonymous usage records for the IP "unknown" (default when no ConnectInfo).
    for _ in 0..5 {
        app.auth_repo
            .record_usage(UsageDoc {
                id: None,
                api_key_id: None,
                ip: "unknown".to_string(),
                endpoint: "/v1/links".to_string(),
                ts: mongodb::bson::DateTime::now(),
            })
            .await;
    }

    // The 6th anonymous request should be rate limited.
    let resp = app.client.get(app.url("/v1/links")).send().await.unwrap();

    assert_eq!(resp.status(), 429);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "rate_limited");
}

#[tokio::test]
async fn api_key_auth_injects_tenant_id() {
    let app = common::spawn_app().await;
    let (key, _tenant_id) = common::seed_api_key(&app).await;

    // Authenticated request to list links — should return 200 with empty list.
    let resp = app
        .client
        .get(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["links"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn invalid_api_key_returns_401() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/v1/links"))
        .header("Authorization", "Bearer rl_live_doesnotexist")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_key");
}

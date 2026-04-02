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

    // Pre-fill 5 anonymous usage records for the IP "unknown" (default when no ConnectInfo).
    use rift::services::auth::usage::repo::{UsageDoc, UsageRepository};

    for _ in 0..5 {
        app.usage_repo
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

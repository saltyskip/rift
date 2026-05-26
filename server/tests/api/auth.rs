use crate::common;

// `/v1/auth/signup` is gone — the signin flow at `/v1/auth/signin` is the
// single front door now. Note: the test harness wires `sessions_service: None`
// (no real Mongo behind it), so signin returns 503. These tests assert that
// the OLD endpoint is gone (404) and that signin's wiring is at least live.
//
// Full signin happy-path coverage lives in the integration test that boots
// against a real MongoDB; mock-mongo here doesn't support the session
// repo's atomic ops cleanly.

#[tokio::test]
async fn old_signup_endpoint_is_gone() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .post(app.url("/v1/auth/signup"))
        .json(&serde_json::json!({ "email": "anyone@example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn signin_endpoint_is_wired() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .post(app.url("/v1/auth/signin"))
        .json(&serde_json::json!({ "email": "anyone@example.com" }))
        .send()
        .await
        .unwrap();

    // Sessions service is `None` in the mock harness → 503. The point of
    // this test is that the route exists (not a 404) and is reachable.
    assert!(
        resp.status() == 503 || resp.status() == 200,
        "expected 503 (no sessions service) or 200 (live), got {}",
        resp.status()
    );
}

#[tokio::test]
async fn verify_get_renders_interstitial() {
    // `GET /v1/auth/verify` is now a stateless interstitial — it never
    // consumes the token. The token survives email-scanner prefetch
    // (Avanan, Defender Safe Links, etc.) so the actual user click still
    // works. Validation happens on POST.
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/v1/auth/verify?token=bogus"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.starts_with("text/html"),
        "expected HTML interstitial, got content-type={content_type}"
    );
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("<form") && body.contains("method=\"post\""),
        "interstitial body should contain a POST form"
    );
}

#[tokio::test]
async fn verify_post_with_invalid_token_redirects_to_signin_error() {
    // `POST /v1/auth/verify` is where consumption happens. A bogus token
    // redirects to `${marketing_url}/signin?error=invite_invalid` (303).
    // Build a no-redirect client inline so we can observe the 303.
    let app = common::spawn_app().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let resp = client
        .post(app.url("/v1/auth/verify"))
        .form(&[("token", "bogus")])
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 303);
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        location.contains("/signin?error=invite_invalid"),
        "expected redirect to invite_invalid, got {location}"
    );
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

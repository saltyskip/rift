use crate::common;

#[tokio::test]
async fn resolve_redirects_to_destination() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Create link with destination.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "redir-test",
            "destination": "https://example.com/target"
        }))
        .send()
        .await
        .unwrap();

    // Resolve — don't follow redirects.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let resp = client.get(app.url("/r/redir-test")).send().await.unwrap();

    assert_eq!(resp.status(), 307);
    assert_eq!(
        resp.headers().get("location").unwrap().to_str().unwrap(),
        "https://example.com/target"
    );
}

#[tokio::test]
async fn resolve_returns_json_for_agents() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "json-test",
            "destination": "https://example.com",
            "metadata": { "campaign": "summer" }
        }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url("/r/json-test"))
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "json-test");
    assert_eq!(body["destination"], "https://example.com");
    assert_eq!(body["metadata"]["campaign"], "summer");
}

#[tokio::test]
async fn resolve_missing_link_returns_404() {
    let app = common::spawn_app().await;

    let resp = app.client.get(app.url("/r/NONEXIST")).send().await.unwrap();

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn resolve_increments_click_count() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "click-count",
            "destination": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    // Resolve 3 times.
    for _ in 0..3 {
        app.client
            .get(app.url("/r/click-count"))
            .header("Accept", "application/json")
            .send()
            .await
            .unwrap();
    }

    // Check stats.
    let resp = app
        .client
        .get(app.url("/v1/links/click-count/stats"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["click_count"], 3);
}

#[tokio::test]
async fn resolve_no_destination_shows_landing() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "custom_id": "no-dest" }))
        .send()
        .await
        .unwrap();

    let resp = app.client.get(app.url("/r/no-dest")).send().await.unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("no-dest"));
    assert!(body.contains("No destination configured"));
}

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
            "destination": "https://example.com"
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
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "my-link",
            "destination": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "my-link");
}

#[tokio::test]
async fn duplicate_custom_id_returns_409() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let payload = serde_json::json!({
        "custom_id": "taken",
        "destination": "https://example.com"
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
    for dest in ["https://a.com", "https://b.com"] {
        app.client
            .post(app.url("/v1/links"))
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({ "destination": dest }))
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
    let (key, _) = common::seed_api_key(&app).await;

    // Create a link.
    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "stats-test",
            "destination": "https://example.com"
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
async fn create_link_invalid_custom_id() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "ab",
            "destination": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_custom_id");
}

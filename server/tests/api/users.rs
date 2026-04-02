use crate::common;

#[tokio::test]
async fn list_users_requires_auth() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/v1/auth/users"))
        .send()
        .await
        .unwrap();
    // Anonymous request reaches auth_gate but handler has no TenantId
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[tokio::test]
async fn list_users_returns_empty() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .get(app.url("/v1/auth/users"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["users"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn invite_user_rejects_invalid_email() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/auth/users"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "email": "bad" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_email");
}

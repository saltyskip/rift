use crate::common;

#[tokio::test]
async fn list_secret_keys_requires_auth() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/v1/auth/secret-keys"))
        .send()
        .await
        .unwrap();

    // Anonymous request reaches auth_gate but handler has no TenantId
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[tokio::test]
async fn list_secret_keys_returns_keys_for_v2() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key_v2(&app).await;

    let resp = app
        .client
        .get(app.url("/v1/auth/secret-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["keys"].as_array().unwrap().len(), 1);
    // Should show prefix, not full key
    let first = &body["keys"][0];
    assert!(first["key_prefix"].as_str().unwrap().ends_with("..."));
    assert!(first["id"].as_str().is_some());
    assert!(first["created_at"].as_str().is_some());
}

#[tokio::test]
async fn delete_last_secret_key_returns_409() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key_v2(&app).await;

    // List to get the key ID
    let resp = app
        .client
        .get(app.url("/v1/auth/secret-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    let key_id = body["keys"][0]["id"].as_str().unwrap();

    // Try to delete the only key
    let resp = app
        .client
        .delete(app.url(&format!("/v1/auth/secret-keys/{key_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
    let body: serde_json::Value = resp.json().await.unwrap();
    // self_delete fires first since you're authed with this key
    assert_eq!(body["code"], "self_delete");
}

#[tokio::test]
async fn request_create_key_rejects_non_member() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key_v2(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/auth/secret-keys"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "email": "nobody@example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "not_a_member");
}

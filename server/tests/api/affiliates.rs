use crate::common;

#[tokio::test]
async fn create_affiliate_returns_201() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["name"], "Bcom");
    assert_eq!(body["partner_key"], "bcom");
    assert_eq!(body["status"], "active");
    assert!(body.get("signing_secret").is_none());
    assert!(body.get("postback_url").is_none());
}

#[tokio::test]
async fn list_affiliates_returns_created() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let list = body["affiliates"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["partner_key"], "bcom");
}

#[tokio::test]
async fn get_affiliate_returns_detail() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .client
        .get(app.url(&format!("/v1/affiliates/{id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], id);
    assert_eq!(body["partner_key"], "bcom");
}

#[tokio::test]
async fn create_rejects_invalid_partner_key() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    for bad in ["", "a", "Bcom", "-bcom", "bcom-", "bc om"] {
        let resp = app
            .client
            .post(app.url("/v1/affiliates"))
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({ "name": "X", "partner_key": bad }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 400, "expected 400 for partner_key {bad:?}");
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["code"], "invalid_partner_key");
    }
}

#[tokio::test]
async fn create_rejects_duplicate_partner_key() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom 2", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "partner_key_taken");
}

#[tokio::test]
async fn patch_updates_name_and_status() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .client
        .patch(app.url(&format!("/v1/affiliates/{id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom Italy", "status": "disabled" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Bcom Italy");
    assert_eq!(body["status"], "disabled");
}

#[tokio::test]
async fn patch_empty_body_returns_400() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .client
        .patch(app.url(&format!("/v1/affiliates/{id}")))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "empty_update");
}

#[tokio::test]
async fn delete_returns_204_then_404() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .client
        .delete(app.url(&format!("/v1/affiliates/{id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = app
        .client
        .delete(app.url(&format!("/v1/affiliates/{id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn cross_tenant_isolation() {
    let app = common::spawn_app().await;
    let (key_a, _) =
        common::seed_api_key_with(&app, "rl_live_aaaaaaaa00000000000000000000000000000000").await;
    let (key_b, _) =
        common::seed_api_key_with(&app, "rl_live_bbbbbbbb00000000000000000000000000000000").await;

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key_a}"))
        .json(&serde_json::json!({ "name": "A's partner", "partner_key": "ap" }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Tenant B can't see A's affiliate
    let resp = app
        .client
        .get(app.url(&format!("/v1/affiliates/{id}")))
        .header("Authorization", format!("Bearer {key_b}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // Tenant B's list should be empty
    let resp = app
        .client
        .get(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key_b}"))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["affiliates"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn scoped_key_cannot_create_affiliate() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;

    // Create an affiliate so we have one to scope a credential to.
    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let aff_oid = mongodb::bson::oid::ObjectId::parse_str(&id).unwrap();

    // Seed a partner-scoped credential and try to create another affiliate.
    let partner_key = common::seed_affiliate_scoped_key(
        &app,
        tenant_id,
        aff_oid,
        "rl_live_partnerkey00000000000000000000000000",
    )
    .await;

    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {partner_key}"))
        .json(&serde_json::json!({ "name": "B", "partner_key": "b" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "forbidden_scope");
}

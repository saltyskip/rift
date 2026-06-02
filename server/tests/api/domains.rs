use crate::common;

#[tokio::test]
async fn create_domain_returns_201() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "go.example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["domain"], "go.example.com");
    assert_eq!(body["verified"], false);
    assert!(body["verification_token"].as_str().is_some());
    assert!(body["txt_record"]
        .as_str()
        .unwrap()
        .contains("_rift-verify"));
}

#[tokio::test]
async fn create_domain_rejects_invalid() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "not valid!" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn create_domain_rejects_primary_domain() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "riftl.ink" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn list_domains_returns_created() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "links.foo.com" }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let domains = body["domains"].as_array().unwrap();
    assert_eq!(domains.len(), 1);
    assert_eq!(domains[0]["domain"], "links.foo.com");
}

#[tokio::test]
async fn list_domains_accepts_publishable_key() {
    // The mobile SDK reads the tenant's domains with a pk_live_ key to validate
    // deferred-deep-link clipboard hosts.
    let app = common::spawn_app().await;
    let (_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let pk = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .get(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {pk}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let domains = body["domains"].as_array().unwrap();
    assert!(domains.iter().any(|d| d["domain"] == "go.example.com"));
}

#[tokio::test]
async fn create_domain_rejects_publishable_key() {
    // Mutating routes stay secret-key only — a pk_live_ key must not create.
    let app = common::spawn_app().await;
    let (_key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    let pk = common::seed_sdk_key(&app, &tenant_id, "go.example.com").await;

    let resp = app
        .client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {pk}"))
        .json(&serde_json::json!({ "domain": "links.foo.com" }))
        .send()
        .await
        .unwrap();

    // A pk_live_ key is not a valid secret credential for `auth_gate`, so the
    // create must not succeed (the mutating routes never reach the dual gate).
    assert!(!resp.status().is_success());
}

#[tokio::test]
async fn delete_domain_returns_204() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    app.client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "domain": "del.example.com" }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .delete(app.url("/v1/domains/del.example.com"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);

    // Verify it's gone.
    let resp = app
        .client
        .get(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["domains"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn delete_nonexistent_domain_returns_404() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .delete(app.url("/v1/domains/nope.example.com"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn duplicate_domain_returns_409() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let payload = serde_json::json!({ "domain": "dup.example.com" });

    let resp1 = app
        .client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), 201);

    let resp2 = app
        .client
        .post(app.url("/v1/domains"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), 409);
}

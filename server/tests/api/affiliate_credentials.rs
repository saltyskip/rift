use crate::common;

async fn create_affiliate(app: &common::TestApp, key: &str) -> String {
    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Bcom", "partner_key": "bcom" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn mint_credential_returns_raw_key_once() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;

    let resp = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let api_key = body["api_key"].as_str().unwrap();
    assert!(api_key.starts_with("rl_live_"));
    assert_eq!(body["affiliate_id"], aff);
    assert!(body["key_prefix"].as_str().unwrap().starts_with("rl_live_"));
}

#[tokio::test]
async fn mint_credential_with_scoped_caller_is_forbidden() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;
    let aff_oid = mongodb::bson::oid::ObjectId::parse_str(&aff).unwrap();

    let partner_key = common::seed_affiliate_scoped_key(
        &app,
        tenant_id,
        aff_oid,
        "rl_live_partnerc1000000000000000000000000000000",
    )
    .await;

    // Affiliate-scoped key tries to mint another credential for itself.
    let resp = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {partner_key}"))
        .send()
        .await
        .unwrap();

    // Middleware allowlist 403s before we even reach the service —
    // POST /v1/affiliates/* is not on the allowlist.
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "forbidden_scope");
}

#[tokio::test]
async fn list_credentials_omits_raw_key() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;

    app.client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let creds = body["credentials"].as_array().unwrap();
    assert_eq!(creds.len(), 1);
    assert!(creds[0].get("api_key").is_none());
    assert!(creds[0]["key_prefix"]
        .as_str()
        .unwrap()
        .starts_with("rl_live_"));
}

#[tokio::test]
async fn revoke_credential_blocks_subsequent_auth() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;

    let mint_resp = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let mint_body: serde_json::Value = mint_resp.json().await.unwrap();
    let cred_id = mint_body["id"].as_str().unwrap().to_string();
    let partner_key = mint_body["api_key"].as_str().unwrap().to_string();

    // Partner key works while not revoked: scoped key should pass middleware
    // for POST /v1/links (the allowlisted path).
    let pre = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {partner_key}"))
        .json(&serde_json::json!({ "web_url": "https://example.com" }))
        .send()
        .await
        .unwrap();
    assert_eq!(pre.status(), 201);

    // Revoke.
    let resp = app
        .client
        .delete(app.url(&format!("/v1/affiliates/{aff}/credentials/{cred_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Now the same call should 401 — key has been deleted.
    let post = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {partner_key}"))
        .json(&serde_json::json!({ "web_url": "https://example.com" }))
        .send()
        .await
        .unwrap();
    assert_eq!(post.status(), 401);
}

#[tokio::test]
async fn mint_credential_caps_at_three_per_affiliate() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;

    // First three mints succeed.
    for i in 0..3 {
        let resp = app
            .client
            .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "mint {i} should succeed");
    }

    // Fourth mint hits the cap.
    let resp = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "credential_limit");

    // After revoking one, the next mint succeeds.
    let creds = app
        .client
        .get(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = creds.json().await.unwrap();
    let cred_id = body["credentials"][0]["id"].as_str().unwrap().to_string();

    let resp = app
        .client
        .delete(app.url(&format!("/v1/affiliates/{aff}/credentials/{cred_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
}

#[tokio::test]
async fn revoke_other_affiliates_credential_404s() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Two affiliates.
    let aff_a = create_affiliate(&app, &key).await;
    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Other", "partner_key": "other" }))
        .send()
        .await
        .unwrap();
    let aff_b = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Credential for A.
    let mint = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff_a}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let cred_id = mint.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Try to revoke A's cred via B's path.
    let resp = app
        .client
        .delete(app.url(&format!("/v1/affiliates/{aff_b}/credentials/{cred_id}")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

// ── Scoped link minting tests ──

#[tokio::test]
async fn scoped_key_creates_link_pinned_to_affiliate() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;
    let aff_oid = mongodb::bson::oid::ObjectId::parse_str(&aff).unwrap();

    let mint = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let partner_key = mint.json::<serde_json::Value>().await.unwrap()["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    // Partner mints a link without specifying affiliate_id — server pins it.
    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {partner_key}"))
        .json(&serde_json::json!({
            "web_url": "https://example.com",
            "metadata": { "partner_user_id": "usr_001" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let link_id = body["link_id"].as_str().unwrap();

    // Read it back and confirm affiliate_id was stamped.
    let stored = app
        .links_repo
        .links
        .lock()
        .unwrap()
        .iter()
        .find(|l| l.link_id == link_id)
        .cloned()
        .unwrap();
    assert_eq!(stored.affiliate_id, Some(aff_oid));
}

#[tokio::test]
async fn scoped_key_with_mismatched_affiliate_id_400s() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;

    let mint = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let partner_key = mint.json::<serde_json::Value>().await.unwrap()["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    // Some other (random) ObjectId.
    let other = mongodb::bson::oid::ObjectId::new().to_hex();

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {partner_key}"))
        .json(&serde_json::json!({
            "web_url": "https://example.com",
            "affiliate_id": other
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "affiliate_scope_mismatch");
}

#[tokio::test]
async fn full_scope_with_unknown_affiliate_id_404s() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let bogus = mongodb::bson::oid::ObjectId::new().to_hex();

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "web_url": "https://example.com",
            "affiliate_id": bogus
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "affiliate_not_found");
}

#[tokio::test]
async fn full_scope_with_known_affiliate_id_succeeds() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;
    let aff_oid = mongodb::bson::oid::ObjectId::parse_str(&aff).unwrap();

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "web_url": "https://example.com",
            "affiliate_id": aff
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = resp.json().await.unwrap();
    let link_id = body["link_id"].as_str().unwrap();
    let stored = app
        .links_repo
        .links
        .lock()
        .unwrap()
        .iter()
        .find(|l| l.link_id == link_id)
        .cloned()
        .unwrap();
    assert_eq!(stored.affiliate_id, Some(aff_oid));
}

// ── Middleware allowlist ──

#[tokio::test]
async fn scoped_key_blocked_from_non_allowlisted_path() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;

    let mint = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let partner_key = mint.json::<serde_json::Value>().await.unwrap()["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    // POST /v1/webhooks is NOT allowlisted for affiliate scope.
    let resp = app
        .client
        .post(app.url("/v1/webhooks"))
        .header("Authorization", format!("Bearer {partner_key}"))
        .json(&serde_json::json!({
            "url": "https://example.com/h",
            "events": ["click"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "forbidden_scope");
}

#[tokio::test]
async fn scoped_key_can_get_own_link_but_not_other_affiliates() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    // Two affiliates: A and B.
    let aff_a = create_affiliate(&app, &key).await;
    let resp = app
        .client
        .post(app.url("/v1/affiliates"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "name": "Other", "partner_key": "other" }))
        .send()
        .await
        .unwrap();
    let aff_b = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // A scoped credential for A.
    let mint = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff_a}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let partner_key_a = mint.json::<serde_json::Value>().await.unwrap()["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    // Advertiser mints links: one for A, one for B.
    let resp_a = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "web_url": "https://a.example.com", "affiliate_id": aff_a }))
        .send()
        .await
        .unwrap();
    let link_a = resp_a.json::<serde_json::Value>().await.unwrap()["link_id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp_b = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "web_url": "https://b.example.com", "affiliate_id": aff_b }))
        .send()
        .await
        .unwrap();
    let link_b = resp_b.json::<serde_json::Value>().await.unwrap()["link_id"]
        .as_str()
        .unwrap()
        .to_string();

    // A's credential reads its own link → 200.
    let resp = app
        .client
        .get(app.url(&format!("/v1/links/{link_a}")))
        .header("Authorization", format!("Bearer {partner_key_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // A's credential tries to read B's link → 404 (existence not disclosed).
    let resp = app
        .client
        .get(app.url(&format!("/v1/links/{link_b}")))
        .header("Authorization", format!("Bearer {partner_key_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn scoped_key_blocked_from_list_links() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;
    let aff = create_affiliate(&app, &key).await;

    let mint = app
        .client
        .post(app.url(&format!("/v1/affiliates/{aff}/credentials")))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    let partner_key = mint.json::<serde_json::Value>().await.unwrap()["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    // GET /v1/links (collection) is NOT allowlisted for affiliate scope —
    // only GET /v1/links/{id} is.
    let resp = app
        .client
        .get(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {partner_key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "forbidden_scope");
}

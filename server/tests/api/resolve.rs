use rift::services::domains::repo::DomainsRepository;

use crate::common;

#[tokio::test]
async fn resolve_redirects_to_web_url() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Create link with web_url only (no platform destinations -> redirect).
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "redir-test",
            "web_url": "https://example.com/target"
        }))
        .send()
        .await
        .unwrap();

    // Resolve -- don't follow redirects.
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
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "json-test",
            "ios_deep_link": "myapp://home",
            "web_url": "https://example.com",
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
    assert_eq!(body["ios_deep_link"], "myapp://home");
    assert_eq!(body["web_url"], "https://example.com");
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
async fn resolve_no_destination_shows_landing() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

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

#[tokio::test]
async fn resolve_ios_deep_link_shows_smart_landing() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "ios-link",
            "ios_deep_link": "myapp://product/42",
            "ios_store_url": "https://apps.apple.com/app/id999",
            "web_url": "https://example.com/product/42"
        }))
        .send()
        .await
        .unwrap();

    // Request with iPhone user-agent.
    let resp = app
        .client
        .get(app.url("/r/ios-link"))
        .header(
            "User-Agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    // Landing page shows store button (deep link is not used for navigation —
    // Universal Links handle that at the OS level before the page loads).
    assert!(body.contains("apps.apple.com"));
    // Deep link data is still in JSON-LD for agents/crawlers.
    assert!(body.contains("myapp://product/42"));
    // The Default template drives the palette through CSS custom properties.
    assert!(body.contains(":root"));
    assert!(body.contains("--accent:"));
}

#[tokio::test]
async fn resolve_landing_keeps_og_image_meta_without_visible_hero() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "promo",
            "ios_deep_link": "myapp://promo",
            "ios_store_url": "https://apps.apple.com/app/id999",
            "social_preview": {
                "title": "Summer Sale",
                "description": "50% off everything",
                "image_url": "https://cdn.example.com/promo-banner.jpg"
            }
        }))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(app.url("/r/promo"))
        .header(
            "User-Agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    // The OG image still drives social unfurls via meta tags...
    assert!(body.contains(r#"property="og:image""#));
    assert!(body.contains("https://cdn.example.com/promo-banner.jpg"));
    // ...but it is NOT rendered as a visible hero (OG images are small and
    // look poor stretched as a banner — see the logo/brand-led layout).
    assert!(!body.contains(r#"class="hero""#));
}

#[tokio::test]
async fn resolve_custom_domain_succeeds() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "download",
            "web_url": "https://example.com/download"
        }))
        .send()
        .await
        .unwrap();

    // Resolve via custom domain route (/{link_id} with x-rift-host header).
    let resp = app
        .client
        .get(app.url("/download"))
        .header("x-rift-host", "go.example.com")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["link_id"], "download");
    assert_eq!(body["web_url"], "https://example.com/download");
}

#[tokio::test]
async fn resolve_custom_domain_wrong_tenant_returns_404() {
    let app = common::spawn_app().await;

    // Tenant A owns "download"
    let (key_a, tenant_a) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_a, "go.tenant-a.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key_a}"))
        .json(&serde_json::json!({
            "custom_id": "download",
            "web_url": "https://a.com/download"
        }))
        .send()
        .await
        .unwrap();

    // Tenant B has a different domain but no "download" link
    let (_, tenant_b) = common::seed_api_key_with(
        &app,
        "rl_live_test_b_234567890abcdef1234567890abcdef12345678",
    )
    .await;
    common::seed_verified_domain(&app, &tenant_b, "go.tenant-b.com").await;

    // Resolve "download" via tenant B's domain -- should 404.
    let resp = app
        .client
        .get(app.url("/download"))
        .header("x-rift-host", "go.tenant-b.com")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn resolve_custom_domain_unverified_returns_404() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;

    // Create an unverified domain (seed without marking verified).
    app.domains_repo
        .create_domain(
            rift::core::public_id::TenantId::from_object_id(tenant_id),
            "unverified.example.com".to_string(),
            "tok".to_string(),
            rift::services::domains::models::DomainRole::Primary,
        )
        .await
        .unwrap();

    // Also need a verified domain to create the custom_id link.
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "download",
            "web_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();

    // Resolve via unverified domain -- should 404.
    let resp = app
        .client
        .get(app.url("/download"))
        .header("x-rift-host", "unverified.example.com")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn resolve_two_tenants_same_slug_via_custom_domains() {
    let app = common::spawn_app().await;

    let (key_a, tenant_a) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_a, "go.tenant-a.com").await;

    let (key_b, tenant_b) = common::seed_api_key_with(
        &app,
        "rl_live_test_b_234567890abcdef1234567890abcdef12345678",
    )
    .await;
    common::seed_verified_domain(&app, &tenant_b, "go.tenant-b.com").await;

    // Both create "download" pointing to different destinations.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key_a}"))
        .json(&serde_json::json!({ "custom_id": "download", "web_url": "https://a.com" }))
        .send()
        .await
        .unwrap();

    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key_b}"))
        .json(&serde_json::json!({ "custom_id": "download", "web_url": "https://b.com" }))
        .send()
        .await
        .unwrap();

    // Tenant A's domain resolves to A's link.
    let resp = app
        .client
        .get(app.url("/download"))
        .header("x-rift-host", "go.tenant-a.com")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["web_url"], "https://a.com");

    // Tenant B's domain resolves to B's link.
    let resp = app
        .client
        .get(app.url("/download"))
        .header("x-rift-host", "go.tenant-b.com")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["web_url"], "https://b.com");
}

#[tokio::test]
async fn serve_rift_js() {
    let app = common::spawn_app().await;

    let resp = app
        .client
        .get(app.url("/sdk/rift.js"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("javascript"));
    let body = resp.text().await.unwrap();
    assert!(body.contains("Rift"));
}

use crate::common;

/// PUT branding, then a resolved link for that tenant renders with the brand.
#[tokio::test]
async fn put_branding_themes_tenant_landing_pages() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    // Set OrangeRock branding.
    let put = app
        .client
        .put(app.url("/v1/tenant/branding"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "theme_color": "#ff7a1a",
            "color_scheme": "dark",
            "brand_name": "OrangeRock",
            "tagline": "Trade without limits"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(put.status(), 200);

    // Create a link for this tenant.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "trade",
            "ios_deep_link": "orangerock://trade",
            "ios_store_url": "https://apps.apple.com/app/id999"
        }))
        .send()
        .await
        .unwrap();

    // Resolve it — the landing page should carry the tenant's brand.
    let resp = app
        .client
        .get(app.url("/r/trade"))
        .header(
            "User-Agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    // brand_name only appears when the tenant theme is loaded (default ⇒ "App").
    assert!(body.contains("OrangeRock"), "brand name should render");
    assert!(
        body.contains("Open in OrangeRock"),
        "CTA should use brand name"
    );
    assert!(
        body.contains("Trade without limits"),
        "tagline should render"
    );
    // The palette engine emitted CSS variables for the brand accent.
    assert!(body.contains("--accent:"), "derived palette should render");
}

#[tokio::test]
async fn get_branding_returns_defaults_when_unset() {
    let app = common::spawn_app().await;
    let (key, _tenant_id) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .get(app.url("/v1/tenant/branding"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["template"], "default");
    assert_eq!(body["color_scheme"], "auto");
    assert_eq!(body["show_agent_panel"], true);
}

#[tokio::test]
async fn put_branding_rejects_invalid_color() {
    let app = common::spawn_app().await;
    let (key, _tenant_id) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .put(app.url("/v1/tenant/branding"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "theme_color": "orange" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_branding");
}

/// A per-link `landing_theme` override merges over the tenant theme — here it
/// hides the agent panel on one link while another (no override) keeps it.
#[tokio::test]
async fn per_link_override_hides_agent_panel() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;
    // Tenant default keeps the agent panel on (show_agent_panel defaults true).

    // Link WITH a per-link override hiding the agent panel.
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "hidden",
            "ios_deep_link": "app://x",
            "ios_store_url": "https://apps.apple.com/app/id1",
            "landing_theme": { "show_agent_panel": false }
        }))
        .send()
        .await
        .unwrap();

    // Control link with no override → inherits the tenant theme (panel shown).
    app.client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "shown",
            "ios_deep_link": "app://x",
            "ios_store_url": "https://apps.apple.com/app/id1"
        }))
        .send()
        .await
        .unwrap();

    let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)";
    let hidden = app
        .client
        .get(app.url("/r/hidden"))
        .header("User-Agent", ua)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let shown = app
        .client
        .get(app.url("/r/shown"))
        .header("User-Agent", ua)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // The override removed the agent panel for this link only.
    assert!(!hidden.contains("Machine-Readable Link"));
    assert!(hidden.contains("split solo"));
    // The control link still shows it.
    assert!(shown.contains("Machine-Readable Link"));
}

use crate::common;

#[tokio::test]
async fn create_theme_returns_201() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/themes"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "name": "Nord Roast",
            "slug": "nord-roast",
            "is_default": true,
            "tokens": {
                "palette": {
                    "primary": "#C46A2D",
                    "background": "#F4EBDD",
                    "text": "#2F241D"
                }
            },
            "copy": {
                "brand_name": "Nord Roast",
                "default_headline": "Your next bag is one tap away",
                "default_subheadline": "Open the app to claim this roast."
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["slug"], "nord-roast");
    assert_eq!(body["is_default"], true);
}

#[tokio::test]
async fn creating_new_default_theme_clears_previous_default() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    for slug in ["nord-roast", "volt-run"] {
        let resp = app
            .client
            .post(app.url("/v1/themes"))
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({
                "name": slug,
                "slug": slug,
                "is_default": true,
                "tokens": {
                    "palette": {
                        "primary": "#0d9488",
                        "background": "#081019",
                        "text": "#f8fafc"
                    }
                }
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 201);
    }

    let resp = app
        .client
        .get(app.url("/v1/themes"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    let themes = body["themes"].as_array().unwrap();
    assert_eq!(
        themes
            .iter()
            .filter(|theme| theme["is_default"] == true)
            .count(),
        1
    );
    let default_theme = themes
        .iter()
        .find(|theme| theme["is_default"] == true)
        .unwrap();
    assert_eq!(default_theme["slug"], "volt-run");
}

#[tokio::test]
async fn domain_theme_and_link_override_are_applied_to_landing_page() {
    let app = common::spawn_app().await;
    let (key, tenant_id) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_id, "go.example.com").await;

    let theme_resp = app
        .client
        .post(app.url("/v1/themes"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "name": "Atelier Stay",
            "slug": "atelier-stay",
            "is_default": true,
            "tokens": {
                "palette": {
                    "primary": "#1E3A5F",
                    "background": "#F7F4EF",
                    "surface": "#FFFCF8",
                    "surface_muted": "#EFE8DE",
                    "text": "#1F2933",
                    "text_muted": "#6B7280",
                    "border": "#DDD4C8"
                },
                "typography": {
                    "heading_font": "editorial_serif",
                    "body_font": "modern_sans",
                    "scale": "spacious"
                },
                "shape": {
                    "radius": "rounded",
                    "button_style": "soft",
                    "card_style": "flat",
                    "shadow": "soft"
                }
            },
            "copy": {
                "brand_name": "Atelier Stay",
                "tagline": "Private hotels for slower travel.",
                "default_headline": "Continue your reservation",
                "default_subheadline": "Open the app to view suite details and concierge recommendations.",
                "primary_cta_label": "Open Atelier Stay",
                "footer_text": "Member support available 24/7."
            },
            "media": {
                "hero_image_url": "https://assets.example.com/demo-themes/atelier-stay/hero-suite.png"
            },
            "layout": {
                "template": "centered",
                "alignment": "center",
                "content_width": "narrow"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(theme_resp.status(), 201);
    let theme_body: serde_json::Value = theme_resp.json().await.unwrap();
    let theme_id = theme_body["id"].as_str().unwrap();

    let domain_resp = app
        .client
        .put(app.url("/v1/domains/go.example.com/theme"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({ "theme_id": theme_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(domain_resp.status(), 200);

    let create_link = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "custom_id": "stay-launch",
            "ios_deep_link": "atelier://stay/launch",
            "ios_store_url": "https://apps.apple.com/app/id123456789",
            "web_url": "https://atelier.example.com/stay/launch",
            "theme_override": {
                "headline": "Book the Cedar Suite",
                "subheadline": "Launch-week access with late checkout and a private dining credit.",
                "badge_text": "Founding Guest",
                "primary_cta_label": "Claim your suite",
                "hero_image_url": "https://assets.example.com/demo-themes/atelier-stay/campaign-cedar-suite.png"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_link.status(), 201);

    let resp = app
        .client
        .get(app.url("/stay-launch"))
        .header("x-rift-host", "go.example.com")
        .header(
            "User-Agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Book the Cedar Suite"));
    assert!(body.contains("Founding Guest"));
    assert!(body.contains("Claim your suite"));
    assert!(body.contains("campaign-cedar-suite.png"));
    assert!(body.contains("#1E3A5F"));
}

#[tokio::test]
async fn link_theme_override_rejects_theme_from_other_tenant() {
    let app = common::spawn_app().await;
    let (key_a, tenant_a) = common::seed_api_key(&app).await;
    common::seed_verified_domain(&app, &tenant_a, "go.tenant-a.com").await;

    let theme_resp = app
        .client
        .post(app.url("/v1/themes"))
        .header("Authorization", format!("Bearer {key_a}"))
        .json(&serde_json::json!({
            "name": "Volt Run",
            "slug": "volt-run",
            "tokens": {
                "palette": {
                    "primary": "#B6FF00",
                    "background": "#05070A",
                    "text": "#F5F7FA"
                }
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(theme_resp.status(), 201);
    let theme_body: serde_json::Value = theme_resp.json().await.unwrap();
    let theme_id = theme_body["id"].as_str().unwrap();

    let (key_b, tenant_b) = common::seed_api_key_with(
        &app,
        "rl_live_test_b_234567890abcdef1234567890abcdef12345678",
    )
    .await;
    common::seed_verified_domain(&app, &tenant_b, "go.tenant-b.com").await;

    let resp = app
        .client
        .post(app.url("/v1/links"))
        .header("Authorization", format!("Bearer {key_b}"))
        .json(&serde_json::json!({
            "custom_id": "other-tenant-theme",
            "web_url": "https://tenant-b.example.com",
            "theme_override": {
                "theme_id": theme_id
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_theme_override");
}

use crate::common;

#[tokio::test]
async fn create_android_app_requires_fingerprints() {
    let app = common::spawn_app().await;
    let (key, _) = common::seed_api_key(&app).await;

    let resp = app
        .client
        .post(app.url("/v1/apps"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "platform": "android",
            "package_name": "com.example.app"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

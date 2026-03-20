use crate::common;

#[tokio::test]
async fn health_returns_ok() {
    let app = common::spawn_app().await;
    let resp = app.client.get(app.url("/health")).send().await.unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn openapi_json_returns_spec() {
    let app = common::spawn_app().await;
    let resp = app
        .client
        .get(app.url("/openapi.json"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["info"]["title"].as_str().unwrap().contains("Relay"));
    assert!(!body["paths"].as_object().unwrap().is_empty());
}

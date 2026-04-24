use super::*;
use std::collections::HashMap;
use std::sync::Mutex;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// In-memory test double for `RiftStorage`.
#[derive(Debug, Default)]
struct InMemoryStorage {
    data: Mutex<HashMap<String, String>>,
}

impl RiftStorage for InMemoryStorage {
    fn get(&self, key: String) -> Result<Option<String>, StorageError> {
        Ok(self.data.lock().unwrap().get(&key).cloned())
    }

    fn set(&self, key: String, value: String) -> Result<(), StorageError> {
        self.data.lock().unwrap().insert(key, value);
        Ok(())
    }

    fn remove(&self, key: String) -> Result<(), StorageError> {
        self.data.lock().unwrap().remove(&key);
        Ok(())
    }
}

/// Build a RiftSdk pointed at a wiremock server with an in-memory storage
/// and a shared handle to the storage so tests can inspect state.
fn make_sdk(base_url: String) -> (Arc<RiftSdk>, Arc<InMemoryStorage>) {
    let storage: Arc<InMemoryStorage> = Arc::new(InMemoryStorage::default());
    let storage_for_sdk: Arc<dyn RiftStorage> = storage.clone();
    let sdk = RiftSdk::new(
        RiftConfig {
            publishable_key: "pk_live_test".to_string(),
            base_url: Some(base_url),
            log_level: Some("error".to_string()),
            app_version: Some("1.0.0-test".to_string()),
        },
        storage_for_sdk,
    );
    (sdk, storage)
}

// ── install_id ──

#[tokio::test]
async fn install_id_is_stable_across_calls() {
    let server = MockServer::start().await;
    let (sdk, _storage) = make_sdk(server.uri());

    let id1 = sdk.install_id().unwrap();
    let id2 = sdk.install_id().unwrap();
    assert_eq!(id1, id2);
}

#[tokio::test]
async fn install_id_generates_valid_uuid() {
    let server = MockServer::start().await;
    let (sdk, _storage) = make_sdk(server.uri());

    let id = sdk.install_id().unwrap();
    assert!(uuid::Uuid::parse_str(&id).is_ok(), "expected a valid UUID");
}

#[tokio::test]
async fn install_id_persists_via_storage() {
    let server = MockServer::start().await;
    let (sdk, storage) = make_sdk(server.uri());

    let id = sdk.install_id().unwrap();
    assert_eq!(
        storage.get(STORAGE_KEY_INSTALL_ID.to_string()).unwrap(),
        Some(id)
    );
}

// ── set_user_id ──

#[tokio::test]
async fn set_user_id_happy_path_marks_synced() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/v1/attribution/identify"))
        .and(header("Authorization", "Bearer pk_live_test"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "success": true })),
        )
        .mount(&server)
        .await;

    let (sdk, storage) = make_sdk(server.uri());

    sdk.set_user_id("usr_abc".to_string()).await.unwrap();

    assert_eq!(
        storage.get(STORAGE_KEY_USER_ID.to_string()).unwrap(),
        Some("usr_abc".to_string())
    );
    assert_eq!(
        storage.get(STORAGE_KEY_USER_ID_SYNCED.to_string()).unwrap(),
        Some("true".to_string())
    );
}

#[tokio::test]
async fn set_user_id_server_error_marks_unsynced() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/v1/attribution/identify"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(serde_json::json!({ "error": "db error" })),
        )
        .mount(&server)
        .await;

    let (sdk, storage) = make_sdk(server.uri());

    let err = sdk.set_user_id("usr_xyz".to_string()).await;
    assert!(err.is_err());

    // user_id is still stored (so we can retry later), but synced flag is "false".
    assert_eq!(
        storage.get(STORAGE_KEY_USER_ID.to_string()).unwrap(),
        Some("usr_xyz".to_string())
    );
    assert_eq!(
        storage.get(STORAGE_KEY_USER_ID_SYNCED.to_string()).unwrap(),
        Some("false".to_string())
    );
}

#[tokio::test]
async fn set_user_id_rejects_empty() {
    let server = MockServer::start().await;
    let (sdk, _storage) = make_sdk(server.uri());

    let err = sdk.set_user_id("".to_string()).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn set_user_id_idempotent_noop_when_already_synced() {
    let server = MockServer::start().await;
    // Only one call should ever hit the server.
    Mock::given(method("PUT"))
        .and(path("/v1/attribution/identify"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "success": true })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (sdk, _storage) = make_sdk(server.uri());

    sdk.set_user_id("usr_same".to_string()).await.unwrap();
    sdk.set_user_id("usr_same".to_string()).await.unwrap();
    // If the second call hit the server, wiremock's `expect(1)` would fail on drop.
}

#[tokio::test]
async fn clear_user_id_removes_both_keys() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/v1/attribution/identify"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "success": true })),
        )
        .mount(&server)
        .await;

    let (sdk, storage) = make_sdk(server.uri());

    sdk.set_user_id("usr_logout".to_string()).await.unwrap();
    sdk.clear_user_id().unwrap();

    assert_eq!(storage.get(STORAGE_KEY_USER_ID.to_string()).unwrap(), None);
    assert_eq!(
        storage.get(STORAGE_KEY_USER_ID_SYNCED.to_string()).unwrap(),
        None
    );
    // install_id is preserved.
    assert!(storage
        .get(STORAGE_KEY_INSTALL_ID.to_string())
        .unwrap()
        .is_some());
}

// ── retry_pending_binding ──

#[tokio::test]
async fn retry_pending_binding_fires_when_unsynced() {
    // Pre-seed storage with an unbound user_id, then construct the SDK.
    // The constructor spots the unsynced state and spawns a retry task
    // in the background. We poll storage briefly to confirm it lands.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/v1/attribution/identify"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "success": true })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let storage: Arc<InMemoryStorage> = Arc::new(InMemoryStorage::default());
    storage
        .set(
            STORAGE_KEY_INSTALL_ID.to_string(),
            "preseed-install".to_string(),
        )
        .unwrap();
    storage
        .set(STORAGE_KEY_USER_ID.to_string(), "usr_retry".to_string())
        .unwrap();
    storage
        .set(STORAGE_KEY_USER_ID_SYNCED.to_string(), "false".to_string())
        .unwrap();

    let storage_for_sdk: Arc<dyn RiftStorage> = storage.clone();
    let _sdk = RiftSdk::new(
        RiftConfig {
            publishable_key: "pk_live_test".to_string(),
            base_url: Some(server.uri()),
            log_level: Some("error".to_string()),
            app_version: Some("1.0.0-test".to_string()),
        },
        storage_for_sdk,
    );

    // Wait for the background retry task to complete. 500ms is plenty for
    // a local wiremock roundtrip.
    for _ in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        if storage.get(STORAGE_KEY_USER_ID_SYNCED.to_string()).unwrap() == Some("true".to_string())
        {
            return;
        }
    }
    panic!("retry_pending_binding never marked synced=true within 500ms");
}

#[tokio::test]
async fn retry_pending_binding_noop_when_already_synced() {
    // Server must never be called if the synced flag is true.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/v1/attribution/identify"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let storage: Arc<InMemoryStorage> = Arc::new(InMemoryStorage::default());
    storage
        .set(STORAGE_KEY_USER_ID.to_string(), "usr_ok".to_string())
        .unwrap();
    storage
        .set(STORAGE_KEY_USER_ID_SYNCED.to_string(), "true".to_string())
        .unwrap();

    let storage_for_sdk: Arc<dyn RiftStorage> = storage.clone();
    let sdk = RiftSdk::new(
        RiftConfig {
            publishable_key: "pk_live_test".to_string(),
            base_url: Some(server.uri()),
            log_level: Some("error".to_string()),
            app_version: Some("1.0.0-test".to_string()),
        },
        storage_for_sdk,
    );

    sdk.retry_pending_binding().await.unwrap();
}

#[tokio::test]
async fn retry_pending_binding_noop_when_no_user_id() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/v1/attribution/identify"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let (sdk, _storage) = make_sdk(server.uri());
    sdk.retry_pending_binding().await.unwrap();
}

use std::fs;
use tempfile::tempdir;

use rift_cli::config::StoredConfig;

#[test]
fn save_and_load_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let config = StoredConfig {
        secret_key: "rl_live_test1234567890".into(),
        base_url: "https://api.riftl.ink".into(),
    };

    config.save_to(&path).unwrap();
    let loaded = StoredConfig::load_from(&path).unwrap();
    assert_eq!(config, loaded);
}

#[test]
fn save_creates_parent_directories() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("deep").join("config.json");

    let config = StoredConfig {
        secret_key: "rl_live_abc".into(),
        base_url: "https://localhost:3000".into(),
    };

    config.save_to(&path).unwrap();
    assert!(path.exists());

    let loaded = StoredConfig::load_from(&path).unwrap();
    assert_eq!(config, loaded);
}

#[test]
fn load_missing_file_returns_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");

    let result = StoredConfig::load_from(&path);
    assert!(result.is_err());
}

#[test]
fn load_corrupt_json_returns_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, "not valid json {{{").unwrap();

    let result = StoredConfig::load_from(&path);
    assert!(result.is_err());
}

#[test]
fn load_missing_field_returns_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, r#"{"secret_key": "rl_live_test"}"#).unwrap();

    let result = StoredConfig::load_from(&path);
    assert!(result.is_err());
}

#[test]
fn saved_file_is_pretty_json() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let config = StoredConfig {
        secret_key: "rl_live_xyz".into(),
        base_url: "https://api.riftl.ink".into(),
    };

    config.save_to(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();

    // Pretty-printed JSON has newlines and indentation
    assert!(text.contains('\n'));
    assert!(text.contains("  "));

    // Verify the raw JSON contains our values
    assert!(text.contains("rl_live_xyz"));
    assert!(text.contains("https://api.riftl.ink"));
}

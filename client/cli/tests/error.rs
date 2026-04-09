use rift_cli::error::{exit, CliError};
use rift_client_core::error::RiftClientError;

// ── Exit code mapping ──

#[test]
fn not_logged_in_exit_code() {
    assert_eq!(CliError::NotLoggedIn.exit_code(), exit::NOT_LOGGED_IN);
}

#[test]
fn auth_failed_exit_code() {
    assert_eq!(CliError::AuthFailed.exit_code(), exit::AUTH_FAILED);
}

#[test]
fn api_error_exit_code() {
    let err = CliError::Api {
        status: 404,
        message: "not found".into(),
    };
    assert_eq!(err.exit_code(), exit::API_ERROR);
}

#[test]
fn network_error_exit_code() {
    assert_eq!(
        CliError::Network("timeout".into()).exit_code(),
        exit::NETWORK
    );
}

#[test]
fn general_error_exit_code() {
    assert_eq!(CliError::General("oops".into()).exit_code(), exit::GENERAL);
}

// ── From<RiftClientError> conversions ──

#[test]
fn from_api_401_becomes_auth_failed() {
    let err: CliError = RiftClientError::Api {
        status: 401,
        message: "Unauthorized".into(),
    }
    .into();
    assert_eq!(err.exit_code(), exit::AUTH_FAILED);
    assert!(err.to_string().contains("rift login"));
}

#[test]
fn from_api_404_becomes_api_error() {
    let err: CliError = RiftClientError::Api {
        status: 404,
        message: "not found".into(),
    }
    .into();
    assert_eq!(err.exit_code(), exit::API_ERROR);
    assert!(err.to_string().contains("404"));
    assert!(err.to_string().contains("not found"));
}

#[test]
fn from_network_becomes_network_error() {
    let err: CliError = RiftClientError::Network("connection refused".into()).into();
    assert_eq!(err.exit_code(), exit::NETWORK);
    assert!(err.to_string().contains("connection refused"));
}

#[test]
fn from_deserialize_becomes_general() {
    let err: CliError = RiftClientError::Deserialize("bad json".into()).into();
    assert_eq!(err.exit_code(), exit::GENERAL);
    assert!(err.to_string().contains("bad json"));
}

// ── Display messages are actionable ──

#[test]
fn not_logged_in_message_suggests_init_and_login() {
    let msg = CliError::NotLoggedIn.to_string();
    assert!(msg.contains("rift init"));
    assert!(msg.contains("rift login"));
}

#[test]
fn auth_failed_message_suggests_login() {
    let msg = CliError::AuthFailed.to_string();
    assert!(msg.contains("rift login"));
    assert!(msg.contains("rl_live_"));
}

#[test]
fn api_error_message_includes_status_and_body() {
    let err = CliError::Api {
        status: 409,
        message: "Domain already registered".into(),
    };
    let msg = err.to_string();
    assert!(msg.contains("409"));
    assert!(msg.contains("Domain already registered"));
}

#[test]
fn network_error_message_includes_cause() {
    let msg = CliError::Network("dns resolution failed".into()).to_string();
    assert!(msg.contains("dns resolution failed"));
    assert!(msg.contains("Rift API"));
}

// ── From<&str> and From<String> ──

#[test]
fn from_str_creates_general() {
    let err: CliError = "something went wrong".into();
    assert_eq!(err.exit_code(), exit::GENERAL);
    assert_eq!(err.to_string(), "something went wrong");
}

#[test]
fn from_string_creates_general() {
    let err: CliError = String::from("something went wrong").into();
    assert_eq!(err.exit_code(), exit::GENERAL);
    assert_eq!(err.to_string(), "something went wrong");
}

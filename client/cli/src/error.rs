use std::fmt;
use std::process;

use rift_client_core::error::RiftClientError;

/// Exit codes for the CLI.
pub mod exit {
    pub const GENERAL: i32 = 1;
    pub const NOT_LOGGED_IN: i32 = 2;
    pub const AUTH_FAILED: i32 = 3;
    pub const API_ERROR: i32 = 4;
    pub const NETWORK: i32 = 5;
}

/// Structured CLI error with actionable messages and distinct exit codes.
pub enum CliError {
    /// User hasn't run `rift init` or `rift login` yet.
    NotLoggedIn,
    /// API key is invalid or revoked (401).
    AuthFailed,
    /// API returned a non-auth error.
    Api { status: u16, message: String },
    /// Network/connection error.
    Network(String),
    /// User cancelled or other general error.
    General(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLoggedIn => {
                write!(
                    f,
                    "Not logged in. Run `rift init` or `rift login` to connect this machine."
                )
            }
            Self::AuthFailed => {
                write!(
                    f,
                    "API key is invalid or revoked. Run `rift login` with a valid `rl_live_...` key."
                )
            }
            Self::Api { status, message } => write!(f, "API error ({status}): {message}"),
            Self::Network(msg) => write!(f, "Could not reach the Rift API: {msg}"),
            Self::General(msg) => write!(f, "{msg}"),
        }
    }
}

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for CliError {}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotLoggedIn => exit::NOT_LOGGED_IN,
            Self::AuthFailed => exit::AUTH_FAILED,
            Self::Api { .. } => exit::API_ERROR,
            Self::Network(_) => exit::NETWORK,
            Self::General(_) => exit::GENERAL,
        }
    }
}

impl From<RiftClientError> for CliError {
    fn from(err: RiftClientError) -> Self {
        match err {
            RiftClientError::Api { status: 401, .. } => Self::AuthFailed,
            RiftClientError::Api { status, message } => Self::Api { status, message },
            RiftClientError::Network(msg) => Self::Network(msg),
            RiftClientError::Deserialize(msg) => {
                Self::General(format!("Unexpected API response: {msg}"))
            }
        }
    }
}

impl From<String> for CliError {
    fn from(msg: String) -> Self {
        Self::General(msg)
    }
}

impl From<&str> for CliError {
    fn from(msg: &str) -> Self {
        Self::General(msg.to_string())
    }
}

impl From<dialoguer::Error> for CliError {
    fn from(err: dialoguer::Error) -> Self {
        Self::General(err.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        Self::General(err.to_string())
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::General(err.to_string())
    }
}

/// Run the CLI and exit with the appropriate code on error.
pub fn run_main(result: Result<(), CliError>) {
    if let Err(err) = result {
        eprintln!("Error: {err}");
        process::exit(err.exit_code());
    }
}

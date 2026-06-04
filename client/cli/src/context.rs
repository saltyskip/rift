use crate::config::StoredConfig;
use crate::error::CliError;
use rift_client_core::RiftClient;

/// Load stored config and return an authenticated client.
/// Returns `CliError::NotLoggedIn` when the user hasn't run `rift login` yet.
///
/// A browser-login `session_token` takes precedence over a `secret_key` when
/// both are somehow present; normally exactly one is stored.
pub fn authenticated_client() -> Result<RiftClient, CliError> {
    let config = StoredConfig::load().map_err(|_| CliError::NotLoggedIn)?;
    if let Some(token) = config.session_token {
        Ok(RiftClient::with_session_token(token, Some(config.base_url)))
    } else if let Some(key) = config.secret_key {
        Ok(RiftClient::with_secret_key(key, Some(config.base_url)))
    } else {
        Err(CliError::NotLoggedIn)
    }
}

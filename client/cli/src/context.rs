use crate::config::StoredConfig;
use crate::error::CliError;
use rift_client_core::RiftClient;

/// Load stored config and return an authenticated client.
/// Returns `CliError::NotLoggedIn` when the user hasn't run `rift init` yet.
pub fn authenticated_client() -> Result<RiftClient, CliError> {
    let config = StoredConfig::load().map_err(|_| CliError::NotLoggedIn)?;
    Ok(RiftClient::with_secret_key(
        config.secret_key,
        Some(config.base_url),
    ))
}

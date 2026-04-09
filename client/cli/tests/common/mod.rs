use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::TempDir;
use wiremock::MockServer;

use rift_cli::config::StoredConfig;

/// A running mock API server with a temp HOME directory containing a valid config.
pub struct TestHarness {
    pub server: MockServer,
    pub home: TempDir,
}

impl TestHarness {
    /// Spin up a wiremock server and write a config file that points at it.
    pub async fn spawn() -> Self {
        let server = MockServer::start().await;
        let home = tempfile::tempdir().unwrap();

        let config = StoredConfig {
            secret_key: "rl_live_test_key_1234567890".into(),
            base_url: server.uri(),
        };

        let config_path = config_path_for_home(home.path());
        config.save_to(&config_path).unwrap();

        Self { server, home }
    }

    /// Build a `Command` for the `rift` binary with HOME pointed at the temp dir.
    /// Also sets XDG_CONFIG_HOME on Linux so dirs::config_dir() resolves correctly
    /// even when CI has its own XDG_CONFIG_HOME set.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("rift").unwrap();
        cmd.env("HOME", self.home.path());
        if cfg!(not(target_os = "macos")) {
            cmd.env("XDG_CONFIG_HOME", self.home.path().join(".config"));
        }
        cmd
    }
}

/// Resolve the config file path the same way `dirs::config_dir()` does when HOME is overridden.
/// macOS: $HOME/Library/Application Support/rift/config.json
/// Linux: $HOME/.config/rift/config.json
fn config_path_for_home(home: &std::path::Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/rift/config.json")
    } else {
        home.join(".config/rift/config.json")
    }
}

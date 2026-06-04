//! `rift login` — sign in with the browser (default) or paste an API key
//! (`--api-key`).
//!
//! Browser flow: bind a one-shot loopback listener, open
//! `${api}/v1/auth/cli/start?redirect_uri=…&state=…`, which bounces to the
//! dashboard's `/cli/authorize` page. The user signs in (magic-link or OAuth)
//! and approves; the dashboard navigates the browser back to our loopback
//! listener carrying the freshly minted session token.

use std::time::{Duration, Instant};

use dialoguer::Password;

use crate::config::StoredConfig;
use crate::error::CliError;
use crate::ui;

const DEFAULT_BASE_URL: &str = "https://api.riftl.ink";
/// How long to wait for the browser round-trip before giving up.
const LOGIN_TIMEOUT_SECS: u64 = 180;

pub async fn run(base_url: Option<String>, api_key: bool, json: bool) -> Result<(), CliError> {
    let base_url = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    if api_key {
        run_api_key(base_url, json).await
    } else {
        run_browser(base_url, json).await
    }
}

// ── Browser flow (default) ──

async fn run_browser(base_url: String, json: bool) -> Result<(), CliError> {
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| CliError::General(format!("Could not start local listener: {e}")))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|addr| addr.port())
        .ok_or_else(|| CliError::General("Could not resolve local listener port".to_string()))?;

    let state = random_state()?;
    let redirect_uri = format!("http://127.0.0.1:{port}/");
    let start_url = reqwest::Url::parse_with_params(
        &format!("{base_url}/v1/auth/cli/start"),
        &[
            ("redirect_uri", redirect_uri.as_str()),
            ("state", state.as_str()),
        ],
    )
    .map_err(|e| CliError::General(e.to_string()))?;

    if !json {
        ui::heading("Rift Login", "Sign in with your browser.");
        ui::spacer();
        ui::note(&format!("Opening {start_url}"));
    }
    if let Err(e) = webbrowser::open(start_url.as_str()) {
        if !json {
            ui::warning(&format!(
                "Couldn't open the browser automatically ({e}). Visit the URL above to continue."
            ));
        }
    }

    let pb = (!json).then(|| ui::spinner("Waiting for browser sign-in…"));

    // The listener blocks, so wait on a worker thread.
    let expected_state = state.clone();
    let token = tokio::task::spawn_blocking(move || wait_for_token(server, &expected_state))
        .await
        .map_err(|e| CliError::General(format!("login task failed: {e}")))?;
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }
    let token = token?;

    // Confirm the token works and learn who we are before persisting.
    let client =
        rift_client_core::RiftClient::with_session_token(token.clone(), Some(base_url.clone()));
    let me = client.me().await?;

    StoredConfig::from_session_token(token, base_url.clone())
        .save()
        .map_err(CliError::General)?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "base_url": base_url,
                "configured": true,
                "email": me.user.email,
            })
        );
    } else {
        ui::success(&format!("Signed in as {}.", me.user.email));
        let config_path = StoredConfig::path().map_err(CliError::General)?;
        ui::kv("Saved config", config_path.display().to_string());
    }

    Ok(())
}

// ── API-key flow (`--api-key`) ──

async fn run_api_key(base_url: String, json: bool) -> Result<(), CliError> {
    if !json {
        ui::heading(
            "Rift Login",
            "Connect this machine with an existing API key.",
        );
        ui::spacer();
    }

    let secret_key = Password::with_theme(&ui::theme())
        .with_prompt("Paste your `rl_live_...` API key")
        .with_confirmation("Confirm API key", "Keys did not match")
        .interact()?;

    // Verify the key actually works before saving.
    let client =
        rift_client_core::RiftClient::with_secret_key(secret_key.clone(), Some(base_url.clone()));
    client.list_users().await?;

    StoredConfig::from_secret_key(secret_key, base_url.clone())
        .save()
        .map_err(CliError::General)?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "base_url": base_url,
                "configured": true,
            })
        );
    } else {
        let config_path = StoredConfig::path().map_err(CliError::General)?;
        ui::success("This machine is now connected to Rift.");
        ui::kv("Saved config", config_path.display().to_string());
    }

    Ok(())
}

// ── Helpers ──

/// Block until the browser hits the loopback listener with a `token`. Ignores
/// stray requests (e.g. `/favicon.ico`) and enforces the `state` nonce.
fn wait_for_token(server: tiny_http::Server, expected_state: &str) -> Result<String, CliError> {
    let deadline = Instant::now() + Duration::from_secs(LOGIN_TIMEOUT_SECS);
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(timeout_err());
        };
        match server.recv_timeout(remaining) {
            Ok(Some(req)) => match parse_callback(req.url()) {
                Some((token, callback_state)) => {
                    if callback_state.as_deref() != Some(expected_state) {
                        respond_html(req, 400, ERROR_PAGE);
                        return Err(CliError::General(
                            "Sign-in state mismatch — possible cross-site attempt. Run `rift login` again.".to_string(),
                        ));
                    }
                    respond_html(req, 200, SUCCESS_PAGE);
                    return Ok(token);
                }
                // Stray request (favicon, preflight, etc.) — keep waiting.
                None => {
                    let _ = req.respond(tiny_http::Response::empty(204));
                }
            },
            // recv_timeout returns Ok(None) when the deadline elapses.
            Ok(None) => return Err(timeout_err()),
            Err(_) => continue,
        }
    }
}

/// Parse `token` / `state` from the loopback request target
/// (`/?token=…&state=…`). Returns `None` if there's no `token`.
fn parse_callback(target: &str) -> Option<(String, Option<String>)> {
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}")).ok()?;
    let mut token = None;
    let mut state = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "token" => token = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }
    token.map(|t| (t, state))
}

fn respond_html(req: tiny_http::Request, status: u16, html: &str) {
    let response = tiny_http::Response::from_string(html).with_status_code(status);
    let response =
        match tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
        {
            Ok(header) => response.with_header(header),
            Err(()) => response,
        };
    let _ = req.respond(response);
}

fn random_state() -> Result<String, CliError> {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).map_err(|e| CliError::General(format!("RNG error: {e}")))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

fn timeout_err() -> CliError {
    CliError::General("Timed out waiting for browser sign-in. Run `rift login` again.".to_string())
}

const SUCCESS_PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Signed in</title></head><body style=\"font-family:system-ui,sans-serif;background:#0a0a0b;color:#fafafa;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"font-size:20px\">You're signed in.</h1><p style=\"color:#a1a1aa\">Return to your terminal — you can close this tab.</p></div></body></html>";

const ERROR_PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Sign-in failed</title></head><body style=\"font-family:system-ui,sans-serif;background:#0a0a0b;color:#fafafa;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"font-size:20px\">Sign-in failed.</h1><p style=\"color:#a1a1aa\">Return to your terminal and run <code>rift login</code> again.</p></div></body></html>";

//! Query-param DTOs for the OAuth route handlers.
//!
//! Both `/start` and `/callback` are GETs — the browser sends query strings,
//! never JSON bodies. Response shapes don't exist (both endpoints redirect).

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StartQuery {
    /// Same-origin path on the marketing site to redirect the user to after
    /// a successful sign-in. Validated by `sanitize_next` at start time;
    /// falls back to `/account` on rejection.
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    /// Authorization code from the provider. Exchanged for an access token.
    pub code: Option<String>,
    /// State token we issued at `/start`. Carries `code_verifier`, `next`,
    /// `origin`. Single-use.
    pub state: Option<String>,
    /// Set by GitHub/Google when the user cancels or denies — we forward
    /// these through to the signin page as a toast.
    #[serde(default)]
    pub error: Option<String>,
}

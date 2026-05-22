//! Transport-only DTOs for the analytics HTTP surface.
//!
//! The response types and the parsed-params struct live on the service
//! (`services/analytics/models.rs`) per CLAUDE.md's shared-model rule.
//! This file holds only the things that are genuinely HTTP-shaped —
//! the raw query string parsed off the URL.

use serde::Deserialize;
use utoipa::IntoParams;

/// Query parameters for `GET /v1/analytics/stats`. Transport-only —
/// the parsed shape (`FunnelParams`) lives on the service.
#[derive(Debug, Deserialize, IntoParams)]
pub struct StatsQuery {
    /// Comma-separated set of link_ids. One link = special case of one
    /// ID. "Campaign" is whatever set the caller passes.
    pub link_ids: String,
    /// Start of date range, RFC 3339. Defaults to 30 days ago.
    pub from: Option<String>,
    /// End of date range, RFC 3339. Defaults to now.
    pub to: Option<String>,
    /// Attribution model: `last_touch` (default — matches the
    /// marketer's "which campaign closed this user"), `first_touch`,
    /// or `touched`. Unknown values fall back to `last_touch`.
    pub credit: Option<String>,
}

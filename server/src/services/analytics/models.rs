//! Shared types for `services/analytics/` — request params, response
//! shapes, and the error enum.
//!
//! Per the "shared model layer" rule (see CLAUDE.md), these types are
//! consumed by both transports (REST today, MCP later) so the response
//! contract stays in sync. `ToSchema` powers REST OpenAPI; `JsonSchema`
//! (behind the `mcp` feature) powers MCP tool-input schemas.

use mongodb::bson::DateTime;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use utoipa::ToSchema;

use crate::services::auth::permissions::AuthzError;
use crate::services::links::models::CreditModel;

/// Parsed funnel-stats query. Transport layers do their own input
/// parsing (HTTP query string, MCP tool input, etc.) and hand the
/// service a `FunnelParams`.
#[derive(Debug, Clone)]
pub struct FunnelParams {
    pub link_ids: Vec<String>,
    pub from: DateTime,
    pub to: DateTime,
    pub credit: CreditModel,
}

/// Funnel response — one number per leaf, four top-level stages.
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct FunnelResult {
    /// ISO 8601.
    #[schema(example = "2026-04-01T00:00:00Z")]
    pub from: String,
    #[schema(example = "2026-05-21T00:00:00Z")]
    pub to: String,
    pub link_ids: Vec<String>,
    /// Attribution model the counts were computed under.
    #[schema(example = "last_touch")]
    pub credit: String,
    pub funnel: Funnel,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct Funnel {
    #[schema(example = 4892)]
    pub clicks: u64,
    pub new_users: NewUsers,
    pub returning_users: ReturningUsers,
    /// Conversions across both new and returning users, keyed by
    /// conversion type (e.g. "signup", "purchase").
    pub conversions: BTreeMap<String, u64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct NewUsers {
    /// First-time install_ids attributed to these links.
    #[schema(example = 240)]
    pub installed: u64,
    /// Of those installs, how many became identified users.
    #[schema(example = 198)]
    pub identified: u64,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
pub struct ReturningUsers {
    /// Known user appeared on a new install_id with the SAME device
    /// model (uninstall + reinstall — Android-detectable).
    #[schema(example = 30)]
    pub reinstalled: u64,
    /// Known user appeared on a new install_id with a DIFFERENT device
    /// model (added another phone / tablet).
    #[schema(example = 20)]
    pub new_device: u64,
    /// Known install touched the link again (active re-engagement).
    #[schema(example = 1200)]
    pub engaged: u64,
}

/// Service-layer error enum. Route handlers map these to HTTP status
/// codes; future MCP tool handlers map them to MCP error responses.
#[derive(Debug)]
pub enum AnalyticsError {
    /// Caller passed an empty `link_ids` set.
    EmptyLinkIds,
    /// Caller passed `from > to`.
    InvalidDateRange,
    /// Required infrastructure (database) is not configured. Surfaced
    /// instead of panicking in reduced-feature builds.
    Unavailable,
    Forbidden(AuthzError),
    Internal(String),
}

impl fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLinkIds => write!(f, "link_ids is required (comma-separated)"),
            Self::InvalidDateRange => write!(f, "from must be <= to"),
            Self::Unavailable => write!(f, "Analytics not configured"),
            Self::Forbidden(e) => write!(f, "{e}"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl AnalyticsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyLinkIds | Self::InvalidDateRange => "bad_request",
            Self::Unavailable => "no_database",
            Self::Forbidden(e) => e.code(),
            Self::Internal(_) => "db_error",
        }
    }
}

impl From<AuthzError> for AnalyticsError {
    fn from(err: AuthzError) -> Self {
        AnalyticsError::Forbidden(err)
    }
}

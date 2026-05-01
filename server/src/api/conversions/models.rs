//! Request / response DTOs for `api/conversions/routes.rs`.

use serde::Deserialize;
use utoipa::ToSchema;

/// Request body for the SDK conversion endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SdkConversionRequest {
    /// The user ID (must match a previously bound user via setUserId).
    #[schema(example = "usr_abc123")]
    pub user_id: String,
    /// Conversion type (free-form, e.g. "spot_trade", "perps_trade", "swap").
    #[serde(rename = "type")]
    #[schema(example = "spot_trade")]
    pub conversion_type: String,
    /// Idempotency key to prevent double-counting (e.g. order ID, tx hash).
    #[schema(example = "order-12345")]
    pub idempotency_key: Option<String>,
    /// Arbitrary metadata (max 1KB). Stored verbatim, forwarded on outbound webhooks.
    pub metadata: Option<serde_json::Value>,
}

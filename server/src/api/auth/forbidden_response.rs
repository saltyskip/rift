//! Shared mapping from `AuthzError` to a 403 HTTP response. Every service
//! that surfaces a permission failure (via `?` against a `Forbidden(AuthzError)`
//! variant in its error enum) maps through this helper so the wire shape stays
//! consistent across endpoints — mirrors `api/billing/quota_response::to_response`
//! for `QuotaError`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;

use crate::services::auth::permissions::AuthzError;

/// Turn an `AuthzError` into the documented 403 body. Carries the missing
/// permission string(s) so clients can render actionable errors without
/// parsing the human-readable message.
pub fn to_response(err: AuthzError) -> Response {
    let body = match &err {
        AuthzError::MissingPermission(p) => json!({
            "error": err.to_string(),
            "code": err.code(),
            "missing": p.to_wire_str(),
        }),
        AuthzError::AnyOfMissing(ps) => {
            let strs: Vec<&'static str> = ps.iter().map(|p| p.to_wire_str()).collect();
            json!({
                "error": err.to_string(),
                "code": err.code(),
                "missing_any_of": strs,
            })
        }
    };
    (StatusCode::FORBIDDEN, Json(body)).into_response()
}

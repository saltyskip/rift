use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use super::models::{
    BillingStatusResponse, CheckoutSessionResponse, LimitsView, MagicLinkRequest,
    MagicLinkResponse, PortalSessionResponse,
};
use crate::api::auth::models::TenantId;
use crate::app::AppState;
use crate::services::auth::tenants::repo::{BillingMethod, PlanTier};
use crate::services::billing::handoff::{BillingHandoffError, HandoffOutcome};
use crate::services::billing::limits::limits_for;
use crate::services::billing::models::{BillingError, BillingStatus};
use crate::services::billing::stripe_client::{
    cancel_subscription_at_period_end, create_checkout_session, create_portal_session, StripeError,
};

fn render(status: BillingStatus) -> BillingStatusResponse {
    let limits = limits_for(status.effective_tier);
    BillingStatusResponse {
        plan_tier: status.plan_tier,
        effective_tier: status.effective_tier,
        comp_active: status.comp_active,
        billing_method: status.billing_method,
        status: status.status,
        current_period_end: status.current_period_end.map(|d| d.timestamp_millis()),
        limits: LimitsView {
            max_links: limits.max_links,
            max_events_per_month: limits.max_events_per_month,
            max_domains: limits.max_domains,
            max_team_members: limits.max_team_members,
            max_webhooks: limits.max_webhooks,
            analytics_retention: limits.retention_bucket,
        },
    }
}

#[utoipa::path(
    get,
    path = "/v1/billing/status",
    tag = "Billing",
    responses(
        (status = 200, description = "Current billing state for the authenticated tenant", body = BillingStatusResponse),
        (status = 401, description = "Missing or invalid API key", body = crate::error::ErrorResponse),
        (status = 503, description = "Billing service not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_billing_status(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(service) = state.billing_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Billing service not configured", "code": "no_billing" })),
        )
            .into_response();
    };

    match service.status(&tenant.0).await {
        Ok(status) => (StatusCode::OK, Json(render(status))).into_response(),
        Err(BillingError::TenantNotFound) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Tenant not found", "code": "tenant_not_found" })),
        )
            .into_response(),
        Err(BillingError::Internal(e)) => {
            tracing::error!(error = %e, "billing_status_db_error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── POST /v1/billing/stripe/checkout — start Stripe Checkout for a paid tier ──

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CheckoutQuery {
    /// Target tier. One of: pro, business, scale.
    #[param(example = "pro")]
    pub tier: String,
}

fn parse_paid_tier(s: &str) -> Option<PlanTier> {
    match s {
        "pro" => Some(PlanTier::Pro),
        "business" => Some(PlanTier::Business),
        "scale" => Some(PlanTier::Scale),
        _ => None,
    }
}

#[utoipa::path(
    post,
    path = "/v1/billing/stripe/checkout",
    tag = "Billing",
    params(CheckoutQuery),
    responses(
        (status = 200, description = "Stripe Checkout session created", body = CheckoutSessionResponse),
        (status = 400, description = "Invalid tier", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid API key", body = crate::error::ErrorResponse),
        (status = 503, description = "Stripe not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn create_stripe_checkout(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Query(q): Query<CheckoutQuery>,
) -> Response {
    let Some(tier) = parse_paid_tier(&q.tier) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "tier must be one of pro, business, scale",
                "code": "invalid_tier"
            })),
        )
            .into_response();
    };

    let cfg = crate::services::billing::stripe_client::StripeConfig {
        secret_key: state.config.stripe_secret_key.clone(),
        price_id_pro: state.config.stripe_price_id_pro.clone(),
        price_id_business: state.config.stripe_price_id_business.clone(),
        price_id_scale: state.config.stripe_price_id_scale.clone(),
        success_url: state.config.stripe_success_url.clone(),
        cancel_url: state.config.stripe_cancel_url.clone(),
    };

    match create_checkout_session(&cfg, tier, &tenant.0.to_hex()).await {
        Ok(session) => (
            StatusCode::OK,
            Json(CheckoutSessionResponse {
                checkout_url: session.url,
            }),
        )
            .into_response(),
        Err(StripeError::NotConfigured) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Stripe is not configured on this server",
                "code": "stripe_not_configured"
            })),
        )
            .into_response(),
        Err(StripeError::MissingPriceId(_)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Missing Stripe price ID for requested tier",
                "code": "stripe_missing_price_id"
            })),
        )
            .into_response(),
        Err(e @ (StripeError::Api(_) | StripeError::Network(_))) => {
            tracing::error!(error = %e, "stripe_checkout_create_failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to create Stripe Checkout session",
                    "code": "stripe_api_error"
                })),
            )
                .into_response()
        }
    }
}

// ── POST /v1/billing/magic-link — public, rate-limited ──

fn extract_client_ip(headers: &HeaderMap) -> String {
    // In production we run behind Fly's proxy which sets X-Forwarded-For.
    // Local dev without the header falls back to a single shared bucket,
    // which is fine for a dev machine.
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "local".to_string())
}

#[utoipa::path(
    post,
    path = "/v1/billing/magic-link",
    tag = "Billing",
    request_body = MagicLinkRequest,
    responses(
        (status = 200, description = "Always returned (prevents email enumeration)", body = MagicLinkResponse),
        (status = 400, description = "Invalid intent or missing tier", body = crate::error::ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = crate::error::ErrorResponse),
        (status = 503, description = "Billing not configured", body = crate::error::ErrorResponse),
    ),
)]
#[tracing::instrument(skip(state, headers, body))]
pub async fn create_magic_link(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<MagicLinkRequest>,
) -> Response {
    let Some(service) = state.billing_handoff_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Billing not configured", "code": "no_billing" })),
        )
            .into_response();
    };

    let client_ip = extract_client_ip(&headers);
    match service
        .request(&body.email, &body.intent, body.tier.as_deref(), &client_ip)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(MagicLinkResponse { status: "sent" })).into_response(),
        Err(BillingHandoffError::RateLimited) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Too many magic-link requests. Try again later.",
                "code": "rate_limited"
            })),
        )
            .into_response(),
        Err(BillingHandoffError::Invalid { code, message }) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": message, "code": code })),
        )
            .into_response(),
    }
}

// ── GET /v1/billing/go — redeem magic link → Stripe redirect ──

#[derive(Debug, Deserialize)]
pub struct MagicLinkGoQuery {
    pub token: String,
}

#[tracing::instrument(skip(state))]
pub async fn redeem_magic_link(
    State(state): State<Arc<AppState>>,
    Query(q): Query<MagicLinkGoQuery>,
) -> Response {
    // `/pricing` doesn't exist as a standalone route on the marketing site —
    // it's an anchor on the home page.
    let expired_url = format!("{}/?error=link_expired#pricing", state.config.marketing_url);
    let no_subscription_url = format!(
        "{}/manage?error=no_subscription",
        state.config.marketing_url
    );

    let Some(service) = state.billing_handoff_service.as_ref() else {
        return Redirect::to(&expired_url).into_response();
    };

    match service.redeem(&q.token).await {
        HandoffOutcome::CheckoutUrl(url) | HandoffOutcome::PortalUrl(url) => {
            Redirect::to(&url).into_response()
        }
        HandoffOutcome::NoSubscription => Redirect::to(&no_subscription_url).into_response(),
        HandoffOutcome::Expired => Redirect::to(&expired_url).into_response(),
    }
}

// ── POST /v1/billing/stripe/portal — Stripe Billing Portal for current tenant ──

#[utoipa::path(
    post,
    path = "/v1/billing/stripe/portal",
    tag = "Billing",
    responses(
        (status = 200, description = "Stripe Billing Portal session created", body = PortalSessionResponse),
        (status = 400, description = "Tenant has no Stripe customer", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid API key", body = crate::error::ErrorResponse),
        (status = 503, description = "Stripe not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn create_stripe_portal(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(tenants) = state.tenants_repo.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let tenant_doc = match tenants.find_by_id(&tenant.0).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Tenant not found", "code": "tenant_not_found" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "billing_portal_tenant_fetch_failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    let Some(customer_id) = tenant_doc.stripe_customer_id.as_deref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Tenant has no Stripe customer. Subscribe first.",
                "code": "no_customer"
            })),
        )
            .into_response();
    };

    let return_url = format!("{}/manage", state.config.marketing_url);
    match create_portal_session(&state.config.stripe_secret_key, customer_id, &return_url).await {
        Ok(session) => (
            StatusCode::OK,
            Json(PortalSessionResponse {
                portal_url: session.url,
            }),
        )
            .into_response(),
        Err(StripeError::NotConfigured) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Stripe is not configured on this server",
                "code": "stripe_not_configured"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "stripe_portal_create_failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to create Stripe Billing Portal session",
                    "code": "stripe_api_error"
                })),
            )
                .into_response()
        }
    }
}

// ── POST /v1/billing/cancel — cancel at current_period_end ──

#[utoipa::path(
    post,
    path = "/v1/billing/cancel",
    tag = "Billing",
    responses(
        (status = 200, description = "Cancellation scheduled for period end"),
        (status = 400, description = "Tenant has no cancellable subscription", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid API key", body = crate::error::ErrorResponse),
        (status = 503, description = "Stripe not configured", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn cancel_subscription(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(billing) = state.billing_service.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Billing service not configured", "code": "no_billing" })),
        )
            .into_response();
    };

    let status = match billing.status(&tenant.0).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "billing_cancel_status_fetch_failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    match status.billing_method {
        BillingMethod::Stripe => {
            // Stripe path: schedule cancel-at-period-end via the API. The
            // webhook handler will flip plan_tier to Free when the period
            // actually ends.
            let Some(tenants) = state.tenants_repo.as_ref() else {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({ "error": "Database not configured", "code": "no_database" })),
                )
                    .into_response();
            };
            let tenant_doc = match tenants.find_by_id(&tenant.0).await {
                Ok(Some(t)) => t,
                Ok(None) => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!({
                            "error": "Tenant not found",
                            "code": "tenant_not_found"
                        })),
                    )
                        .into_response();
                }
                Err(e) => {
                    tracing::error!(error = %e, "billing_cancel_tenant_fetch_failed");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Internal error", "code": "db_error" })),
                    )
                        .into_response();
                }
            };
            let Some(sub_id) = tenant_doc.stripe_subscription_id.as_deref() else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Tenant has no Stripe subscription to cancel",
                        "code": "no_subscription"
                    })),
                )
                    .into_response();
            };

            match cancel_subscription_at_period_end(&state.config.stripe_secret_key, sub_id).await
            {
                Ok(()) => (
                    StatusCode::OK,
                    Json(json!({
                        "status": "cancel_scheduled",
                        "current_period_end": status.current_period_end.map(|d| d.timestamp_millis()),
                    })),
                )
                    .into_response(),
                Err(StripeError::NotConfigured) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "error": "Stripe is not configured on this server",
                        "code": "stripe_not_configured"
                    })),
                )
                    .into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "stripe_cancel_failed");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Failed to cancel Stripe subscription",
                            "code": "stripe_api_error"
                        })),
                    )
                        .into_response()
                }
            }
        }
        BillingMethod::X402 => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "x402 subscriptions end naturally at period end — just don't renew",
                "code": "x402_no_cancel"
            })),
        )
            .into_response(),
        BillingMethod::Free => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "No paid subscription to cancel",
                "code": "not_subscribed"
            })),
        )
            .into_response(),
    }
}

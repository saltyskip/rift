//! Stripe webhook receiver for billing lifecycle events.
//!
//! Flow:
//!   1. Verify the `Stripe-Signature` header against the raw request body
//!      using our `whsec_` secret. Anything malformed or stale → 400.
//!   2. Dedup on `event.id` via `stripe_webhook_dedup`. A duplicate means
//!      Stripe is retrying / fanning out — ack with 200 and do nothing.
//!   3. Route the event type to a handler that updates the tenant via
//!      TenantsRepository::{apply_subscription_update, clear_subscription}.
//!
//! Events handled: `customer.subscription.created | updated | deleted`,
//! `invoice.payment_failed`, `invoice.paid`. Anything else is ACK'd with
//! 200 and logged at INFO.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::{self, oid::ObjectId};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::app::AppState;
use crate::services::auth::tenants::repo::{
    BillingMethod, PlanTier, SubscriptionStatus, SubscriptionUpdate,
};
use crate::services::billing::stripe_client::{verify_webhook_signature, WebhookVerifyError};

// ── Inbound event envelope ──

#[derive(Debug, Deserialize)]
struct StripeEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    data: StripeEventData,
}

#[derive(Debug, Deserialize)]
struct StripeEventData {
    object: Value,
}

// ── Payload subsets we care about ──

#[derive(Debug, Deserialize)]
struct StripeSubscription {
    id: String,
    customer: String,
    status: String,
    #[serde(default)]
    current_period_start: Option<i64>,
    #[serde(default)]
    current_period_end: Option<i64>,
    items: StripeItems,
    #[serde(default)]
    metadata: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct StripeItems {
    data: Vec<StripeItem>,
}

#[derive(Debug, Deserialize)]
struct StripeItem {
    price: StripePrice,
    // Stripe API versions from 2025 dropped `current_period_{start,end}` from
    // the top-level subscription and moved them onto the individual items.
    // Fall back to the first item's copy when the top-level field is None.
    #[serde(default)]
    current_period_start: Option<i64>,
    #[serde(default)]
    current_period_end: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct StripePrice {
    id: String,
}

#[derive(Debug, Deserialize)]
struct StripeInvoice {
    #[serde(default)]
    customer: Option<String>,
    #[serde(default)]
    subscription: Option<String>,
    #[serde(default)]
    billing_reason: Option<String>,
}

// ── Tenant resolution ──

fn tenant_id_from_metadata(meta: &serde_json::Map<String, Value>) -> Option<ObjectId> {
    meta.get("tenant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| ObjectId::parse_str(s).ok())
}

fn price_id_to_tier(state: &AppState, price_id: &str) -> Option<PlanTier> {
    if price_id == state.config.stripe_price_id_pro {
        Some(PlanTier::Pro)
    } else if price_id == state.config.stripe_price_id_business {
        Some(PlanTier::Business)
    } else if price_id == state.config.stripe_price_id_scale {
        Some(PlanTier::Scale)
    } else {
        None
    }
}

fn map_status(stripe_status: &str) -> SubscriptionStatus {
    match stripe_status {
        "active" | "trialing" => SubscriptionStatus::Active,
        "past_due" | "unpaid" => SubscriptionStatus::PastDue,
        "canceled" | "incomplete_expired" | "incomplete" => SubscriptionStatus::Canceled,
        _ => SubscriptionStatus::Active,
    }
}

fn secs_to_bson_dt(secs: i64) -> bson::DateTime {
    bson::DateTime::from_millis(secs.saturating_mul(1000))
}

// ── Route handler ──
//
// Deliberately NOT annotated with `#[utoipa::path]`. The body is raw bytes
// (for HMAC verification), which utoipa can't derive a schema for. Stripe
// documents the webhook payload on their end; we just ACK it.

#[tracing::instrument(skip(state, headers, body))]
pub async fn receive_stripe_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let secret = &state.config.stripe_webhook_secret;
    if secret.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Stripe webhook not configured", "code": "no_webhook_secret" })),
        )
            .into_response();
    }
    let (Some(tenants_repo), Some(dedup)) = (
        state.tenants_repo.as_ref(),
        state.stripe_webhook_dedup.as_ref(),
    ) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(header_value) = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Missing Stripe-Signature", "code": "missing_signature" })),
        )
            .into_response();
    };

    let now = chrono::Utc::now().timestamp();
    if let Err(e) = verify_webhook_signature(secret, header_value, &body, now) {
        tracing::warn!(error = %e, "stripe_webhook_signature_rejected");
        return match e {
            WebhookVerifyError::TimestampTooOld => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "timestamp outside tolerance", "code": "stale_webhook" })),
            )
                .into_response(),
            _ => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "bad signature", "code": "bad_signature" })),
            )
                .into_response(),
        };
    }

    let event: StripeEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "stripe_webhook_parse_failed");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid webhook payload", "code": "bad_payload" })),
            )
                .into_response();
        }
    };

    // Idempotency. If we've seen this event.id before, ack and no-op.
    match dedup.mark_processed(&event.id).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!(event_id = %event.id, "stripe_webhook_duplicate_ignored");
            return StatusCode::OK.into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "stripe_webhook_dedup_db_error");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    }

    let apply_result = match event.event_type.as_str() {
        "customer.subscription.created" | "customer.subscription.updated" => {
            handle_subscription_upsert(&state, tenants_repo.as_ref(), event.data.object).await
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(tenants_repo.as_ref(), event.data.object).await
        }
        "invoice.payment_failed" => {
            handle_invoice_status(
                tenants_repo.as_ref(),
                event.data.object,
                SubscriptionStatus::PastDue,
            )
            .await
        }
        "invoice.paid" => {
            handle_invoice_status(
                tenants_repo.as_ref(),
                event.data.object,
                SubscriptionStatus::Active,
            )
            .await
        }
        other => {
            tracing::info!(event_type = %other, "stripe_webhook_event_ignored");
            Ok(())
        }
    };

    match apply_result {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!(error = %e, event_type = %event.event_type, "stripe_webhook_apply_failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "apply_failed" })),
            )
                .into_response()
        }
    }
}

async fn handle_subscription_upsert(
    state: &AppState,
    tenants: &dyn crate::services::auth::tenants::repo::TenantsRepository,
    object: Value,
) -> Result<(), String> {
    let sub: StripeSubscription = serde_json::from_value(object).map_err(|e| e.to_string())?;

    let first_item = sub.items.data.first();
    let price_id = first_item.map(|i| i.price.id.as_str()).unwrap_or_default();

    let plan_tier = price_id_to_tier(state, price_id);
    if plan_tier.is_none() {
        tracing::warn!(price_id, "stripe_webhook_unknown_price_id");
    }

    let period_start = sub
        .current_period_start
        .or_else(|| first_item.and_then(|i| i.current_period_start));
    let period_end = sub
        .current_period_end
        .or_else(|| first_item.and_then(|i| i.current_period_end));

    // Resolve (or materialize) the tenant. Fast path: metadata.tenant_id or
    // existing stripe_customer_id. Fallback: pending_email from the
    // magic-link flow, which means this is the first webhook for a
    // brand-new customer — we create the tenant, owner user, and rl_live_
    // key here, then email the key to the customer.
    let (tenant_id, newly_minted_key) =
        match try_resolve_tenant(tenants, &sub.metadata, &sub.customer).await? {
            Some(id) => (id, None),
            None => {
                let Some(pending_email) =
                    sub.metadata.get("pending_email").and_then(|v| v.as_str())
                else {
                    return Err(format!(
                        "no tenant mapping for customer {} and no pending_email metadata",
                        sub.customer
                    ));
                };
                let (tid, key_info) =
                    materialize_tenant_from_pending_email(state, pending_email).await?;
                (tid, key_info)
            }
        };

    let update = SubscriptionUpdate {
        plan_tier,
        billing_method: Some(BillingMethod::Stripe),
        status: Some(map_status(&sub.status)),
        current_period_start: period_start.map(secs_to_bson_dt),
        current_period_end: period_end.map(secs_to_bson_dt),
        stripe_customer_id: Some(sub.customer),
        stripe_subscription_id: Some(sub.id),
    };

    tenants
        .apply_subscription_update(&tenant_id, update)
        .await?;

    // Send welcome email *after* the subscription is persisted. Failure to
    // email is logged but doesn't re-queue the webhook — the webhook already
    // succeeded at its primary job (tenant + subscription state). Customers
    // can always use the magic-link portal flow to recover.
    if let Some((email, api_key)) = newly_minted_key {
        let billing_tier = match plan_tier {
            Some(crate::services::auth::tenants::repo::PlanTier::Pro) => {
                Some(crate::services::billing::handoff::BillingTier::Pro)
            }
            Some(crate::services::auth::tenants::repo::PlanTier::Business) => {
                Some(crate::services::billing::handoff::BillingTier::Business)
            }
            Some(crate::services::auth::tenants::repo::PlanTier::Scale) => {
                Some(crate::services::billing::handoff::BillingTier::Scale)
            }
            _ => None,
        };
        if let Some(billing_tier) = billing_tier {
            if let Err(e) = crate::services::billing::email::send_welcome(
                &state.config.resend_api_key,
                &state.config.resend_from_email,
                &email,
                &api_key,
                billing_tier,
                &state.config.marketing_url,
            )
            .await
            {
                tracing::error!(error = %e, email = %email, "welcome_email_send_failed");
            }
        }
    }

    Ok(())
}

/// Create a fresh tenant for a customer that completed Checkout without ever
/// having an account on our side. Returns `(tenant_id, Some((email,
/// api_key)))` so the caller can email the customer their freshly-minted key.
async fn materialize_tenant_from_pending_email(
    state: &AppState,
    pending_email: &str,
) -> Result<(ObjectId, Option<(String, String)>), String> {
    let users_service = state
        .users_service
        .as_ref()
        .ok_or("users_service not configured")?;
    let sk_repo = state
        .secret_keys_repo
        .as_ref()
        .ok_or("secret_keys_repo not configured")?;

    let (tenant_id, user_id) = users_service
        .create_tenant_with_verified_owner(pending_email)
        .await
        .map_err(|e| format!("tenant materialize failed: {e}"))?;

    let created = crate::services::auth::secret_keys::service::mint_for_tenant(
        sk_repo.as_ref(),
        tenant_id,
        user_id,
    )
    .await?;

    Ok((tenant_id, Some((pending_email.to_string(), created.key))))
}

async fn handle_subscription_deleted(
    tenants: &dyn crate::services::auth::tenants::repo::TenantsRepository,
    object: Value,
) -> Result<(), String> {
    let sub: StripeSubscription = serde_json::from_value(object).map_err(|e| e.to_string())?;
    let Some(tenant_id) = try_resolve_tenant(tenants, &sub.metadata, &sub.customer).await? else {
        // No tenant mapping for a subscription delete means we never fully
        // materialized a tenant (webhook failed before the welcome path).
        // Nothing to clean up.
        tracing::warn!(customer = %sub.customer, "stripe_webhook_deleted_no_tenant");
        return Ok(());
    };
    tenants.clear_subscription(&tenant_id).await?;
    Ok(())
}

async fn handle_invoice_status(
    tenants: &dyn crate::services::auth::tenants::repo::TenantsRepository,
    object: Value,
    new_status: SubscriptionStatus,
) -> Result<(), String> {
    let invoice: StripeInvoice = serde_json::from_value(object).map_err(|e| e.to_string())?;

    // Only apply to subscription invoices — ignore one-off invoices.
    if invoice.subscription.is_none() {
        return Ok(());
    }
    // Also skip subscription_create invoices — the subscription.created event
    // already sets status correctly, and the invoice.paid fires immediately
    // after which would clobber a PastDue → Active we didn't set.
    if invoice.billing_reason.as_deref() == Some("subscription_create") {
        return Ok(());
    }

    let Some(customer) = invoice.customer else {
        return Ok(());
    };
    let tenant = tenants.find_by_stripe_customer_id(&customer).await?;
    let Some(tenant) = tenant else {
        tracing::warn!(customer, "stripe_webhook_invoice_tenant_not_found");
        return Ok(());
    };
    let Some(tenant_id) = tenant.id else {
        return Ok(());
    };

    let update = SubscriptionUpdate {
        status: Some(new_status),
        ..SubscriptionUpdate::default()
    };
    tenants
        .apply_subscription_update(&tenant_id, update)
        .await?;
    Ok(())
}

/// Resolve a tenant for a Stripe subscription event. Returns `None` if the
/// event references a customer we don't yet know about — the caller decides
/// whether that's a fatal error (subscription.deleted) or a signal to create
/// a new tenant via the pending_email path (subscription.created/updated).
async fn try_resolve_tenant(
    tenants: &dyn crate::services::auth::tenants::repo::TenantsRepository,
    metadata: &serde_json::Map<String, Value>,
    customer_id: &str,
) -> Result<Option<ObjectId>, String> {
    if let Some(id) = tenant_id_from_metadata(metadata) {
        return Ok(Some(id));
    }
    let found = tenants.find_by_stripe_customer_id(customer_id).await?;
    Ok(found.and_then(|t| t.id))
}

pub mod affiliates;
pub mod apps;
pub mod auth;
pub mod billing;
pub mod conversions;
pub mod domains;
pub mod health;
pub mod links;
pub mod webhooks;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use utoipa::OpenApi;

use crate::app::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rift API",
        version = "0.1.0",
        description = "Deep links for humans and agents.\n\nRift creates smart, cross-platform deep links with structured context for AI agents. Each link carries per-platform destinations, metadata, and an optional `agent_context` that makes links machine-readable.\n\n## Authentication\n\nSign up at `POST /v1/auth/signup` to get an API key (starts with `rl_live_`). Include it as a Bearer token in the Authorization header. Client SDKs use publishable keys (`pk_live_`) for click tracking and attribution.\n\n## Quick start\n\n1. `POST /v1/auth/signup` — get an API key\n2. `POST /v1/links` — create your first link\n3. `GET /r/{link_id}` — resolve it (redirect for browsers, JSON for agents)\n\n## Content negotiation\n\nThe resolve endpoint (`GET /r/{link_id}`) is content-negotiated. Browsers receive a redirect or landing page. Requests with `Accept: application/json` receive structured link data including `agent_context` and a `_rift_meta` trust envelope.",
        contact(name = "Rift", url = "https://riftl.ink"),
    ),
    paths(
        // Authentication — signup, verify, key management, team management
        auth::secret_keys::routes::signup,
        auth::secret_keys::routes::verify_email,
        auth::secret_keys::routes::request_create_key,
        auth::secret_keys::routes::confirm_create_key,
        auth::secret_keys::routes::list_secret_keys,
        auth::secret_keys::routes::delete_secret_key,
        auth::publishable_keys::routes::create_sdk_key,
        auth::publishable_keys::routes::list_sdk_keys,
        auth::publishable_keys::routes::revoke_sdk_key,
        auth::users::routes::invite_user,
        auth::users::routes::list_users,
        auth::users::routes::delete_user,
        // Domains — custom domain setup
        domains::routes::create_domain,
        domains::routes::list_domains,
        domains::routes::delete_domain,
        domains::routes::verify_domain,
        // Apps — app configuration and association files
        apps::routes::create_app,
        apps::routes::list_apps,
        apps::routes::delete_app,
        apps::routes::serve_aasa,
        apps::routes::serve_assetlinks,
        // Links — create, list, update, delete, resolve
        links::routes::create_link,
        links::routes::create_links_bulk,
        links::routes::get_link,
        links::routes::list_links,
        links::routes::get_link_stats,
        links::routes::get_link_qr,
        links::routes::update_link,
        links::routes::delete_link,
        links::routes::resolve_link,
        links::routes::resolve_link_custom,
        // Attribution — click tracking, install reporting, timeseries
        links::routes::attribution_click,
        links::routes::attribution_report,
        links::routes::link_attribution,
        links::routes::get_link_timeseries,
        // Webhooks — event notifications
        webhooks::routes::create_webhook,
        webhooks::routes::list_webhooks,
        webhooks::routes::delete_webhook,
        webhooks::routes::patch_webhook,
        // Affiliates — partner CRUD + scoped credentials (postback dispatch
        // ships in a follow-up; v1 unblocks partner-side link minting).
        affiliates::routes::create_affiliate,
        affiliates::routes::list_affiliates,
        affiliates::routes::get_affiliate,
        affiliates::routes::patch_affiliate,
        affiliates::routes::delete_affiliate,
        affiliates::routes::create_affiliate_credential,
        affiliates::routes::list_affiliate_credentials,
        affiliates::routes::revoke_affiliate_credential,
        // Conversions — sources, webhook ingestion, and SDK tracking
        // (receive_webhook is intentionally excluded — opaque parser-specific body)
        conversions::routes::create_source,
        conversions::routes::list_sources,
        conversions::routes::get_source,
        conversions::routes::delete_source,
        conversions::routes::sdk_track_conversion,
        // Billing — plan status, Stripe checkout, webhooks
        billing::routes::get_billing_status,
        billing::routes::create_stripe_checkout,
        billing::routes::create_stripe_portal,
        billing::routes::cancel_subscription,
        billing::routes::create_magic_link,
        // (Stripe webhook omitted — raw-body handler, documented on Stripe's end)
        // System
        health::routes::health,
    ),
    components(schemas(
        health::models::HealthResponse,
        crate::error::ErrorResponse,
        auth::secret_keys::models::SignupRequest,
        auth::secret_keys::models::SignupResponse,
        crate::services::links::models::CreateLinkRequest,
        crate::services::links::models::CreateLinkResponse,
        crate::services::links::models::BulkCreateLinksRequest,
        crate::services::links::models::BulkCreateLinksResponse,
        crate::services::links::models::BulkLinkTemplate,
        crate::services::links::models::BulkLinkResult,
        crate::services::links::models::BatchItemError,
        crate::services::links::models::UpdateLinkRequest,
        crate::services::links::models::LinkDetail,
        crate::services::links::models::ListLinksResponse,
        crate::services::links::models::LinkStatsResponse,
        crate::services::links::models::ResolvedLink,
        crate::services::links::models::RiftMeta,
        crate::services::links::models::ClickRequest,
        crate::services::links::models::AttributionReportRequest,
        crate::services::links::models::LinkAttributionRequest,
        crate::services::links::models::AttributionResponse,
        crate::services::links::models::AgentContext,
        crate::services::links::models::SocialPreview,
        crate::services::links::models::TimeseriesDataPoint,
        crate::services::links::models::TimeseriesResponse,
        auth::secret_keys::models::RequestCreateKeyRequest,
        auth::secret_keys::models::ConfirmCreateKeyRequest,
        auth::secret_keys::models::CreateKeyResponse,
        auth::secret_keys::models::SecretKeyDetail,
        auth::secret_keys::models::ListSecretKeysResponse,
        auth::users::models::InviteUserRequest,
        auth::users::models::InviteUserResponse,
        auth::users::models::UserDetail,
        auth::users::models::ListUsersResponse,
        crate::services::auth::publishable_keys::models::CreateSdkKeyRequest,
        crate::services::auth::publishable_keys::models::CreateSdkKeyResponse,
        crate::services::auth::publishable_keys::models::SdkKeyDetail,
        crate::services::auth::publishable_keys::models::ListSdkKeysResponse,
        crate::services::domains::models::CreateDomainRequest,
        crate::services::domains::models::CreateDomainResponse,
        crate::services::domains::models::DomainDetail,
        crate::services::domains::models::VerifyDomainResponse,
        crate::services::apps::models::CreateAppRequest,
        crate::services::apps::models::AppDetail,
        crate::services::webhooks::models::CreateWebhookRequest,
        crate::services::webhooks::models::CreateWebhookResponse,
        crate::services::webhooks::models::WebhookDetail,
        crate::services::webhooks::models::ListWebhooksResponse,
        crate::services::webhooks::models::UpdateWebhookRequest,
        crate::services::webhooks::models::WebhookEventType,
        crate::services::affiliates::models::CreateAffiliateRequest,
        crate::services::affiliates::models::AffiliateDetail,
        crate::services::affiliates::models::ListAffiliatesResponse,
        crate::services::affiliates::models::UpdateAffiliateRequest,
        crate::services::affiliates::models::AffiliateStatus,
        crate::services::affiliates::models::CreateAffiliateCredentialResponse,
        crate::services::affiliates::models::AffiliateCredentialDetail,
        crate::services::affiliates::models::ListAffiliateCredentialsResponse,
        crate::services::conversions::models::SourceType,
        crate::services::conversions::models::CreateSourceRequest,
        crate::services::conversions::models::CreateSourceResponse,
        crate::services::conversions::models::SourceDetail,
        crate::api::conversions::models::SdkConversionRequest,
        crate::services::conversions::models::ListSourcesResponse,
        crate::services::conversions::models::ConversionDetail,
        crate::api::billing::models::BillingStatusResponse,
        crate::api::billing::models::LimitsView,
        crate::api::billing::models::CheckoutSessionResponse,
        crate::api::billing::models::PortalSessionResponse,
        crate::api::billing::models::MagicLinkRequest,
        crate::api::billing::models::MagicLinkResponse,
        crate::services::auth::tenants::repo::PlanTier,
        crate::services::auth::tenants::repo::BillingMethod,
        crate::services::auth::tenants::repo::SubscriptionStatus,
    )),
    security(
        ("api_key" = []),
        ("x402" = []),
    ),
    tags(
        (name = "Signup", description = "Account registration and email verification"),
        (name = "Secret Keys", description = "Server-side API key management (rl_live_)"),
        (name = "Publishable Keys", description = "Client-safe SDK key management (pk_live_)"),
        (name = "Team Members", description = "Invite and manage users on your team"),
        (name = "Domains", description = "Custom domain setup and verification"),
        (name = "Apps", description = "App configuration and association files (AASA / Asset Links)"),
        (name = "Links", description = "Create, list, update, delete, and resolve deep links"),
        (name = "Attribution", description = "Click tracking, install reporting, and timeseries analytics"),
        (name = "Webhooks", description = "Real-time event notifications for clicks, attributions, and conversions"),
        (name = "Conversions", description = "Backend-only conversion tracking via webhook sources"),
        (name = "Affiliates", description = "Named partners with scoped credentials that mint links pinned to their affiliate"),
        (name = "Billing", description = "Plan status, upgrades, and subscription lifecycle"),
        (name = "System", description = "Health checks and operational endpoints"),
    )
)]
struct ApiDoc;

async fn serve_rift_js() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../sdk/rift.js"),
    )
}

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let mut spec = ApiDoc::openapi();

    // Add x-tagGroups for nested sidebar in Scalar
    spec.extensions = Some(
        utoipa::openapi::extensions::ExtensionsBuilder::new()
            .add(
                "x-tagGroups",
                serde_json::json!([
                    { "name": "Authentication", "tags": ["Signup", "Secret Keys", "Publishable Keys", "Team Members"] },
                    { "name": "Configuration", "tags": ["Domains", "Apps"] },
                    { "name": "Links", "tags": ["Links", "Attribution"] },
                    { "name": "Integrations", "tags": ["Webhooks", "Conversions", "Affiliates"] },
                    { "name": "Billing", "tags": ["Billing"] },
                    { "name": "System", "tags": ["System"] },
                ]),
            )
            .build(),
    );

    let openapi_json = Router::new().route(
        "/openapi.json",
        get(move || async move { axum::Json(spec) }),
    );

    let sdk = Router::new().route("/sdk/rift.js", get(serve_rift_js));

    health::router()
        .merge(auth::router(state.clone()))
        .merge(links::router(state.clone()))
        .merge(domains::router(state.clone()))
        .merge(apps::router(state.clone()))
        .merge(webhooks::router(state.clone()))
        .merge(conversions::router(state.clone()))
        .merge(affiliates::router(state.clone()))
        .merge(billing::router(state.clone()))
        .merge(openapi_json)
        .merge(sdk)
}

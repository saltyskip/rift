use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;
use std::sync::Arc;

use super::models::*;
use crate::api::auth::middleware::TenantId;
use crate::api::AppState;

// ── POST /v1/domains — Register a custom domain ──

#[utoipa::path(
    post,
    path = "/v1/domains",
    tag = "Domains",
    request_body = CreateDomainRequest,
    responses(
        (status = 201, description = "Domain registered", body = CreateDomainResponse),
        (status = 400, description = "Invalid domain", body = crate::error::ErrorResponse),
        (status = 409, description = "Domain already registered", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_domain(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<CreateDomainRequest>,
) -> Response {
    let Some(repo) = &state.domains_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let domain = req.domain.trim().to_lowercase();

    if let Err(e) = validate_domain(&domain, &state.config.primary_domain) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e, "code": "invalid_domain" })),
        )
            .into_response();
    }

    // Check if already registered.
    if repo.find_by_domain(&domain).await.ok().flatten().is_some() {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "Domain already registered", "code": "domain_taken" })),
        )
            .into_response();
    }

    let token = generate_verification_token();
    let created = match repo
        .create_domain(tenant.0, domain.clone(), token.clone())
        .await
    {
        Ok(d) => d,
        Err(e) if e.to_string().contains("E11000") => {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "Domain already registered", "code": "domain_taken" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to create domain: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    };

    let txt_record = format!("_rift-verify.{domain}");
    let resp = CreateDomainResponse {
        domain: created.domain,
        verified: created.verified,
        verification_token: token,
        txt_record,
        cname_target: state.config.primary_domain.clone(),
    };

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(resp).unwrap()),
    )
        .into_response()
}

// ── GET /v1/domains — List tenant's custom domains ──

#[utoipa::path(
    get,
    path = "/v1/domains",
    tag = "Domains",
    responses(
        (status = 200, description = "List of domains", body = Vec<DomainDetail>),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_domains(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
) -> Response {
    let Some(repo) = &state.domains_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match repo.list_by_tenant(&tenant.0).await {
        Ok(domains) => {
            let details: Vec<DomainDetail> = domains
                .iter()
                .map(|d| DomainDetail {
                    domain: d.domain.clone(),
                    verified: d.verified,
                    created_at: d.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(json!({ "domains": details })).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list domains: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── DELETE /v1/domains/{domain} — Remove a custom domain ──

#[utoipa::path(
    delete,
    path = "/v1/domains/{domain}",
    tag = "Domains",
    params(("domain" = String, Path, description = "Domain to delete")),
    responses(
        (status = 204, description = "Domain deleted"),
        (status = 404, description = "Domain not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn delete_domain(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(domain): Path<String>,
) -> Response {
    let Some(repo) = &state.domains_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    match repo.delete_domain(&tenant.0, &domain).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Domain not found", "code": "not_found" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete domain: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response()
        }
    }
}

// ── POST /v1/domains/{domain}/verify — Verify DNS ownership ──

#[utoipa::path(
    post,
    path = "/v1/domains/{domain}/verify",
    tag = "Domains",
    params(("domain" = String, Path, description = "Domain to verify")),
    responses(
        (status = 200, description = "Verification result", body = VerifyDomainResponse),
        (status = 404, description = "Domain not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn verify_domain(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(domain): Path<String>,
) -> Response {
    let Some(repo) = &state.domains_repo else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Database not configured", "code": "no_database" })),
        )
            .into_response();
    };

    let Some(existing) = repo.find_by_domain(&domain).await.ok().flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Domain not found", "code": "not_found" })),
        )
            .into_response();
    };

    if existing.tenant_id != tenant.0 {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Domain not found", "code": "not_found" })),
        )
            .into_response();
    }

    if existing.verified {
        return Json(json!({ "domain": domain, "verified": true })).into_response();
    }

    // Query DNS TXT record via dig.
    let txt_host = format!("_rift-verify.{domain}");
    let verified = match tokio::process::Command::new("dig")
        .args(["+short", "TXT", &txt_host])
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .any(|line| line.trim().trim_matches('"') == existing.verification_token)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to run dig for DNS verification");
            false
        }
    };

    if verified {
        if let Err(e) = repo.mark_verified(&domain).await {
            tracing::error!("Failed to mark domain verified: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal error", "code": "db_error" })),
            )
                .into_response();
        }
    }

    Json(json!({ "domain": domain, "verified": verified })).into_response()
}

// ── Helpers ──

fn generate_verification_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    hex::encode(bytes)
}

fn validate_domain(domain: &str, primary_domain: &str) -> Result<(), String> {
    if domain.is_empty() || domain.len() > 253 {
        return Err("Domain must be 1-253 characters".to_string());
    }
    if !domain.contains('.') {
        return Err("Domain must contain at least one dot".to_string());
    }
    if domain.contains("://") || domain.contains('/') || domain.contains(':') {
        return Err("Domain must not contain protocol, path, or port".to_string());
    }
    if domain == primary_domain {
        return Err(format!(
            "Cannot register the primary domain '{primary_domain}'"
        ));
    }
    // Basic label validation.
    for label in domain.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err("Each domain label must be 1-63 characters".to_string());
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Domain labels must be alphanumeric with hyphens only".to_string());
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err("Domain labels must not start or end with a hyphen".to_string());
        }
    }
    Ok(())
}

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::models::AuthKeyId;
use crate::app::AppState;
use crate::core::public_id::AffiliateId;
use crate::services::affiliates::models::*;
use crate::services::auth::permissions::AuthContext;

// ── Affiliate CRUD ──

#[utoipa::path(
    post,
    path = "/v1/affiliates",
    tag = "Affiliates",
    request_body = CreateAffiliateRequest,
    responses(
        (status = 201, description = "Affiliate created", body = AffiliateDetail),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 409, description = "partner_key already taken", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx, req))]
pub async fn create_affiliate(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Json(req): Json<CreateAffiliateRequest>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc.create_affiliate(&ctx, req.name, req.partner_key).await {
        Ok(a) => (StatusCode::CREATED, Json(to_detail(&a))).into_response(),
        Err(e) => affiliate_error_to_response(e),
    }
}

#[utoipa::path(
    get,
    path = "/v1/affiliates",
    tag = "Affiliates",
    responses(
        (status = 200, description = "List of affiliates", body = ListAffiliatesResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn list_affiliates(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc.list_affiliates(&ctx).await {
        Ok(list) => Json(ListAffiliatesResponse {
            affiliates: list.iter().map(to_detail).collect(),
        })
        .into_response(),
        Err(e) => affiliate_error_to_response(e),
    }
}

#[utoipa::path(
    get,
    path = "/v1/affiliates/{affiliate_id}",
    tag = "Affiliates",
    params(("affiliate_id" = AffiliateId, Path, description = "Affiliate id")),
    responses(
        (status = 200, description = "Affiliate detail", body = AffiliateDetail),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn get_affiliate(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(affiliate_id): Path<AffiliateId>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc.get_affiliate(&ctx, affiliate_id).await {
        Ok(a) => Json(to_detail(&a)).into_response(),
        Err(e) => affiliate_error_to_response(e),
    }
}

#[utoipa::path(
    patch,
    path = "/v1/affiliates/{affiliate_id}",
    tag = "Affiliates",
    params(("affiliate_id" = AffiliateId, Path, description = "Affiliate id")),
    request_body = UpdateAffiliateRequest,
    responses(
        (status = 200, description = "Affiliate updated", body = AffiliateDetail),
        (status = 400, description = "Empty body", body = crate::error::ErrorResponse),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx, req))]
pub async fn patch_affiliate(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(affiliate_id): Path<AffiliateId>,
    Json(req): Json<UpdateAffiliateRequest>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc.update_affiliate(&ctx, affiliate_id, req).await {
        Ok(a) => Json(to_detail(&a)).into_response(),
        Err(e) => affiliate_error_to_response(e),
    }
}

#[utoipa::path(
    delete,
    path = "/v1/affiliates/{affiliate_id}",
    tag = "Affiliates",
    params(("affiliate_id" = AffiliateId, Path, description = "Affiliate id")),
    responses(
        (status = 204, description = "Affiliate deleted"),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn delete_affiliate(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(affiliate_id): Path<AffiliateId>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc.delete_affiliate(&ctx, affiliate_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => affiliate_error_to_response(e),
    }
}

// ── Affiliate credentials (scoped rl_live_… keys) ──

// Mint a new partner-scoped rl_live_… API key for this affiliate.
// The key is returned ONCE in `api_key`. Store it out-of-band — Rift never
// reveals it again. The caller's key must have full tenant scope; affiliate
// keys cannot mint additional credentials.
#[utoipa::path(
    post,
    path = "/v1/affiliates/{affiliate_id}/credentials",
    tag = "Affiliates",
    params(("affiliate_id" = String, Path, description = "Affiliate ObjectId")),
    responses(
        (status = 201, description = "Credential minted; api_key shown once", body = CreateAffiliateCredentialResponse),
        (status = 403, description = "Caller scope cannot mint credentials", body = crate::error::ErrorResponse),
        (status = 404, description = "Affiliate not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn create_affiliate_credential(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    axum::Extension(auth_key): axum::Extension<AuthKeyId>,
    Path(affiliate_id): Path<AffiliateId>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc
        .mint_credential(&ctx, affiliate_id, auth_key.0.to_object_id())
        .await
    {
        Ok(minted) => (
            StatusCode::CREATED,
            Json(CreateAffiliateCredentialResponse {
                id: minted.created_key.id.to_string(),
                affiliate_id: minted.affiliate_id,
                api_key: minted.created_key.key,
                key_prefix: minted.created_key.key_prefix,
                created_at: minted
                    .created_key
                    .created_at
                    .try_to_rfc3339_string()
                    .unwrap_or_default(),
            }),
        )
            .into_response(),
        Err(e) => affiliate_error_to_response(e),
    }
}

#[utoipa::path(
    get,
    path = "/v1/affiliates/{affiliate_id}/credentials",
    tag = "Affiliates",
    params(("affiliate_id" = String, Path, description = "Affiliate ObjectId")),
    responses(
        (status = 200, description = "List of credentials (no raw secrets)", body = ListAffiliateCredentialsResponse),
        (status = 404, description = "Affiliate not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn list_affiliate_credentials(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path(affiliate_id): Path<AffiliateId>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc.list_credentials(&ctx, affiliate_id).await {
        Ok(keys) => {
            let creds: Vec<AffiliateCredentialDetail> = keys
                .into_iter()
                .map(|k| AffiliateCredentialDetail {
                    id: k.id.to_string(),
                    key_prefix: k.key_prefix,
                    created_at: k.created_at.try_to_rfc3339_string().unwrap_or_default(),
                })
                .collect();
            Json(ListAffiliateCredentialsResponse { credentials: creds }).into_response()
        }
        Err(e) => affiliate_error_to_response(e),
    }
}

#[utoipa::path(
    delete,
    path = "/v1/affiliates/{affiliate_id}/credentials/{key_id}",
    tag = "Affiliates",
    params(
        ("affiliate_id" = String, Path, description = "Affiliate ObjectId"),
        ("key_id" = String, Path, description = "Credential ObjectId"),
    ),
    responses(
        (status = 204, description = "Credential revoked"),
        (status = 404, description = "Not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = [])),
)]
#[tracing::instrument(skip(state, ctx))]
pub async fn revoke_affiliate_credential(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
    Path((affiliate_id, key_id)): Path<(AffiliateId, crate::core::public_id::SecretKeyId)>,
) -> Response {
    let Some(svc) = &state.affiliates_service else {
        return no_database();
    };

    match svc
        .revoke_credential(&ctx, affiliate_id, key_id.to_object_id())
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => affiliate_error_to_response(e),
    }
}

// ── Helpers ──

fn to_detail(a: &Affiliate) -> AffiliateDetail {
    AffiliateDetail {
        id: a.id,
        name: a.name.clone(),
        partner_key: a.partner_key.clone(),
        status: a.status,
        created_at: a.created_at.try_to_rfc3339_string().unwrap_or_default(),
        updated_at: a.updated_at.try_to_rfc3339_string().unwrap_or_default(),
    }
}

fn affiliate_error_to_response(err: AffiliateError) -> Response {
    match err {
        AffiliateError::QuotaExceeded(q) => {
            return crate::api::billing::quota_response::to_response(q);
        }
        AffiliateError::Forbidden(authz) => {
            return crate::api::auth::forbidden_response::to_response(authz);
        }
        _ => {}
    }
    let status = match &err {
        AffiliateError::InvalidName(_)
        | AffiliateError::InvalidPartnerKey(_)
        | AffiliateError::EmptyUpdate => StatusCode::BAD_REQUEST,
        AffiliateError::PartnerKeyTaken(_) | AffiliateError::CredentialLimit => {
            StatusCode::CONFLICT
        }
        AffiliateError::NotFound | AffiliateError::CredentialNotFound => StatusCode::NOT_FOUND,
        AffiliateError::QuotaExceeded(_) | AffiliateError::Forbidden(_) => {
            unreachable!("handled above")
        }
        AffiliateError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let code = err.code();
    let message = err.to_string();
    (status, Json(json!({ "error": message, "code": code }))).into_response()
}

fn no_database() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Database not configured", "code": "no_database" })),
    )
        .into_response()
}

fn invalid_id() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "Invalid ID", "code": "invalid_id" })),
    )
        .into_response()
}

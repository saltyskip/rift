use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use mongodb::bson::oid::ObjectId;
use serde_json::json;
use std::sync::Arc;

use super::models::*;
use crate::api::auth::middleware::TenantId;
use crate::api::AppState;
use crate::core::validation;

#[utoipa::path(
    post,
    path = "/v1/themes",
    tag = "Themes",
    request_body = CreateThemeRequest,
    responses(
        (status = 201, description = "Theme created", body = ThemeDetail),
        (status = 400, description = "Invalid theme", body = crate::error::ErrorResponse),
        (status = 409, description = "Theme slug already exists", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn create_theme(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Json(req): Json<CreateThemeRequest>,
) -> Response {
    let Some(repo) = &state.themes_repo else {
        return service_unavailable();
    };

    if let Err(e) = validation::validate_theme_request(
        &req.name,
        &req.slug,
        &req.tokens,
        &req.copy,
        &req.media,
        &req.seo,
    ) {
        return bad_request("invalid_theme", e);
    }

    if repo
        .find_by_tenant_and_slug(&tenant.0, &req.slug)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "Theme slug already exists", "code": "theme_slug_taken" })),
        )
            .into_response();
    }

    if req.is_default {
        if let Err(e) = repo.clear_default_for_tenant(&tenant.0, None).await {
            tracing::error!("Failed to clear default themes: {e}");
            return db_error();
        }
    }

    let now = mongodb::bson::DateTime::now();
    let theme = LandingTheme {
        id: ObjectId::new(),
        tenant_id: tenant.0,
        name: req.name,
        slug: req.slug,
        is_default: req.is_default,
        status: ThemeStatus::Active,
        tokens: req.tokens,
        copy: req.copy,
        media: req.media,
        layout: req.layout,
        modules: req.modules,
        seo: req.seo,
        created_at: now,
        updated_at: now,
    };

    match repo.create_theme(theme).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(to_detail(&created)).unwrap()),
        )
            .into_response(),
        Err(e) if e.contains("E11000") => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "Theme slug already exists", "code": "theme_slug_taken" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to create theme: {e}");
            db_error()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/themes",
    tag = "Themes",
    params(ListThemesQuery),
    responses((status = 200, description = "List of themes", body = ListThemesResponse)),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn list_themes(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Query(query): Query<ListThemesQuery>,
) -> Response {
    let Some(repo) = &state.themes_repo else {
        return service_unavailable();
    };

    match repo
        .list_by_tenant(&tenant.0, query.status.as_deref())
        .await
    {
        Ok(themes) => Json(json!({
            "themes": themes.iter().map(to_detail).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to list themes: {e}");
            db_error()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/themes/{theme_id}",
    tag = "Themes",
    params(("theme_id" = String, Path, description = "Theme ID")),
    responses(
        (status = 200, description = "Theme detail", body = ThemeDetail),
        (status = 404, description = "Theme not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn get_theme(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(theme_id): Path<String>,
) -> Response {
    let Some(repo) = &state.themes_repo else {
        return service_unavailable();
    };

    let Ok(theme_id) = ObjectId::parse_str(&theme_id) else {
        return bad_request("invalid_id", "Invalid theme_id");
    };

    match repo.find_by_tenant_and_id(&tenant.0, &theme_id).await {
        Ok(Some(theme)) => Json(serde_json::to_value(to_detail(&theme)).unwrap()).into_response(),
        Ok(None) => not_found(),
        Err(e) => {
            tracing::error!("Failed to fetch theme: {e}");
            db_error()
        }
    }
}

#[utoipa::path(
    patch,
    path = "/v1/themes/{theme_id}",
    tag = "Themes",
    params(("theme_id" = String, Path, description = "Theme ID")),
    request_body = UpdateThemeRequest,
    responses(
        (status = 200, description = "Theme updated", body = ThemeDetail),
        (status = 404, description = "Theme not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state, req))]
pub async fn update_theme(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(theme_id): Path<String>,
    Json(req): Json<UpdateThemeRequest>,
) -> Response {
    let Some(repo) = &state.themes_repo else {
        return service_unavailable();
    };

    let Ok(theme_id) = ObjectId::parse_str(&theme_id) else {
        return bad_request("invalid_id", "Invalid theme_id");
    };

    let Some(mut theme) = repo
        .find_by_tenant_and_id(&tenant.0, &theme_id)
        .await
        .ok()
        .flatten()
    else {
        return not_found();
    };

    if let Some(name) = req.name {
        theme.name = name;
    }
    if let Some(slug) = req.slug {
        theme.slug = slug;
    }
    if let Some(is_default) = req.is_default {
        theme.is_default = is_default;
    }
    if let Some(status) = req.status {
        theme.status = status;
    }
    if let Some(tokens) = req.tokens {
        theme.tokens = tokens;
    }
    if let Some(copy) = req.copy {
        theme.copy = copy;
    }
    if let Some(media) = req.media {
        theme.media = media;
    }
    if let Some(layout) = req.layout {
        theme.layout = layout;
    }
    if let Some(modules) = req.modules {
        theme.modules = modules;
    }
    if let Some(seo) = req.seo {
        theme.seo = seo;
    }

    if let Err(e) = validation::validate_theme_request(
        &theme.name,
        &theme.slug,
        &theme.tokens,
        &theme.copy,
        &theme.media,
        &theme.seo,
    ) {
        return bad_request("invalid_theme", e);
    }

    if let Ok(Some(existing)) = repo.find_by_tenant_and_slug(&tenant.0, &theme.slug).await {
        if existing.id != theme.id {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "Theme slug already exists", "code": "theme_slug_taken" })),
            )
                .into_response();
        }
    }

    if matches!(theme.status, ThemeStatus::Archived)
        && theme_is_referenced(&state, &tenant.0, &theme.id).await
    {
        return bad_request(
            "theme_in_use",
            "Themes in use by domains or links cannot be archived",
        );
    }

    if theme.is_default {
        if let Err(e) = repo
            .clear_default_for_tenant(&tenant.0, Some(&theme.id))
            .await
        {
            tracing::error!("Failed to clear default themes: {e}");
            return db_error();
        }
    } else if repo
        .find_default_by_tenant(&tenant.0)
        .await
        .ok()
        .flatten()
        .is_some_and(|default| default.id == theme.id)
    {
        return bad_request(
            "default_theme_required",
            "Use another theme as default before unsetting the current default",
        );
    }

    theme.updated_at = mongodb::bson::DateTime::now();

    match repo.replace_theme(theme).await {
        Ok(updated) => Json(serde_json::to_value(to_detail(&updated)).unwrap()).into_response(),
        Err(e) => {
            tracing::error!("Failed to update theme: {e}");
            db_error()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/v1/themes/{theme_id}",
    tag = "Themes",
    params(("theme_id" = String, Path, description = "Theme ID")),
    responses(
        (status = 204, description = "Theme deleted"),
        (status = 404, description = "Theme not found", body = crate::error::ErrorResponse),
    ),
    security(("api_key" = []), ("x402" = [])),
)]
#[tracing::instrument(skip(state))]
pub async fn delete_theme(
    State(state): State<Arc<AppState>>,
    axum::Extension(tenant): axum::Extension<TenantId>,
    Path(theme_id): Path<String>,
) -> Response {
    let Some(repo) = &state.themes_repo else {
        return service_unavailable();
    };

    let Ok(theme_id) = ObjectId::parse_str(&theme_id) else {
        return bad_request("invalid_id", "Invalid theme_id");
    };

    let Some(theme) = repo
        .find_by_tenant_and_id(&tenant.0, &theme_id)
        .await
        .ok()
        .flatten()
    else {
        return not_found();
    };

    if theme.is_default {
        return bad_request(
            "default_theme_required",
            "Select a different default theme before deleting this one",
        );
    }

    if theme_is_referenced(&state, &tenant.0, &theme.id).await {
        return bad_request(
            "theme_in_use",
            "Themes in use by domains or links cannot be deleted",
        );
    }

    match repo.delete_theme(&tenant.0, &theme_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => not_found(),
        Err(e) => {
            tracing::error!("Failed to delete theme: {e}");
            db_error()
        }
    }
}

async fn theme_is_referenced(
    state: &Arc<AppState>,
    tenant_id: &ObjectId,
    theme_id: &ObjectId,
) -> bool {
    let domain_count = match &state.domains_repo {
        Some(repo) => repo.count_by_theme(tenant_id, theme_id).await.unwrap_or(0),
        None => 0,
    };
    let link_count = match &state.links_repo {
        Some(repo) => repo.count_by_theme(tenant_id, theme_id).await.unwrap_or(0),
        None => 0,
    };
    domain_count > 0 || link_count > 0
}

fn to_detail(theme: &LandingTheme) -> ThemeDetail {
    ThemeDetail {
        id: theme.id.to_hex(),
        name: theme.name.clone(),
        slug: theme.slug.clone(),
        is_default: theme.is_default,
        status: theme.status.clone(),
        tokens: theme.tokens.clone(),
        copy: theme.copy.clone(),
        media: theme.media.clone(),
        layout: theme.layout.clone(),
        modules: theme.modules.clone(),
        seo: theme.seo.clone(),
        created_at: theme.created_at.try_to_rfc3339_string().unwrap_or_default(),
        updated_at: theme.updated_at.try_to_rfc3339_string().unwrap_or_default(),
    }
}

fn service_unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Database not configured", "code": "no_database" })),
    )
        .into_response()
}

fn db_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "Internal error", "code": "db_error" })),
    )
        .into_response()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "Theme not found", "code": "not_found" })),
    )
        .into_response()
}

fn bad_request(code: &str, error: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": error.into(), "code": code })),
    )
        .into_response()
}

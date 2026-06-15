use std::sync::Arc;

use super::models::TenantError;
use super::repo::{TenantDoc, TenantsRepository};
use crate::core::public_id::TenantId;
use crate::core::validation::{validate_cta, validate_hex_color, validate_web_url};
use crate::services::landing::models::LandingTheme;

const MAX_BRAND_NAME_LEN: usize = 80;
const MAX_TAGLINE_LEN: usize = 200;

crate::impl_container!(TenantsService);
/// Tenant lifecycle primitives shared across signup (and later billing / agent)
/// flows. Keeps credential-specific concerns (email owners, wallet credentials,
/// secret keys) out of the tenant layer.
pub struct TenantsService {
    tenants_repo: Arc<dyn TenantsRepository>,
}

impl TenantsService {
    pub fn new(tenants_repo: Arc<dyn TenantsRepository>) -> Self {
        Self { tenants_repo }
    }

    /// Create a bare tenant with default limits and return its id. Callers are
    /// responsible for attaching an owner (email user, wallet credential, etc.)
    /// immediately after.
    pub async fn create_blank(&self) -> Result<crate::core::public_id::TenantId, String> {
        let id = crate::core::public_id::TenantId::new();
        let doc = TenantDoc {
            id: Some(id),
            ..TenantDoc::default()
        };
        self.tenants_repo.create(&doc).await?;
        Ok(id)
    }

    /// Validate and persist a tenant's landing-page branding (full replace).
    /// Both transports share this entry point, so validation lives here, not in
    /// a route handler.
    pub async fn update_landing_theme(
        &self,
        tenant_id: &TenantId,
        theme: LandingTheme,
    ) -> Result<(), TenantError> {
        validate_landing_theme(&theme).map_err(TenantError::Invalid)?;
        self.tenants_repo
            .set_landing_theme(tenant_id, &theme)
            .await
            .map_err(TenantError::Storage)?;
        Ok(())
    }

    /// Read a tenant's branding, falling back to Rift defaults when unset.
    pub async fn get_landing_theme(
        &self,
        tenant_id: &TenantId,
    ) -> Result<LandingTheme, TenantError> {
        let theme = self
            .tenants_repo
            .find_by_id(tenant_id)
            .await
            .map_err(TenantError::Storage)?
            .and_then(|t| t.landing_theme)
            .unwrap_or_default();
        Ok(theme)
    }
}

// ── Helpers ──

/// Validate the constrained free-text/URL/color fields of a theme. Enum fields
/// are type-checked at deserialize, so they need no runtime validation.
fn validate_landing_theme(theme: &LandingTheme) -> Result<(), String> {
    if let Some(color) = &theme.theme_color {
        validate_hex_color(color).map_err(|e| format!("theme_color: {e}"))?;
    }
    if let Some(url) = &theme.icon_url {
        validate_web_url(url).map_err(|e| format!("icon_url: {e}"))?;
    }
    if let Some(url) = &theme.logo_url {
        validate_web_url(url).map_err(|e| format!("logo_url: {e}"))?;
    }
    if let Some(cta) = &theme.cta_label {
        validate_cta(cta).map_err(|e| format!("cta_label: {e}"))?;
    }
    if theme
        .brand_name
        .as_deref()
        .is_some_and(|s| s.len() > MAX_BRAND_NAME_LEN)
    {
        return Err(format!(
            "brand_name must be {MAX_BRAND_NAME_LEN} characters or fewer"
        ));
    }
    if theme
        .tagline
        .as_deref()
        .is_some_and(|s| s.len() > MAX_TAGLINE_LEN)
    {
        return Err(format!(
            "tagline must be {MAX_TAGLINE_LEN} characters or fewer"
        ));
    }
    Ok(())
}

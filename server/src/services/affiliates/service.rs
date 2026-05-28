use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use rift_macros::requires;
use std::sync::Arc;

use super::models::{
    Affiliate, AffiliateError, AffiliateStatus, MintedCredential, UpdateAffiliateRequest,
    MAX_CREDENTIALS_PER_AFFILIATE,
};
use super::repo::AffiliatesRepository;
use crate::core::public_id::AffiliateId;
use crate::services::auth::permissions::{AuthContext, Permission};
use crate::services::auth::secret_keys::repo::{KeyScope, SecretKeysRepository};
use crate::services::auth::secret_keys::service::mint_scoped;
use crate::services::billing::quota::{QuotaChecker, Resource};

// ── Service ──

crate::impl_container!(AffiliatesService);
pub struct AffiliatesService {
    repo: Arc<dyn AffiliatesRepository>,
    secret_keys_repo: Arc<dyn SecretKeysRepository>,
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl AffiliatesService {
    pub fn new(
        repo: Arc<dyn AffiliatesRepository>,
        secret_keys_repo: Arc<dyn SecretKeysRepository>,
        quota: Option<Arc<dyn QuotaChecker>>,
    ) -> Self {
        Self {
            repo,
            secret_keys_repo,
            quota,
        }
    }

    /// Create an affiliate. Quota-checked + partner_key uniqueness enforced.
    /// Caller must carry `AffiliatesWrite` (only full-tenant keys do today).
    #[requires(Permission::AffiliatesWrite)]
    pub async fn create_affiliate(
        &self,
        ctx: &AuthContext,
        name: String,
        partner_key: String,
    ) -> Result<Affiliate, AffiliateError> {
        validate_name(&name)?;
        validate_partner_key(&partner_key)?;

        if let Some(q) = &self.quota {
            q.check(&ctx.tenant_id, Resource::CreateAffiliate).await?;
        }

        // Pre-check uniqueness for a clean error before hitting the DB write.
        // We still rely on the unique compound index as the race-safe
        // backstop — see the E11000 catch below.
        if self
            .repo
            .find_by_partner_key(&ctx.tenant_id.into(), &partner_key)
            .await
            .map_err(AffiliateError::Internal)?
            .is_some()
        {
            return Err(AffiliateError::PartnerKeyTaken(partner_key));
        }

        let now = DateTime::now();
        let affiliate = Affiliate {
            id: AffiliateId::new(),
            tenant_id: ctx.tenant_id.into(),
            name,
            partner_key: partner_key.clone(),
            status: AffiliateStatus::Active,
            created_at: now,
            updated_at: now,
        };

        self.repo.create_affiliate(&affiliate).await.map_err(|e| {
            if e.contains("E11000") {
                AffiliateError::PartnerKeyTaken(partner_key)
            } else {
                AffiliateError::Internal(e)
            }
        })?;

        Ok(affiliate)
    }

    #[requires(Permission::AffiliatesWrite)]
    pub async fn get_affiliate(
        &self,
        ctx: &AuthContext,
        affiliate_id: AffiliateId,
    ) -> Result<Affiliate, AffiliateError> {
        self.repo
            .get_by_id(&ctx.tenant_id.into(), &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)
    }

    #[requires(Permission::AffiliatesWrite)]
    pub async fn list_affiliates(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Affiliate>, AffiliateError> {
        self.repo
            .list_by_tenant(&ctx.tenant_id.into())
            .await
            .map_err(AffiliateError::Internal)
    }

    #[requires(Permission::AffiliatesWrite)]
    pub async fn update_affiliate(
        &self,
        ctx: &AuthContext,
        affiliate_id: AffiliateId,
        req: UpdateAffiliateRequest,
    ) -> Result<Affiliate, AffiliateError> {
        if req.name.is_none() && req.status.is_none() {
            return Err(AffiliateError::EmptyUpdate);
        }
        if let Some(ref n) = req.name {
            validate_name(n)?;
        }

        let now = DateTime::now();
        let updated = self
            .repo
            .update_affiliate(
                &ctx.tenant_id.into(),
                &affiliate_id,
                req.name.as_deref(),
                req.status,
                now,
            )
            .await
            .map_err(AffiliateError::Internal)?;

        if !updated {
            return Err(AffiliateError::NotFound);
        }

        // Re-fetch so the response carries the persisted values (including
        // the new updated_at).
        self.repo
            .get_by_id(&ctx.tenant_id.into(), &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)
    }

    /// Mint a new partner-scoped credential. Caller must carry `AffiliatesWrite`.
    /// Per-affiliate cap of `MAX_CREDENTIALS_PER_AFFILIATE` enforced.
    #[requires(Permission::AffiliatesWrite)]
    pub async fn mint_credential(
        &self,
        ctx: &AuthContext,
        affiliate_id: ObjectId,
        created_by: ObjectId,
    ) -> Result<MintedCredential, AffiliateError> {
        // Affiliate must exist in this tenant.
        self.repo
            .get_by_id(&ctx.tenant_id.into(), &affiliate_id.into())
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)?;

        // Per-affiliate cap. Counted at the repo level via the scope filter
        // so a compromised tenant key can't spam unbounded credentials.
        let existing = self
            .secret_keys_repo
            .list_by_tenant_and_affiliate(&ctx.tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?;
        if existing.len() >= MAX_CREDENTIALS_PER_AFFILIATE {
            return Err(AffiliateError::CredentialLimit);
        }

        let created_key = mint_scoped(
            self.secret_keys_repo.as_ref(),
            ctx.tenant_id,
            created_by,
            KeyScope::Affiliate { affiliate_id },
        )
        .await
        .map_err(AffiliateError::Internal)?;

        Ok(MintedCredential {
            created_key,
            affiliate_id: affiliate_id.into(),
        })
    }

    /// List credentials scoped to an affiliate. Caller must carry
    /// `AffiliatesWrite`. Returns the raw `SecretKeyDoc`s; routes are
    /// responsible for projecting to a no-secret DTO.
    #[requires(Permission::AffiliatesWrite)]
    pub async fn list_credentials(
        &self,
        ctx: &AuthContext,
        affiliate_id: ObjectId,
    ) -> Result<Vec<crate::services::auth::secret_keys::repo::SecretKeyDoc>, AffiliateError> {
        // Affiliate must exist (404 vs empty list — different semantics).
        self.repo
            .get_by_id(&ctx.tenant_id.into(), &affiliate_id.into())
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)?;

        self.secret_keys_repo
            .list_by_tenant_and_affiliate(&ctx.tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)
    }

    /// Revoke a partner credential. Caller must carry `AffiliatesWrite`.
    /// Atomic at the repo level — no TOCTOU between membership check and delete.
    #[requires(Permission::AffiliatesWrite)]
    pub async fn revoke_credential(
        &self,
        ctx: &AuthContext,
        affiliate_id: ObjectId,
        key_id: ObjectId,
    ) -> Result<(), AffiliateError> {
        // Surface affiliate-not-found distinctly from credential-not-found.
        self.repo
            .get_by_id(&ctx.tenant_id.into(), &affiliate_id.into())
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)?;

        let deleted = self
            .secret_keys_repo
            .delete_affiliate_credential(&ctx.tenant_id, &affiliate_id, &key_id)
            .await
            .map_err(AffiliateError::Internal)?;

        if !deleted {
            return Err(AffiliateError::CredentialNotFound);
        }
        Ok(())
    }

    /// Hard delete.
    ///
    /// TODO(affiliate-link-refs): When the dispatcher lands, decide whether
    /// to (a) reject delete if any `Link.affiliate_id` or scoped `SecretKey`
    /// references this affiliate, or (b) cascade-revoke credentials and
    /// orphan the link refs. v1 leaves orphan refs (inert until dispatch).
    #[requires(Permission::AffiliatesWrite)]
    pub async fn delete_affiliate(
        &self,
        ctx: &AuthContext,
        affiliate_id: AffiliateId,
    ) -> Result<(), AffiliateError> {
        let deleted = self
            .repo
            .delete_affiliate(&ctx.tenant_id.into(), &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?;

        if !deleted {
            return Err(AffiliateError::NotFound);
        }
        Ok(())
    }
}

// ── Validation ──

fn validate_name(name: &str) -> Result<(), AffiliateError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AffiliateError::InvalidName("name cannot be empty".into()));
    }
    if trimmed.chars().count() > 64 {
        return Err(AffiliateError::InvalidName(
            "name must be 64 characters or fewer".into(),
        ));
    }
    Ok(())
}

fn validate_partner_key(key: &str) -> Result<(), AffiliateError> {
    let len = key.chars().count();
    if !(2..=32).contains(&len) {
        return Err(AffiliateError::InvalidPartnerKey(
            "partner_key must be 2-32 characters".into(),
        ));
    }

    let bytes = key.as_bytes();
    let valid_char = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
    if !bytes.iter().copied().all(valid_char) {
        return Err(AffiliateError::InvalidPartnerKey(
            "partner_key may only contain lowercase letters, digits, and '-'".into(),
        ));
    }
    if bytes[0] == b'-' || *bytes.last().unwrap() == b'-' {
        return Err(AffiliateError::InvalidPartnerKey(
            "partner_key must not start or end with '-'".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

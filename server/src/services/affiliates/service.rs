use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use std::sync::Arc;

use super::models::{
    Affiliate, AffiliateError, AffiliateStatus, MintedCredential, UpdateAffiliateRequest,
    MAX_CREDENTIALS_PER_AFFILIATE,
};
use super::repo::AffiliatesRepository;
use crate::services::auth::scope::require_full;
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
    /// Caller must have full tenant scope.
    pub async fn create_affiliate(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        name: String,
        partner_key: String,
    ) -> Result<Affiliate, AffiliateError> {
        require_full(caller_scope)?;
        validate_name(&name)?;
        validate_partner_key(&partner_key)?;

        if let Some(q) = &self.quota {
            q.check(&tenant_id, Resource::CreateAffiliate).await?;
        }

        // Pre-check uniqueness for a clean error before hitting the DB write.
        // We still rely on the unique compound index as the race-safe
        // backstop — see the E11000 catch below.
        if self
            .repo
            .find_by_partner_key(&tenant_id, &partner_key)
            .await
            .map_err(AffiliateError::Internal)?
            .is_some()
        {
            return Err(AffiliateError::PartnerKeyTaken(partner_key));
        }

        let now = DateTime::now();
        let affiliate = Affiliate {
            id: ObjectId::new(),
            tenant_id,
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

    pub async fn get_affiliate(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        affiliate_id: ObjectId,
    ) -> Result<Affiliate, AffiliateError> {
        require_full(caller_scope)?;
        self.repo
            .get_by_id(&tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)
    }

    pub async fn list_affiliates(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
    ) -> Result<Vec<Affiliate>, AffiliateError> {
        require_full(caller_scope)?;
        self.repo
            .list_by_tenant(&tenant_id)
            .await
            .map_err(AffiliateError::Internal)
    }

    pub async fn update_affiliate(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        affiliate_id: ObjectId,
        req: UpdateAffiliateRequest,
    ) -> Result<Affiliate, AffiliateError> {
        require_full(caller_scope)?;

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
                &tenant_id,
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
            .get_by_id(&tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)
    }

    /// Mint a new partner-scoped credential. Caller must have full scope.
    /// Per-affiliate cap of `MAX_CREDENTIALS_PER_AFFILIATE` enforced.
    pub async fn mint_credential(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        affiliate_id: ObjectId,
        created_by: ObjectId,
    ) -> Result<MintedCredential, AffiliateError> {
        require_full(caller_scope)?;

        // Affiliate must exist in this tenant.
        self.repo
            .get_by_id(&tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)?;

        // Per-affiliate cap. Counted at the repo level via the scope filter
        // so a compromised tenant key can't spam unbounded credentials.
        let existing = self
            .secret_keys_repo
            .list_by_tenant_and_affiliate(&tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?;
        if existing.len() >= MAX_CREDENTIALS_PER_AFFILIATE {
            return Err(AffiliateError::CredentialLimit);
        }

        let created_key = mint_scoped(
            self.secret_keys_repo.as_ref(),
            tenant_id,
            created_by,
            KeyScope::Affiliate { affiliate_id },
        )
        .await
        .map_err(AffiliateError::Internal)?;

        Ok(MintedCredential {
            created_key,
            affiliate_id,
        })
    }

    /// List credentials scoped to an affiliate. Caller must have full scope.
    /// Returns the raw `SecretKeyDoc`s; routes are responsible for projecting
    /// to a no-secret DTO.
    pub async fn list_credentials(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        affiliate_id: ObjectId,
    ) -> Result<Vec<crate::services::auth::secret_keys::repo::SecretKeyDoc>, AffiliateError> {
        require_full(caller_scope)?;

        // Affiliate must exist (404 vs empty list — different semantics).
        self.repo
            .get_by_id(&tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)?;

        self.secret_keys_repo
            .list_by_tenant_and_affiliate(&tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)
    }

    /// Revoke a partner credential. Caller must have full scope. Atomic at
    /// the repo level — no TOCTOU between membership check and delete.
    pub async fn revoke_credential(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        affiliate_id: ObjectId,
        key_id: ObjectId,
    ) -> Result<(), AffiliateError> {
        require_full(caller_scope)?;

        // Surface affiliate-not-found distinctly from credential-not-found.
        self.repo
            .get_by_id(&tenant_id, &affiliate_id)
            .await
            .map_err(AffiliateError::Internal)?
            .ok_or(AffiliateError::NotFound)?;

        let deleted = self
            .secret_keys_repo
            .delete_affiliate_credential(&tenant_id, &affiliate_id, &key_id)
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
    pub async fn delete_affiliate(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        affiliate_id: ObjectId,
    ) -> Result<(), AffiliateError> {
        require_full(caller_scope)?;

        let deleted = self
            .repo
            .delete_affiliate(&tenant_id, &affiliate_id)
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

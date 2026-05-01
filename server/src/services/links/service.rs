use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use std::sync::Arc;
use uuid::Uuid;

use super::models::*;
use super::repo::LinksRepository;
use crate::core::threat_feed::ThreatFeed;
use crate::core::validation;
use crate::services::affiliates::repo::AffiliatesRepository;
use crate::services::auth::secret_keys::repo::KeyScope;
use crate::services::billing::quota::{QuotaChecker, Resource};
use crate::services::billing::service::TierResolver;
use crate::services::domains::models::DomainRole;
use crate::services::domains::repo::DomainsRepository;

// ── Service ──

/// Maximum number of links per `POST /v1/links/bulk` request. Caps memory,
/// threat-feed fan-out, and the size of any single transaction.
pub const BULK_LINKS_MAX: usize = 100;

pub struct LinksService {
    links_repo: Arc<dyn LinksRepository>,
    domains_repo: Option<Arc<dyn DomainsRepository>>,
    /// Optional so the test harness and reduced-feature builds can boot
    /// without an affiliates collection. Affiliate-scoped credentials and
    /// explicit `affiliate_id` values require it; absence is treated as
    /// "no affiliates configured" → `AffiliateNotFound` for explicit values
    /// and a graceful pass-through for unscoped/no-affiliate creates.
    affiliates_repo: Option<Arc<dyn AffiliatesRepository>>,
    threat_feed: ThreatFeed,
    public_url: String,
    quota: Option<Arc<dyn QuotaChecker>>,
    tiers: Option<Arc<dyn TierResolver>>,
}

impl LinksService {
    pub fn new(
        links_repo: Arc<dyn LinksRepository>,
        domains_repo: Option<Arc<dyn DomainsRepository>>,
        affiliates_repo: Option<Arc<dyn AffiliatesRepository>>,
        threat_feed: ThreatFeed,
        public_url: String,
        quota: Option<Arc<dyn QuotaChecker>>,
        tiers: Option<Arc<dyn TierResolver>>,
    ) -> Self {
        Self {
            links_repo,
            domains_repo,
            affiliates_repo,
            threat_feed,
            public_url,
            quota,
            tiers,
        }
    }

    /// Fire-and-forget click recording. Runs the `TrackEvent` quota check
    /// (which also increments the atomic counter), resolves the tenant's
    /// retention bucket, then persists. Failures are logged, never
    /// propagated — the caller (public redirect path) must not break on a
    /// DB hiccup or a quota rejection. The counter's conditional `$inc`
    /// already keeps over-limit tenants from spilling past their cap.
    ///
    /// Lives here (not in route handlers) because both the public resolver
    /// and the SDK click endpoint record clicks, and a future MCP / CLI
    /// transport must not be able to bypass quota by inventing its own
    /// path to `LinksRepository::record_click`.
    pub async fn record_click(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        user_agent: Option<String>,
        referer: Option<String>,
        platform: Option<String>,
    ) {
        if let Some(q) = &self.quota {
            if let Err(e) = q.check(&tenant_id, Resource::TrackEvent).await {
                tracing::info!(error = %e, "click_track_quota_skipped");
            }
        }

        let retention_bucket = match &self.tiers {
            Some(b) => b.retention_bucket_for_tenant(&tenant_id).await.to_string(),
            None => "30d".to_string(),
        };

        if let Err(e) = self
            .links_repo
            .record_click(
                tenant_id,
                link_id,
                user_agent,
                referer,
                platform,
                retention_bucket,
            )
            .await
        {
            tracing::warn!(error = %e, "Failed to record click");
        }
    }

    #[tracing::instrument(skip(self, req, caller_scope))]
    pub async fn create_link(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        req: CreateLinkRequest,
    ) -> Result<CreateLinkResponse, LinkError> {
        // Quota enforcement lives here (service layer) so MCP tool invocations
        // and HTTP route handlers both hit the same choke point. CLAUDE.md
        // codifies this rule — see "Quota enforcement" section there.
        if let Some(q) = &self.quota {
            q.check(&tenant_id, Resource::CreateLink).await?;
        }

        // Resolve `affiliate_id` against the caller's scope.
        //
        // | caller_scope        | req.affiliate_id     | result                      |
        // |---------------------|----------------------|-----------------------------|
        // | Affiliate(A)        | None                 | pin to A                    |
        // | Affiliate(A)        | Some(A)              | pin to A                    |
        // | Affiliate(A)        | Some(B) where B != A | AffiliateScopeMismatch (400)|
        // | Full / None (grand) | Some(B)              | validate B; pin to B        |
        // | Full / None (grand) | None                 | None (existing behaviour)   |
        let resolved_affiliate_id = self
            .resolve_affiliate_id(&tenant_id, caller_scope, req.affiliate_id)
            .await?;

        let has_verified_domain = self.tenant_has_verified_domain(&tenant_id).await;

        let link_id = match &req.custom_id {
            Some(custom) => {
                validate_custom_id(custom)?;

                if !has_verified_domain {
                    return Err(LinkError::NoVerifiedDomain);
                }

                if self
                    .links_repo
                    .find_link_by_tenant_and_id(&tenant_id, custom)
                    .await
                    .ok()
                    .flatten()
                    .is_some()
                {
                    return Err(LinkError::LinkIdTaken(custom.clone()));
                }
                custom.clone()
            }
            None => generate_link_id(),
        };

        validate_link_urls(
            req.web_url.as_deref(),
            req.ios_deep_link.as_deref(),
            req.android_deep_link.as_deref(),
            req.ios_store_url.as_deref(),
            req.android_store_url.as_deref(),
        )?;

        if let Some(ref web_url) = req.web_url {
            if let Some(reason) = self.threat_feed.check_url(web_url).await {
                return Err(LinkError::ThreatDetected(reason));
            }
        }

        validate_agent_context(&req.agent_context)?;
        validate_social_preview(&req.social_preview)?;

        if let Some(ref meta) = req.metadata {
            validation::validate_metadata(meta).map_err(LinkError::InvalidMetadata)?;
        }

        let metadata = req
            .metadata
            .and_then(|v| mongodb::bson::to_document(&v).ok());

        let mut input = CreateLinkInput::new(tenant_id, link_id.clone());
        if let Some(v) = req.ios_deep_link {
            input = input.ios_deep_link(v);
        }
        if let Some(v) = req.android_deep_link {
            input = input.android_deep_link(v);
        }
        if let Some(v) = req.web_url {
            input = input.web_url(v);
        }
        if let Some(v) = req.ios_store_url {
            input = input.ios_store_url(v);
        }
        if let Some(v) = req.android_store_url {
            input = input.android_store_url(v);
        }
        if let Some(v) = metadata {
            input = input.metadata(v);
        }
        if let Some(aff) = resolved_affiliate_id {
            input = input.affiliate_id(aff);
        }
        if let Some(ac) = req.agent_context {
            input = input.agent_context(ac);
        }
        if let Some(sp) = req.social_preview {
            input = input.social_preview(sp);
        }

        // Links without a verified custom domain expire after 30 days.
        let expires_at = if !has_verified_domain {
            let thirty_days_ms = 30 * 24 * 60 * 60 * 1000_i64;
            let expiry = DateTime::from_millis(DateTime::now().timestamp_millis() + thirty_days_ms);
            input = input.expires_at(expiry);
            Some(expiry.try_to_rfc3339_string().unwrap_or_default())
        } else {
            None
        };

        self.links_repo.create_link(input).await.map_err(|e| {
            if e.contains("E11000") {
                LinkError::LinkIdTaken(link_id.clone())
            } else {
                tracing::error!("Failed to create link: {e}");
                LinkError::Internal(e)
            }
        })?;

        let url = self.canonical_url(&tenant_id, &link_id).await;
        Ok(CreateLinkResponse {
            link_id,
            url,
            expires_at,
        })
    }

    /// Atomically create up to `BULK_LINKS_MAX` links sharing one template.
    /// Either every row is inserted or none — a transaction wraps the
    /// underlying `insert_many` so a race that takes one of our chosen
    /// `custom_ids` rolls back the whole batch and surfaces as a per-row
    /// `link_id_taken` validation error.
    ///
    /// Gated on a verified custom domain (the bulk endpoint is intended for
    /// customers running campaigns under their own domain). Quota is
    /// pre-checked once via `check_n` so the batch is one decision rather
    /// than failing mid-loop.
    #[tracing::instrument(skip(self, req, caller_scope))]
    pub async fn create_links_bulk(
        &self,
        tenant_id: ObjectId,
        caller_scope: Option<&KeyScope>,
        req: BulkCreateLinksRequest,
    ) -> Result<BulkCreateLinksResponse, LinkError> {
        // 1. Mode — exactly one of custom_ids / count.
        let mode_ids = req.custom_ids.as_deref();
        let mode_count = req.count;
        let n = match (mode_ids, mode_count) {
            (Some(_), Some(_)) => return Err(LinkError::BatchModeAmbiguous),
            (None, None) => return Err(LinkError::BatchModeMissing),
            (Some(ids), None) => ids.len(),
            (None, Some(c)) => c as usize,
        };
        if n == 0 {
            return Err(LinkError::BatchEmpty);
        }
        if n > BULK_LINKS_MAX {
            return Err(LinkError::BatchTooLarge {
                max: BULK_LINKS_MAX,
                got: n,
            });
        }

        // 2. Verified-domain gate.
        if !self.tenant_has_verified_domain(&tenant_id).await {
            return Err(LinkError::NoVerifiedDomain);
        }

        // 3. Affiliate resolution — once for the whole batch.
        let resolved_affiliate_id = self
            .resolve_affiliate_id(&tenant_id, caller_scope, req.template.affiliate_id)
            .await?;

        // 4. Template validation (URL, threat, metadata, agent, social).
        validate_link_urls(
            req.template.web_url.as_deref(),
            req.template.ios_deep_link.as_deref(),
            req.template.android_deep_link.as_deref(),
            req.template.ios_store_url.as_deref(),
            req.template.android_store_url.as_deref(),
        )?;
        if let Some(ref web_url) = req.template.web_url {
            if let Some(reason) = self.threat_feed.check_url(web_url).await {
                return Err(LinkError::ThreatDetected(reason));
            }
        }
        validate_agent_context(&req.template.agent_context)?;
        validate_social_preview(&req.template.social_preview)?;
        if let Some(ref meta) = req.template.metadata {
            validation::validate_metadata(meta).map_err(LinkError::InvalidMetadata)?;
        }
        let metadata_doc = req
            .template
            .metadata
            .as_ref()
            .and_then(|v| mongodb::bson::to_document(v).ok());

        // 5. Resolve the list of link_ids and collect any per-row errors.
        let mut item_errors: Vec<BatchItemError> = Vec::new();
        let link_ids: Vec<String> = match (req.custom_ids, req.count) {
            (Some(ids), _) => {
                let mut seen: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for (i, id) in ids.iter().enumerate() {
                    if let Err(LinkError::InvalidCustomId(msg)) = validate_custom_id(id) {
                        item_errors.push(BatchItemError {
                            index: i,
                            custom_id: Some(id.clone()),
                            code: "invalid_custom_id".to_string(),
                            message: msg,
                        });
                        continue;
                    }
                    if let Some(prev) = seen.insert(id.clone(), i) {
                        item_errors.push(BatchItemError {
                            index: i,
                            custom_id: Some(id.clone()),
                            code: "duplicate_custom_id_in_batch".to_string(),
                            message: format!("'{id}' appears at indices {prev} and {i}"),
                        });
                    }
                }
                ids
            }
            (None, Some(_)) => {
                let mut generated: Vec<String> = Vec::with_capacity(n);
                let mut seen: std::collections::HashSet<String> =
                    std::collections::HashSet::with_capacity(n);
                while generated.len() < n {
                    let id = generate_link_id();
                    if seen.insert(id.clone()) {
                        generated.push(id);
                    }
                    // 8-char uppercase alphanumerics from a UUID — collision
                    // odds are astronomical; the dedupe is purely defensive.
                }
                generated
            }
            (None, None) => unreachable!("mode guarded above"),
        };

        // 6. Pre-check uniqueness against the DB (one query for all ids).
        if !link_ids.is_empty() {
            for (i, id) in link_ids.iter().enumerate() {
                // Skip rows that already failed format validation — their
                // index is already in `item_errors`.
                if item_errors.iter().any(|e| e.index == i) {
                    continue;
                }
                if self
                    .links_repo
                    .find_link_by_tenant_and_id(&tenant_id, id)
                    .await
                    .map_err(LinkError::Internal)?
                    .is_some()
                {
                    item_errors.push(BatchItemError {
                        index: i,
                        custom_id: Some(id.clone()),
                        code: "link_id_taken".to_string(),
                        message: format!("'{id}' is already taken"),
                    });
                }
            }
        }

        if !item_errors.is_empty() {
            item_errors.sort_by_key(|e| e.index);
            return Err(LinkError::BatchValidationFailed(item_errors));
        }

        // 7. Quota gate for the whole batch.
        if let Some(q) = &self.quota {
            q.check_n(&tenant_id, Resource::CreateLink, n as u64)
                .await?;
        }

        // 8. Build inputs and insert atomically. The builder methods on
        // `CreateLinkInput` are designed for the single-create path where
        // each field is fed in separately — for bulk we already have the
        // template as `Option<T>` fields, so we construct the struct
        // directly instead of unwrap-then-rewrap through the builder.
        let template = &req.template;
        let inputs: Vec<CreateLinkInput> = link_ids
            .iter()
            .map(|id| CreateLinkInput {
                tenant_id,
                link_id: id.clone(),
                ios_deep_link: template.ios_deep_link.clone(),
                android_deep_link: template.android_deep_link.clone(),
                web_url: template.web_url.clone(),
                ios_store_url: template.ios_store_url.clone(),
                android_store_url: template.android_store_url.clone(),
                metadata: metadata_doc.clone(),
                affiliate_id: resolved_affiliate_id,
                expires_at: None,
                agent_context: template.agent_context.clone(),
                social_preview: template.social_preview.clone(),
            })
            .collect();

        match self.links_repo.create_many_in_txn(inputs).await {
            Ok(_links) => {
                let domain =
                    resolve_verified_primary_domain(self.domains_repo.as_deref(), &tenant_id).await;
                let public_url = &self.public_url;
                let results: Vec<BulkLinkResult> = link_ids
                    .iter()
                    .map(|id| BulkLinkResult {
                        link_id: id.clone(),
                        url: build_canonical_link_url(public_url, id, domain.as_deref()),
                    })
                    .collect();
                Ok(BulkCreateLinksResponse { links: results })
            }
            Err(BulkInsertError::DuplicateLinkIds(indices)) => {
                let errors = indices
                    .into_iter()
                    .map(|i| BatchItemError {
                        index: i,
                        custom_id: link_ids.get(i).cloned(),
                        code: "link_id_taken".to_string(),
                        message: link_ids
                            .get(i)
                            .map(|id| format!("'{id}' is already taken"))
                            .unwrap_or_else(|| "link id already taken".to_string()),
                    })
                    .collect();
                Err(LinkError::BatchValidationFailed(errors))
            }
            Err(BulkInsertError::Internal(e)) => {
                tracing::error!("Failed bulk insert: {e}");
                Err(LinkError::Internal(e))
            }
        }
    }

    #[tracing::instrument(skip(self, caller_scope))]
    pub async fn get_link(
        &self,
        tenant_id: &ObjectId,
        caller_scope: Option<&KeyScope>,
        link_id: &str,
    ) -> Result<LinkDetail, LinkError> {
        let link = self
            .links_repo
            .find_link_by_tenant_and_id(tenant_id, link_id)
            .await
            .map_err(LinkError::Internal)?
            .ok_or(LinkError::NotFound)?;

        // Affiliate-scoped credentials can only read their own affiliate's
        // links. Return NotFound (not Forbidden) so the existence of links
        // belonging to other affiliates isn't disclosed.
        if let Some(KeyScope::Affiliate { affiliate_id }) = caller_scope {
            if link.affiliate_id != Some(*affiliate_id) {
                return Err(LinkError::NotFound);
            }
        }

        Ok(self.link_to_detail(&link).await)
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_links(
        &self,
        tenant_id: &ObjectId,
        limit: Option<i64>,
        cursor: Option<String>,
    ) -> Result<ListLinksResponse, LinkError> {
        let limit = limit.unwrap_or(50).clamp(1, 100);
        let cursor_id = cursor.and_then(|c| ObjectId::parse_str(&c).ok());

        // Fetch one extra to determine if there's a next page.
        let links = self
            .links_repo
            .list_links_by_tenant(tenant_id, limit + 1, cursor_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list links: {e}");
                LinkError::Internal(e)
            })?;

        let has_more = links.len() as i64 > limit;
        let page: Vec<&Link> = links.iter().take(limit as usize).collect();

        let next_cursor = if has_more {
            page.last().map(|l| l.id.to_hex())
        } else {
            None
        };

        let primary_domain =
            resolve_verified_primary_domain(self.domains_repo.as_deref(), tenant_id).await;
        let details: Vec<LinkDetail> = page
            .iter()
            .map(|l| self.link_to_detail_with_domain(l, primary_domain.as_deref()))
            .collect();

        Ok(ListLinksResponse {
            links: details,
            next_cursor,
        })
    }

    #[tracing::instrument(skip(self, req))]
    pub async fn update_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        req: UpdateLinkRequest,
    ) -> Result<LinkDetail, LinkError> {
        // Flatten Option<Option<String>> to Option<&str> for validation.
        let ios_dl = req.ios_deep_link.as_ref().and_then(|v| v.as_deref());
        let android_dl = req.android_deep_link.as_ref().and_then(|v| v.as_deref());

        validate_link_urls(
            req.web_url.as_deref(),
            ios_dl,
            android_dl,
            req.ios_store_url.as_deref(),
            req.android_store_url.as_deref(),
        )?;

        if let Some(ref web_url) = req.web_url {
            if let Some(reason) = self.threat_feed.check_url(web_url).await {
                return Err(LinkError::ThreatDetected(reason));
            }
        }

        if let Some(ref meta) = req.metadata {
            validation::validate_metadata(meta).map_err(LinkError::InvalidMetadata)?;
        }

        validate_agent_context(&req.agent_context)?;
        validate_social_preview(&req.social_preview)?;

        let mut update = mongodb::bson::Document::new();
        let mut unset = mongodb::bson::Document::new();

        // ios_deep_link and android_deep_link support null to clear.
        match &req.ios_deep_link {
            None => {}
            Some(None) => {
                unset.insert("ios_deep_link", "");
            }
            Some(Some(v)) => {
                update.insert("ios_deep_link", v.clone());
            }
        }
        match &req.android_deep_link {
            None => {}
            Some(None) => {
                unset.insert("android_deep_link", "");
            }
            Some(Some(v)) => {
                update.insert("android_deep_link", v.clone());
            }
        }

        if let Some(v) = &req.web_url {
            update.insert("web_url", v.clone());
        }
        if let Some(v) = &req.ios_store_url {
            update.insert("ios_store_url", v.clone());
        }
        if let Some(v) = &req.android_store_url {
            update.insert("android_store_url", v.clone());
        }
        if let Some(v) = &req.metadata {
            if let Ok(doc) = mongodb::bson::to_document(v) {
                update.insert("metadata", doc);
            }
        }
        if let Some(ref ac) = req.agent_context {
            if let Ok(doc) = mongodb::bson::to_document(ac) {
                update.insert("agent_context", doc);
            }
        }
        if let Some(ref sp) = req.social_preview {
            if let Ok(doc) = mongodb::bson::to_document(sp) {
                update.insert("social_preview", doc);
            }
        }

        if update.is_empty() && unset.is_empty() {
            return Err(LinkError::EmptyUpdate);
        }

        let updated = self
            .links_repo
            .update_link(tenant_id, link_id, update, unset)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update link: {e}");
                LinkError::Internal(e)
            })?;

        if !updated {
            return Err(LinkError::NotFound);
        }

        // Re-fetch the updated link. The caller's update was already
        // authorized; passing None here means "no scope check" (treated as
        // Full for read).
        self.get_link(tenant_id, None, link_id).await
    }

    #[tracing::instrument(skip(self))]
    /// Resolve a link via an alternate domain. Returns the store/web URL to redirect to.
    /// No click recording, no landing page — the alternate domain is a Universal Link trampoline.
    pub async fn resolve_alternate(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        user_agent: &str,
    ) -> Result<String, LinkError> {
        let link = self
            .links_repo
            .find_link_by_tenant_and_id(tenant_id, link_id)
            .await
            .map_err(LinkError::Internal)?
            .ok_or(LinkError::NotFound)?;

        let ua = user_agent.to_lowercase();
        let redirect_url = if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
            link.ios_store_url.as_deref().or(link.web_url.as_deref())
        } else if ua.contains("android") {
            link.android_store_url
                .as_deref()
                .or(link.web_url.as_deref())
        } else {
            link.web_url.as_deref()
        };

        redirect_url
            .map(|s| s.to_string())
            .ok_or(LinkError::NotFound)
    }

    pub async fn delete_link(&self, tenant_id: &ObjectId, link_id: &str) -> Result<(), LinkError> {
        let deleted = self
            .links_repo
            .delete_link(tenant_id, link_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete link: {e}");
                LinkError::Internal(e)
            })?;

        if !deleted {
            return Err(LinkError::NotFound);
        }

        Ok(())
    }

    async fn link_to_detail(&self, link: &Link) -> LinkDetail {
        let domain =
            resolve_verified_primary_domain(self.domains_repo.as_deref(), &link.tenant_id).await;
        self.link_to_detail_with_domain(link, domain.as_deref())
    }

    fn link_to_detail_with_domain(
        &self,
        link: &Link,
        verified_primary_domain: Option<&str>,
    ) -> LinkDetail {
        LinkDetail {
            link_id: link.link_id.clone(),
            url: build_canonical_link_url(&self.public_url, &link.link_id, verified_primary_domain),
            ios_deep_link: link.ios_deep_link.clone(),
            android_deep_link: link.android_deep_link.clone(),
            web_url: link.web_url.clone(),
            ios_store_url: link.ios_store_url.clone(),
            android_store_url: link.android_store_url.clone(),
            created_at: link.created_at.try_to_rfc3339_string().unwrap_or_default(),
            affiliate_id: link.affiliate_id,
            agent_context: link.agent_context.clone(),
            social_preview: link.social_preview.clone(),
        }
    }

    async fn tenant_has_verified_domain(&self, tenant_id: &ObjectId) -> bool {
        let Some(ref repo) = self.domains_repo else {
            return false;
        };
        repo.list_by_tenant(tenant_id)
            .await
            .ok()
            .map(|domains| domains.iter().any(|d| d.verified))
            .unwrap_or(false)
    }

    /// Resolve the link's `affiliate_id` from the caller's scope and the
    /// optional value in the request body. See the branch table in
    /// `create_link` for the full matrix.
    async fn resolve_affiliate_id(
        &self,
        tenant_id: &ObjectId,
        caller_scope: Option<&KeyScope>,
        requested: Option<ObjectId>,
    ) -> Result<Option<ObjectId>, LinkError> {
        match (caller_scope, requested) {
            // Affiliate-scoped credential — server pins to scope; reject mismatch.
            (Some(KeyScope::Affiliate { affiliate_id }), None) => Ok(Some(*affiliate_id)),
            (Some(KeyScope::Affiliate { affiliate_id }), Some(req)) if req == *affiliate_id => {
                Ok(Some(*affiliate_id))
            }
            (Some(KeyScope::Affiliate { .. }), Some(_)) => Err(LinkError::AffiliateScopeMismatch),

            // Full scope (or pre-migration grandfather) — accept request value
            // after validating it exists in this tenant.
            (Some(KeyScope::Full), Some(req)) | (None, Some(req)) => {
                let repo = self
                    .affiliates_repo
                    .as_ref()
                    .ok_or(LinkError::AffiliateNotFound)?;
                repo.get_by_id(tenant_id, &req)
                    .await
                    .map_err(LinkError::Internal)?
                    .ok_or(LinkError::AffiliateNotFound)?;
                Ok(Some(req))
            }

            (Some(KeyScope::Full), None) | (None, None) => Ok(None),
        }
    }

    pub async fn canonical_url(&self, tenant_id: &ObjectId, link_id: &str) -> String {
        let domain = resolve_verified_primary_domain(self.domains_repo.as_deref(), tenant_id).await;
        build_canonical_link_url(&self.public_url, link_id, domain.as_deref())
    }
}

pub fn build_canonical_link_url(
    public_url: &str,
    link_id: &str,
    verified_primary_domain: Option<&str>,
) -> String {
    match verified_primary_domain {
        Some(domain) => format!("https://{domain}/{link_id}"),
        None => format!("{}/r/{link_id}", public_url.trim_end_matches('/')),
    }
}

pub async fn resolve_verified_primary_domain(
    domains_repo: Option<&dyn DomainsRepository>,
    tenant_id: &ObjectId,
) -> Option<String> {
    let repo = domains_repo?;
    repo.list_by_tenant(tenant_id)
        .await
        .ok()?
        .into_iter()
        .find(|d| d.verified && d.role == DomainRole::Primary)
        .map(|d| d.domain)
}

// ── Shared Validators ──

pub fn generate_link_id() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>()
        .to_uppercase()
}

pub fn validate_custom_id(id: &str) -> Result<(), LinkError> {
    if id.len() < 3 || id.len() > 64 {
        return Err(LinkError::InvalidCustomId(
            "custom_id must be 3-64 characters".to_string(),
        ));
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(LinkError::InvalidCustomId(
            "custom_id must be alphanumeric with hyphens only".to_string(),
        ));
    }
    if id.starts_with('-') || id.ends_with('-') {
        return Err(LinkError::InvalidCustomId(
            "custom_id must not start or end with a hyphen".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_link_urls(
    web_url: Option<&str>,
    ios_deep_link: Option<&str>,
    android_deep_link: Option<&str>,
    ios_store_url: Option<&str>,
    android_store_url: Option<&str>,
) -> Result<(), LinkError> {
    if let Some(v) = web_url {
        validation::validate_web_url(v)
            .map_err(|e| LinkError::InvalidUrl(format!("web_url: {e}")))?;
    }
    if let Some(v) = ios_deep_link {
        validation::validate_deep_link(v)
            .map_err(|e| LinkError::InvalidUrl(format!("ios_deep_link: {e}")))?;
    }
    if let Some(v) = android_deep_link {
        validation::validate_deep_link(v)
            .map_err(|e| LinkError::InvalidUrl(format!("android_deep_link: {e}")))?;
    }
    if let Some(v) = ios_store_url {
        validation::validate_store_url(v)
            .map_err(|e| LinkError::InvalidUrl(format!("ios_store_url: {e}")))?;
    }
    if let Some(v) = android_store_url {
        validation::validate_store_url(v)
            .map_err(|e| LinkError::InvalidUrl(format!("android_store_url: {e}")))?;
    }
    Ok(())
}

fn validate_agent_context(agent_context: &Option<AgentContext>) -> Result<(), LinkError> {
    let Some(ac) = agent_context else {
        return Ok(());
    };
    if let Some(ref action) = ac.action {
        validation::validate_agent_action(action).map_err(LinkError::InvalidAgentContext)?;
    }
    if let Some(ref cta) = ac.cta {
        validation::validate_cta(cta).map_err(LinkError::InvalidAgentContext)?;
    }
    if let Some(ref desc) = ac.description {
        validation::validate_agent_description(desc).map_err(LinkError::InvalidAgentContext)?;
    }
    Ok(())
}

fn validate_social_preview(social_preview: &Option<SocialPreview>) -> Result<(), LinkError> {
    let Some(preview) = social_preview else {
        return Ok(());
    };

    if preview.title.is_none() && preview.description.is_none() && preview.image_url.is_none() {
        return Err(LinkError::InvalidSocialPreview(
            "social_preview must include at least one field".to_string(),
        ));
    }

    if let Some(title) = preview.title.as_deref() {
        if title.trim().is_empty() {
            return Err(LinkError::InvalidSocialPreview(
                "social_preview.title must not be empty".to_string(),
            ));
        }
        if title.chars().count() > 120 {
            return Err(LinkError::InvalidSocialPreview(
                "social_preview.title must be 120 characters or fewer".to_string(),
            ));
        }
    }

    if let Some(description) = preview.description.as_deref() {
        if description.trim().is_empty() {
            return Err(LinkError::InvalidSocialPreview(
                "social_preview.description must not be empty".to_string(),
            ));
        }
        if description.chars().count() > 300 {
            return Err(LinkError::InvalidSocialPreview(
                "social_preview.description must be 300 characters or fewer".to_string(),
            ));
        }
    }

    if let Some(image_url) = preview.image_url.as_deref() {
        validation::validate_web_url(image_url).map_err(|e| {
            LinkError::InvalidSocialPreview(format!("social_preview.image_url: {e}"))
        })?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

use mongodb::bson::DateTime;
use rift_macros::requires;
use std::sync::Arc;
use uuid::Uuid;

use super::models::*;
use super::repo::LinksRepository;
use crate::core::public_id::{AffiliateId, TenantId};
use crate::core::threat_feed::ThreatFeed;
use crate::core::validation;
use crate::services::affiliates::repo::AffiliatesRepository;
use crate::services::app_users::models::AppUserUpsert;
use crate::services::app_users::repo::AppUsersRepository;
use crate::services::auth::permissions::{AuthContext, Permission, ResourceScope};
use crate::services::billing::quota::{QuotaChecker, Resource};
use crate::services::billing::service::TierResolver;
use crate::services::domains::models::DomainRole;
use crate::services::domains::repo::DomainsRepository;
use crate::services::install_events::models::InstallContext;
use crate::services::install_events::repo::InstallEventsRepository;

// ── Service ──

/// Maximum number of links per `POST /v1/links/bulk` request. Caps memory,
/// threat-feed fan-out, and the size of any single transaction.
pub const BULK_LINKS_MAX: usize = 100;

crate::impl_container!(LinksService);
pub struct LinksService {
    links_repo: Arc<dyn LinksRepository>,
    domains_repo: Option<Arc<dyn DomainsRepository>>,
    /// Optional so the test harness and reduced-feature builds can boot
    /// without an affiliates collection. Affiliate-scoped credentials and
    /// explicit `affiliate_id` values require it; absence is treated as
    /// "no affiliates configured" → `AffiliateNotFound` for explicit values
    /// and a graceful pass-through for unscoped/no-affiliate creates.
    affiliates_repo: Option<Arc<dyn AffiliatesRepository>>,
    /// Owner of the user-scoped identity table. Optional during the
    /// Phase 1 cutover; identify_install gracefully no-ops the new write
    /// path when absent so legacy boot paths keep working.
    app_users_repo: Option<Arc<dyn AppUsersRepository>>,
    /// Owner of the server-derived install lifecycle stream.
    install_events_repo: Option<Arc<dyn InstallEventsRepository>>,
    threat_feed: ThreatFeed,
    public_url: String,
    quota: Option<Arc<dyn QuotaChecker>>,
    tiers: Option<Arc<dyn TierResolver>>,
}

impl LinksService {
    pub fn new(deps: super::models::LinksServiceDeps) -> Self {
        Self {
            links_repo: deps.links_repo,
            domains_repo: deps.domains_repo,
            affiliates_repo: deps.affiliates_repo,
            app_users_repo: deps.app_users_repo,
            install_events_repo: deps.install_events_repo,
            threat_feed: deps.threat_feed,
            public_url: deps.public_url,
            quota: deps.quota,
            tiers: deps.tiers,
        }
    }

    /// Orchestrate a `/lifecycle/identify` call:
    ///
    /// 1. Reject if `install_id` is already bound to a *different* user
    ///    (rebind protection — see `LinkError::IdentifyConflict`).
    /// 2. Upsert the `app_users` identity row, adding `install_id` to the
    ///    user's `install_ids` array (idempotent via `$addToSet`).
    /// 3. Backfill `attribution_events.user_id` for prior anonymous events
    ///    on this install, filling NULLs only — never overwriting.
    /// 4. Fan-out `install_events` (`identified` plus `reinstalled` /
    ///    `new_device` based on prior install device_models).
    ///
    /// Per CLAUDE.md, all transport layers must call this method instead
    /// of touching the repos directly.
    pub async fn identify_install(
        &self,
        tenant_id: &TenantId,
        install_id: &str,
        user_id: &str,
    ) -> Result<IdentifyOutcome, LinkError> {
        let Some(app_users) = &self.app_users_repo else {
            // No app_users repo configured (reduced-feature build / test
            // harness without identity). Identify is a no-op in that mode.
            return Ok(IdentifyOutcome::AlreadyPresent);
        };

        let tenant_oid = tenant_id.as_object_id();

        // 1. Rebind guard. If the install is already bound to a different
        //    user, refuse — option B from the cutover discussion. The
        //    SDK's expected behavior is one install ↔ one user; rebinding
        //    silently would let a logged-out + re-logged-in flow on a
        //    shared device leak attribution across users.
        if let Some(existing) = app_users
            .find_user_id_for_install(tenant_id, install_id)
            .await
            .map_err(LinkError::Internal)?
        {
            if existing != user_id {
                return Err(LinkError::IdentifyConflict {
                    existing_user_id: existing,
                });
            }
        }

        // 2. Capture the user's prior install_ids BEFORE the upsert adds
        //    the current one — this is what feeds the reinstall vs
        //    new_device classification in step 4.
        let prior_install_ids: Vec<String> =
            match app_users.find_by_user_id(tenant_id, user_id).await {
                Ok(Some(existing)) => existing.install_ids,
                Ok(None) => Vec::new(),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        user_id,
                        "app_users prior lookup failed during identify"
                    );
                    Vec::new()
                }
            };

        let upsert = app_users
            .upsert_with_install(tenant_id, user_id, install_id)
            .await
            .map_err(LinkError::Internal)?;

        // 3 + 4 only run on real state changes. AlreadyPresent is an
        // idempotent SDK retry — skip side-effects to avoid double-firing
        // identify webhooks or redundantly rewriting backfilled rows.
        if matches!(upsert, AppUserUpsert::AlreadyPresent) {
            return Ok(IdentifyOutcome::AlreadyPresent);
        }

        // 3. Backfill attribution_events.user_id for prior anonymous
        //    events on this install. Best-effort; failure logs but
        //    doesn't fail the identify.
        match self
            .links_repo
            .backfill_user_id_on_attribution_events(tenant_oid, install_id, user_id)
            .await
        {
            Ok(n) if n > 0 => {
                tracing::debug!(
                    backfilled = n,
                    install_id,
                    user_id,
                    "attribution_events user_id backfilled"
                );
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    install_id,
                    user_id,
                    "attribution_events backfill failed during identify"
                );
            }
        }

        // 4. Fan-out install lifecycle events. Compares device_model
        //    against the user's prior installs' device_models to classify
        //    reinstall vs new_device.
        if let Some(install_events) = &self.install_events_repo {
            let current_device_model = install_events
                .get_device_model(tenant_id, install_id)
                .await
                .ok()
                .flatten();

            let mut prior_device_models = Vec::with_capacity(prior_install_ids.len());
            for prior_id in &prior_install_ids {
                if let Ok(Some(model)) = install_events.get_device_model(tenant_id, prior_id).await
                {
                    prior_device_models.push(model);
                }
            }

            if let Err(e) = install_events
                .record_identify_lifecycle(
                    tenant_id,
                    install_id,
                    user_id,
                    &prior_install_ids,
                    &prior_device_models,
                    current_device_model.as_deref(),
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    install_id,
                    user_id,
                    "install_events identify fan-out failed"
                );
            }
        }

        // 5. Compute first/last touch credit AFTER the backfill — by
        //    this point the user's attribution chain includes the newly
        //    identified install's prior anonymous events, so the credit
        //    reflects the unified chain. Best-effort lookup; on failure
        //    the webhook still fires with both fields absent.
        let credited_ids = self
            .links_repo
            .credited_links_for_user(tenant_oid, user_id, mongodb::bson::DateTime::now())
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    error = %e,
                    user_id,
                    "credited_links_for_user failed during identify"
                );
                Default::default()
            });
        let credited =
            enrich_credited_with_metadata(self.links_repo.as_ref(), tenant_id, credited_ids).await;

        Ok(match upsert {
            AppUserUpsert::Created => IdentifyOutcome::Created(credited),
            AppUserUpsert::InstallAdded => IdentifyOutcome::InstallAdded(credited),
            AppUserUpsert::AlreadyPresent => unreachable!("guarded above"),
        })
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
        tenant_id: TenantId,
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
                tenant_id.to_object_id(),
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

    /// Record a `/lifecycle/attribute` event. Runs the `TrackEvent` quota
    /// gate, resolves the tenant's retention bucket, looks up the
    /// install's bound user_id (if any) via `app_users`, then inserts the
    /// immutable time-series event with `user_id` stamped at write time.
    ///
    /// Returns the bound `user_id` (when known) so the route handler can
    /// include it in the outbound `attribute` webhook payload without
    /// re-querying.
    pub async fn record_attribute_event(
        &self,
        tenant_id: TenantId,
        link_id: &str,
        install_id: &str,
        app_version: &str,
        context: Option<InstallContext>,
    ) -> Result<Option<String>, String> {
        if let Some(q) = &self.quota {
            if let Err(e) = q.check(&tenant_id, Resource::TrackEvent).await {
                tracing::info!(error = %e, "attribute_track_quota_skipped");
            }
        }

        let retention_bucket = match &self.tiers {
            Some(b) => b.retention_bucket_for_tenant(&tenant_id).await.to_string(),
            None => "30d".to_string(),
        };

        let tenant_oid = tenant_id.to_object_id();

        // Resolve user_id at write time so the row doesn't need to be
        // backfilled later for already-identified installs. Best-effort —
        // a lookup failure logs and falls back to None (the next identify
        // will backfill).
        let user_id = match &self.app_users_repo {
            Some(app_users) => app_users
                .find_user_id_for_install(&tenant_id, install_id)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(error = %e, install_id, "app_users user_id lookup failed");
                    None
                }),
            None => None,
        };

        self.links_repo
            .record_attribute_event(
                tenant_oid,
                link_id,
                install_id,
                app_version,
                user_id.as_deref(),
                retention_bucket,
            )
            .await?;

        // Fan-out: write install.created (first time) or install.opened
        // (subsequent). Best-effort — failure here doesn't fail the
        // attribute call.
        if let Some(install_events) = &self.install_events_repo {
            let ctx = context.unwrap_or_else(|| InstallContext {
                app_version: Some(app_version.to_string()),
                ..InstallContext::default()
            });
            if let Err(e) = install_events
                .record_attribute_lifecycle(&tenant_id, install_id, &ctx)
                .await
            {
                tracing::warn!(
                    error = %e,
                    install_id,
                    "install_events fan-out failed during attribute"
                );
            }
        }

        Ok(user_id)
    }

    #[tracing::instrument(skip(self, req, ctx))]
    #[requires(Permission::LinksWrite)]
    pub async fn create_link(
        &self,
        ctx: &AuthContext,
        req: CreateLinkRequest,
    ) -> Result<CreateLinkResponse, LinkError> {
        let tenant_id = ctx.tenant_id;
        let tenant_oid = tenant_id.to_object_id();
        // Quota enforcement lives here (service layer) so MCP tool invocations
        // and HTTP route handlers both hit the same choke point. CLAUDE.md
        // codifies this rule — see "Quota enforcement" section there.
        if let Some(q) = &self.quota {
            q.check(&tenant_id, Resource::CreateLink).await?;
        }

        // Resolve `affiliate_id` against the caller's resource scope.
        //
        // | resource_scope     | req.affiliate_id     | result                      |
        // |--------------------|----------------------|-----------------------------|
        // | Affiliate(A)       | None                 | pin to A                    |
        // | Affiliate(A)       | Some(A)              | pin to A                    |
        // | Affiliate(A)       | Some(B) where B != A | AffiliateScopeMismatch (400)|
        // | Tenant             | Some(B)              | validate B; pin to B        |
        // | Tenant             | None                 | None                        |
        let resolved_affiliate_id = self
            .resolve_affiliate_id(&tenant_id, &ctx.resource_scope, req.affiliate_id)
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
                    .find_link_by_tenant_and_id(&tenant_oid, custom)
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

        // Links without a verified custom domain expire after 30 days.
        let expires_at_dt = (!has_verified_domain).then(|| {
            let thirty_days_ms = 30 * 24 * 60 * 60 * 1000_i64;
            DateTime::from_millis(DateTime::now().timestamp_millis() + thirty_days_ms)
        });
        let expires_at = expires_at_dt.map(|dt| dt.try_to_rfc3339_string().unwrap_or_default());

        let input = CreateLinkInput::new(tenant_id, link_id.clone())
            .ios_deep_link(req.ios_deep_link)
            .android_deep_link(req.android_deep_link)
            .web_url(req.web_url)
            .ios_store_url(req.ios_store_url)
            .android_store_url(req.android_store_url)
            .metadata(metadata)
            .affiliate_id(resolved_affiliate_id)
            .expires_at(expires_at_dt)
            .agent_context(req.agent_context)
            .social_preview(req.social_preview);

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
    #[tracing::instrument(skip(self, req, ctx))]
    #[requires(Permission::LinksWrite)]
    pub async fn create_links_bulk(
        &self,
        ctx: &AuthContext,
        req: BulkCreateLinksRequest,
    ) -> Result<BulkCreateLinksResponse, LinkError> {
        let tenant_id = ctx.tenant_id;
        let tenant_oid = tenant_id.to_object_id();
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
            .resolve_affiliate_id(&tenant_id, &ctx.resource_scope, req.template.affiliate_id)
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
                    .find_link_by_tenant_and_id(&tenant_oid, id)
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
            q.check_n(&ctx.tenant_id, Resource::CreateLink, n as u64)
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

    #[tracing::instrument(skip(self, ctx))]
    #[requires(Permission::LinksRead)]
    pub async fn get_link(
        &self,
        ctx: &AuthContext,
        link_id: &str,
    ) -> Result<LinkDetail, LinkError> {
        let link = self
            .links_repo
            .find_link_by_tenant_and_id(ctx.tenant_id.as_object_id(), link_id)
            .await
            .map_err(LinkError::Internal)?
            .ok_or(LinkError::NotFound)?;

        // Affiliate-scoped credentials can only read their own affiliate's
        // links. Return NotFound (not Forbidden) so the existence of links
        // belonging to other affiliates isn't disclosed.
        if let ResourceScope::Affiliate { affiliate_id } = &ctx.resource_scope {
            if link.affiliate_id != Some(*affiliate_id) {
                return Err(LinkError::NotFound);
            }
        }

        Ok(self.link_to_detail(&link).await)
    }

    #[tracing::instrument(skip(self, ctx))]
    #[requires(Permission::LinksRead)]
    pub async fn list_links(
        &self,
        ctx: &AuthContext,
        limit: Option<i64>,
        cursor: Option<String>,
    ) -> Result<ListLinksResponse, LinkError> {
        let limit = limit.unwrap_or(50).clamp(1, 100);
        let cursor_id = cursor.and_then(|c| {
            crate::core::public_id::LinkInternalId::parse(&c)
                .ok()
                .map(|id| id.to_object_id())
        });

        // Fetch one extra to determine if there's a next page.
        let links = self
            .links_repo
            .list_links_by_tenant(ctx.tenant_id.as_object_id(), limit + 1, cursor_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list links: {e}");
                LinkError::Internal(e)
            })?;

        let has_more = links.len() as i64 > limit;
        let page: Vec<&Link> = links.iter().take(limit as usize).collect();

        let next_cursor = if has_more {
            page.last().map(|l| l.id.to_string())
        } else {
            None
        };

        let primary_domain =
            resolve_verified_primary_domain(self.domains_repo.as_deref(), &ctx.tenant_id).await;
        let details: Vec<LinkDetail> = page
            .iter()
            .map(|l| self.link_to_detail_with_domain(l, primary_domain.as_deref()))
            .collect();

        Ok(ListLinksResponse {
            links: details,
            next_cursor,
        })
    }

    #[tracing::instrument(skip(self, req, ctx))]
    #[requires(Permission::LinksWrite)]
    pub async fn update_link(
        &self,
        ctx: &AuthContext,
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

        // String fields and serializable structs share the same shape:
        // "if Some, insert into the $set doc". Flatten both into one filter_map
        // chain so the parallel branches collapse to data + a single insert.
        use mongodb::bson::Bson;
        let string_fields = [
            ("web_url", req.web_url.as_ref()),
            ("ios_store_url", req.ios_store_url.as_ref()),
            ("android_store_url", req.android_store_url.as_ref()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|s| (k.to_string(), Bson::String(s.clone()))));

        // bson::to_document errors silently drop the field — preserves the
        // pre-refactor behavior. Validation has already gated bad input.
        let doc_fields = [
            ("metadata", req.metadata.as_ref().and_then(to_doc_value)),
            (
                "agent_context",
                req.agent_context.as_ref().and_then(to_doc_value),
            ),
            (
                "social_preview",
                req.social_preview.as_ref().and_then(to_doc_value),
            ),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|d| (k.to_string(), Bson::Document(d))));

        update.extend(string_fields.chain(doc_fields));

        if update.is_empty() && unset.is_empty() {
            return Err(LinkError::EmptyUpdate);
        }

        let updated = self
            .links_repo
            .update_link(ctx.tenant_id.as_object_id(), link_id, update, unset)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update link: {e}");
                LinkError::Internal(e)
            })?;

        if !updated {
            return Err(LinkError::NotFound);
        }

        // Re-fetch the updated link, bypassing the resource-scope filter —
        // the caller's update was already authorized at the macro layer.
        let link = self
            .links_repo
            .find_link_by_tenant_and_id(ctx.tenant_id.as_object_id(), link_id)
            .await
            .map_err(LinkError::Internal)?
            .ok_or(LinkError::NotFound)?;
        Ok(self.link_to_detail(&link).await)
    }

    #[tracing::instrument(skip(self))]
    /// Resolve a link via an alternate domain. Returns the store/web URL to redirect to.
    /// No click recording, no landing page — the alternate domain is a Universal Link trampoline.
    pub async fn resolve_alternate(
        &self,
        tenant_id: &TenantId,
        link_id: &str,
        user_agent: &str,
    ) -> Result<String, LinkError> {
        let link = self
            .links_repo
            .find_link_by_tenant_and_id(tenant_id.as_object_id(), link_id)
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

    #[requires(Permission::LinksDelete)]
    pub async fn delete_link(&self, ctx: &AuthContext, link_id: &str) -> Result<(), LinkError> {
        let deleted = self
            .links_repo
            .delete_link(ctx.tenant_id.as_object_id(), link_id)
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

    async fn tenant_has_verified_domain(&self, tenant_id: &TenantId) -> bool {
        let Some(ref repo) = self.domains_repo else {
            return false;
        };
        repo.list_by_tenant(tenant_id)
            .await
            .ok()
            .map(|domains| domains.iter().any(|d| d.verified))
            .unwrap_or(false)
    }

    /// Resolve the link's `affiliate_id` from the caller's resource scope and
    /// the optional value in the request body. See the branch table in
    /// `create_link` for the full matrix.
    async fn resolve_affiliate_id(
        &self,
        tenant_id: &TenantId,
        resource_scope: &ResourceScope,
        requested: Option<AffiliateId>,
    ) -> Result<Option<AffiliateId>, LinkError> {
        match (resource_scope, requested) {
            // Affiliate-scoped credential — server pins to scope; reject mismatch.
            (ResourceScope::Affiliate { affiliate_id }, None) => Ok(Some(*affiliate_id)),
            (ResourceScope::Affiliate { affiliate_id }, Some(req)) if req == *affiliate_id => {
                Ok(Some(*affiliate_id))
            }
            (ResourceScope::Affiliate { .. }, Some(_)) => Err(LinkError::AffiliateScopeMismatch),

            // Tenant-wide credential — accept request value after validating
            // it exists in this tenant.
            (ResourceScope::Tenant, Some(req)) => {
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

            (ResourceScope::Tenant, None) => Ok(None),
        }
    }

    pub async fn canonical_url(&self, tenant_id: &TenantId, link_id: &str) -> String {
        let domain = resolve_verified_primary_domain(self.domains_repo.as_deref(), tenant_id).await;
        build_canonical_link_url(&self.public_url, link_id, domain.as_deref())
    }
}

/// Enrich a bare-IDs `CreditedLinks` with the corresponding link
/// metadata. Used by webhook fire-sites (identify, conversion) so the
/// outbound payload carries `*_link_metadata` next to `*_link_id`.
///
/// Both lookups hit the 1-hour link cache, so the cost is negligible
/// in steady state. Lookup failures are swallowed (logged) and the
/// metadata field stays `None` — better than failing the webhook
/// dispatch over a stale cache miss.
pub async fn enrich_credited_with_metadata(
    links_repo: &dyn LinksRepository,
    tenant_id: &TenantId,
    mut credited: CreditedLinks,
) -> CreditedLinks {
    async fn fetch_metadata(
        repo: &dyn LinksRepository,
        tenant_id: &TenantId,
        link_id: &str,
    ) -> Option<serde_json::Value> {
        repo.find_link_by_tenant_and_id(tenant_id.as_object_id(), link_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, link_id, "credited link metadata lookup failed");
                None
            })
            .and_then(|l| {
                l.metadata
                    .as_ref()
                    .and_then(|d| serde_json::to_value(d).ok())
            })
    }

    if let Some(id) = credited.first_touch_link_id.as_deref() {
        credited.first_touch_link_metadata = fetch_metadata(links_repo, tenant_id, id).await;
    }
    if let Some(id) = credited.last_touch_link_id.as_deref() {
        // Skip the second lookup when first == last (single-touch
        // user): same link, same cached metadata.
        if credited.first_touch_link_id.as_deref() == Some(id) {
            credited.last_touch_link_metadata = credited.first_touch_link_metadata.clone();
        } else {
            credited.last_touch_link_metadata = fetch_metadata(links_repo, tenant_id, id).await;
        }
    }
    credited
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
    tenant_id: &TenantId,
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

/// Serialize any `Serialize` value to a Bson `Document`, dropping the value
/// silently on failure. Used by `update_link` to fold optional struct fields
/// (metadata, agent_context, social_preview) into the `$set` payload.
fn to_doc_value<T: serde::Serialize>(v: &T) -> Option<mongodb::bson::Document> {
    mongodb::bson::to_document(v).ok()
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

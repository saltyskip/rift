use rift_macros::requires;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::models::{AnalyticsError, Funnel, FunnelParams, FunnelResult, NewUsers, ReturningUsers};
use crate::services::auth::permissions::{AuthContext, Permission};
use crate::services::conversions::repo::ConversionsRepository;
use crate::services::install_events::models::InstallEventType;
use crate::services::install_events::repo::InstallEventsRepository;
use crate::services::links::repo::LinksRepository;

crate::impl_container!(AnalyticsService);
/// Orchestrates the funnel-stats query across the repos that own the
/// underlying data. Per CLAUDE.md, route handlers (REST today, MCP
/// later) must call this method instead of touching repos directly so
/// that quota / authz / business-rule changes land in one place.
pub struct AnalyticsService {
    links_repo: Arc<dyn LinksRepository>,
    install_events_repo: Arc<dyn InstallEventsRepository>,
    /// Optional because conversion tracking is feature-flagged.
    /// Conversions block in the funnel falls back to empty when absent.
    conversions_repo: Option<Arc<dyn ConversionsRepository>>,
}

impl AnalyticsService {
    pub fn new(
        links_repo: Arc<dyn LinksRepository>,
        install_events_repo: Arc<dyn InstallEventsRepository>,
        conversions_repo: Option<Arc<dyn ConversionsRepository>>,
    ) -> Self {
        Self {
            links_repo,
            install_events_repo,
            conversions_repo,
        }
    }

    /// Branched-funnel query across a set of links under a chosen
    /// attribution model. See [`FunnelResult`] for the response shape.
    #[tracing::instrument(skip(self, ctx))]
    #[requires(Permission::LinksRead)]
    pub async fn funnel(
        &self,
        ctx: &AuthContext,
        params: FunnelParams,
    ) -> Result<FunnelResult, AnalyticsError> {
        // 1. Input validation — invariants the service owns regardless
        //    of transport. Empty link_ids is a malformed query; same
        //    for an inverted date range.
        if params.link_ids.is_empty() {
            return Err(AnalyticsError::EmptyLinkIds);
        }
        if params.from > params.to {
            return Err(AnalyticsError::InvalidDateRange);
        }

        let tenant_id = &ctx.tenant_id;

        // 2. Clicks — credit-independent. Direct count over click_events.
        let clicks = self
            .links_repo
            .count_clicks_for_links(
                tenant_id.as_object_id(),
                &params.link_ids,
                params.from,
                params.to,
            )
            .await
            .map_err(AnalyticsError::Internal)?;

        // 3. Credited installs — the set of install_ids that count
        //    toward this link set under the chosen credit model. Used
        //    by every leaf except `clicks` and (via app_users) the
        //    conversions count.
        let credited_installs = self
            .links_repo
            .distinct_install_ids_credited_to_links(
                tenant_id.as_object_id(),
                &params.link_ids,
                params.from,
                params.to,
                params.credit,
            )
            .await
            .map_err(AnalyticsError::Internal)?;

        // 4. Install lifecycle leaves — five counts, one per event type.
        let installed = self
            .count_lifecycle(
                tenant_id,
                InstallEventType::Created,
                &credited_installs,
                &params,
            )
            .await?;
        let identified = self
            .count_lifecycle(
                tenant_id,
                InstallEventType::Identified,
                &credited_installs,
                &params,
            )
            .await?;
        let reinstalled = self
            .count_lifecycle(
                tenant_id,
                InstallEventType::Reinstalled,
                &credited_installs,
                &params,
            )
            .await?;
        let new_device = self
            .count_lifecycle(
                tenant_id,
                InstallEventType::NewDevice,
                &credited_installs,
                &params,
            )
            .await?;
        let engaged = self
            .count_lifecycle(
                tenant_id,
                InstallEventType::Opened,
                &credited_installs,
                &params,
            )
            .await?;

        // 5. Conversions — credit each conversion to a campaign by
        //    walking the converter's `attribution_events` chain under
        //    the chosen model, then count only those credited to the
        //    queried link set. Avoids the user-scoped leak (one user
        //    touched by campaigns A and B → conversion would appear in
        //    both funnels). Absent conversions_repo = tenant hasn't
        //    configured conversion tracking; surface empty.
        let conversions: BTreeMap<String, u64> = match &self.conversions_repo {
            Some(cr) => cr
                .count_conversions_by_type_credited_to_links(
                    tenant_id.as_object_id(),
                    &params.link_ids,
                    params.from,
                    params.to,
                    params.credit,
                )
                .await
                .map_err(AnalyticsError::Internal)?
                .into_iter()
                .collect(),
            None => BTreeMap::new(),
        };

        Ok(FunnelResult {
            from: params
                .from
                .try_to_rfc3339_string()
                .unwrap_or_else(|_| params.from.to_string()),
            to: params
                .to
                .try_to_rfc3339_string()
                .unwrap_or_else(|_| params.to.to_string()),
            link_ids: params.link_ids,
            credit: params.credit.as_str().to_string(),
            funnel: Funnel {
                clicks,
                new_users: NewUsers {
                    installed,
                    identified,
                },
                returning_users: ReturningUsers {
                    reinstalled,
                    new_device,
                    engaged,
                },
                conversions,
            },
        })
    }

    // ── Helpers ──

    async fn count_lifecycle(
        &self,
        tenant_id: &crate::core::public_id::TenantId,
        event_type: InstallEventType,
        install_ids: &[String],
        params: &FunnelParams,
    ) -> Result<u64, AnalyticsError> {
        let tenant_id = &tenant_id.to_object_id();
        self.install_events_repo
            .count_events_by_type_for_installs(
                tenant_id,
                event_type,
                install_ids,
                params.from,
                params.to,
            )
            .await
            .map_err(AnalyticsError::Internal)
    }
}

use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

use super::models::*;
use super::repo::LinksRepository;
use crate::core::threat_feed::ThreatFeed;
use crate::core::validation;
use crate::services::domains::models::DomainRole;
use crate::services::domains::repo::DomainsRepository;

// ── Error ──

#[derive(Debug)]
pub enum LinkError {
    InvalidCustomId(String),
    InvalidUrl(String),
    InvalidMetadata(String),
    InvalidAgentContext(String),
    InvalidSocialPreview(String),
    ThreatDetected(String),
    LinkIdTaken(String),
    NotFound,
    NoVerifiedDomain,
    EmptyUpdate,
    Internal(String),
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCustomId(e) => write!(f, "{e}"),
            Self::InvalidUrl(e) => write!(f, "{e}"),
            Self::InvalidMetadata(e) => write!(f, "{e}"),
            Self::InvalidAgentContext(e) => write!(f, "{e}"),
            Self::InvalidSocialPreview(e) => write!(f, "{e}"),
            Self::ThreatDetected(e) => write!(f, "{e}"),
            Self::LinkIdTaken(id) => write!(f, "'{id}' is already taken"),
            Self::NotFound => write!(f, "Link not found"),
            Self::NoVerifiedDomain => {
                write!(f, "Custom IDs require a verified custom domain")
            }
            Self::EmptyUpdate => write!(f, "No fields to update"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl LinkError {
    /// Machine-readable error code for API responses.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidCustomId(_) => "invalid_custom_id",
            Self::InvalidUrl(_) => "invalid_url",
            Self::InvalidMetadata(_) => "invalid_metadata",
            Self::InvalidAgentContext(_) => "invalid_agent_context",
            Self::InvalidSocialPreview(_) => "invalid_social_preview",
            Self::ThreatDetected(_) => "threat_detected",
            Self::LinkIdTaken(_) => "link_id_taken",
            Self::NotFound => "not_found",
            Self::NoVerifiedDomain => "no_verified_domain",
            Self::EmptyUpdate => "empty_update",
            Self::Internal(_) => "db_error",
        }
    }
}

// ── Service ──

pub struct LinksService {
    links_repo: Arc<dyn LinksRepository>,
    domains_repo: Option<Arc<dyn DomainsRepository>>,
    threat_feed: ThreatFeed,
    public_url: String,
}

impl LinksService {
    pub fn new(
        links_repo: Arc<dyn LinksRepository>,
        domains_repo: Option<Arc<dyn DomainsRepository>>,
        threat_feed: ThreatFeed,
        public_url: String,
    ) -> Self {
        Self {
            links_repo,
            domains_repo,
            threat_feed,
            public_url,
        }
    }

    #[tracing::instrument(skip(self, req))]
    pub async fn create_link(
        &self,
        tenant_id: ObjectId,
        req: CreateLinkRequest,
    ) -> Result<CreateLinkResponse, LinkError> {
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

    #[tracing::instrument(skip(self))]
    pub async fn get_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<LinkDetail, LinkError> {
        let link = self
            .links_repo
            .find_link_by_tenant_and_id(tenant_id, link_id)
            .await
            .map_err(LinkError::Internal)?
            .ok_or(LinkError::NotFound)?;

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

        // Re-fetch the updated link.
        self.get_link(tenant_id, link_id).await
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
mod tests {
    use super::*;
    use crate::services::domains::repo::DomainsRepository;
    use crate::services::links::models::TimeseriesDataPoint;
    use crate::services::links::repo::LinksRepository;
    use async_trait::async_trait;
    use mongodb::bson::{oid::ObjectId, DateTime, Document};
    use std::sync::Mutex;

    #[test]
    fn canonical_link_url_prefers_verified_primary_domain() {
        assert_eq!(
            build_canonical_link_url(
                "https://api.riftl.ink",
                "summer-sale",
                Some("go.example.com")
            ),
            "https://go.example.com/summer-sale"
        );
    }

    #[test]
    fn canonical_link_url_falls_back_to_public_resolve_path() {
        assert_eq!(
            build_canonical_link_url("https://api.riftl.ink/", "summer-sale", None),
            "https://api.riftl.ink/r/summer-sale"
        );
    }

    // ── Mock LinksRepository ──

    #[derive(Default)]
    struct MockLinksRepo {
        links: Mutex<Vec<Link>>,
    }

    impl MockLinksRepo {
        fn with_links(links: Vec<Link>) -> Self {
            Self {
                links: Mutex::new(links),
            }
        }
    }

    fn make_link(tenant_id: ObjectId, link_id: &str) -> Link {
        Link {
            id: ObjectId::new(),
            tenant_id,
            link_id: link_id.to_string(),
            ios_deep_link: None,
            android_deep_link: None,
            web_url: Some("https://example.com".to_string()),
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            created_at: DateTime::now(),
            status: LinkStatus::Active,
            flag_reason: None,
            expires_at: None,
            agent_context: None,
            social_preview: None,
        }
    }

    #[async_trait]
    impl LinksRepository for MockLinksRepo {
        async fn create_link(&self, input: CreateLinkInput) -> Result<Link, String> {
            let mut links = self.links.lock().unwrap();
            if links.iter().any(|l| l.link_id == input.link_id) {
                return Err("E11000 duplicate key".to_string());
            }
            let link = Link {
                id: ObjectId::new(),
                tenant_id: input.tenant_id,
                link_id: input.link_id,
                ios_deep_link: input.ios_deep_link,
                android_deep_link: input.android_deep_link,
                web_url: input.web_url,
                ios_store_url: input.ios_store_url,
                android_store_url: input.android_store_url,
                metadata: input.metadata,
                created_at: DateTime::now(),
                status: LinkStatus::Active,
                flag_reason: None,
                expires_at: input.expires_at,
                agent_context: input.agent_context,
                social_preview: input.social_preview,
            };
            links.push(link.clone());
            Ok(link)
        }

        async fn find_link_by_id(&self, link_id: &str) -> Result<Option<Link>, String> {
            let links = self.links.lock().unwrap();
            Ok(links.iter().find(|l| l.link_id == link_id).cloned())
        }

        async fn find_link_by_tenant_and_id(
            &self,
            tenant_id: &ObjectId,
            link_id: &str,
        ) -> Result<Option<Link>, String> {
            let links = self.links.lock().unwrap();
            Ok(links
                .iter()
                .find(|l| l.tenant_id == *tenant_id && l.link_id == link_id)
                .cloned())
        }

        async fn update_link(
            &self,
            tenant_id: &ObjectId,
            link_id: &str,
            update: Document,
            _unset: Document,
        ) -> Result<bool, String> {
            let mut links = self.links.lock().unwrap();
            let Some(link) = links
                .iter_mut()
                .find(|l| l.tenant_id == *tenant_id && l.link_id == link_id)
            else {
                return Ok(false);
            };
            if let Ok(v) = update.get_str("web_url") {
                link.web_url = Some(v.to_string());
            }
            if let Ok(v) = update.get_str("ios_deep_link") {
                link.ios_deep_link = Some(v.to_string());
            }
            if let Ok(v) = update.get_str("android_deep_link") {
                link.android_deep_link = Some(v.to_string());
            }
            if let Ok(v) = update.get_str("ios_store_url") {
                link.ios_store_url = Some(v.to_string());
            }
            if let Ok(v) = update.get_str("android_store_url") {
                link.android_store_url = Some(v.to_string());
            }
            if let Ok(v) = update.get_document("metadata") {
                link.metadata = Some(v.clone());
            }
            if let Ok(v) = update.get_document("agent_context") {
                link.agent_context = mongodb::bson::from_document(v.clone()).ok();
            }
            if let Ok(v) = update.get_document("social_preview") {
                link.social_preview = mongodb::bson::from_document(v.clone()).ok();
            }
            Ok(true)
        }

        async fn delete_link(&self, tenant_id: &ObjectId, link_id: &str) -> Result<bool, String> {
            let mut links = self.links.lock().unwrap();
            let len_before = links.len();
            links.retain(|l| !(l.tenant_id == *tenant_id && l.link_id == link_id));
            Ok(links.len() < len_before)
        }

        async fn count_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
            let links = self.links.lock().unwrap();
            Ok(links.iter().filter(|l| l.tenant_id == *tenant_id).count() as u64)
        }

        async fn list_links_by_tenant(
            &self,
            tenant_id: &ObjectId,
            limit: i64,
            _cursor: Option<ObjectId>,
        ) -> Result<Vec<Link>, String> {
            let links = self.links.lock().unwrap();
            Ok(links
                .iter()
                .filter(|l| l.tenant_id == *tenant_id)
                .take(limit as usize)
                .cloned()
                .collect())
        }

        async fn record_click(
            &self,
            _tenant_id: ObjectId,
            _link_id: &str,
            _user_agent: Option<String>,
            _referer: Option<String>,
            _platform: Option<String>,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn count_clicks(&self, _tenant_id: &ObjectId, _link_id: &str) -> Result<u64, String> {
            Ok(0)
        }

        async fn get_click_timeseries(
            &self,
            _tenant_id: &ObjectId,
            _link_id: &str,
            _from: DateTime,
            _to: DateTime,
        ) -> Result<Vec<TimeseriesDataPoint>, String> {
            Ok(vec![])
        }

        async fn upsert_attribution(
            &self,
            _tenant_id: ObjectId,
            _link_id: &str,
            _install_id: &str,
            _app_version: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn link_attribution_to_user(
            &self,
            _tenant_id: &ObjectId,
            _install_id: &str,
            _user_id: &str,
        ) -> Result<bool, String> {
            Ok(true)
        }

        async fn count_attributions(
            &self,
            _tenant_id: &ObjectId,
            _link_id: &str,
        ) -> Result<u64, String> {
            Ok(0)
        }

        async fn find_attribution_by_user(
            &self,
            _tenant_id: &ObjectId,
            _user_id: &str,
        ) -> Result<Option<Attribution>, String> {
            Ok(None)
        }
    }

    // ── Mock DomainsRepository ──

    struct MockDomainsRepo {
        has_verified: bool,
    }

    #[async_trait]
    impl DomainsRepository for MockDomainsRepo {
        async fn create_domain(
            &self,
            _tenant_id: ObjectId,
            _domain: String,
            _verification_token: String,
            _role: crate::services::domains::models::DomainRole,
        ) -> Result<crate::services::domains::models::Domain, String> {
            unimplemented!()
        }

        async fn find_by_domain(
            &self,
            _domain: &str,
        ) -> Result<Option<crate::services::domains::models::Domain>, String> {
            unimplemented!()
        }

        async fn list_by_tenant(
            &self,
            _tenant_id: &ObjectId,
        ) -> Result<Vec<crate::services::domains::models::Domain>, String> {
            if self.has_verified {
                Ok(vec![crate::services::domains::models::Domain {
                    id: ObjectId::new(),
                    tenant_id: ObjectId::new(),
                    domain: "example.com".to_string(),
                    verified: true,
                    verification_token: "token".to_string(),
                    role: crate::services::domains::models::DomainRole::Primary,
                    created_at: DateTime::now(),
                }])
            } else {
                Ok(vec![])
            }
        }

        async fn count_by_tenant(&self, _tenant_id: &ObjectId) -> Result<u64, String> {
            Ok(if self.has_verified { 1 } else { 0 })
        }

        async fn delete_domain(
            &self,
            _tenant_id: &ObjectId,
            _domain: &str,
        ) -> Result<bool, String> {
            Ok(true)
        }

        async fn mark_verified(&self, _domain: &str) -> Result<(), String> {
            Ok(())
        }

        async fn find_alternate_by_tenant(
            &self,
            _tenant_id: &ObjectId,
        ) -> Result<Option<crate::services::domains::models::Domain>, String> {
            Ok(None)
        }
    }

    fn make_service(links: Vec<Link>, has_verified_domain: bool) -> LinksService {
        let repo = Arc::new(MockLinksRepo::with_links(links));
        let domains = Arc::new(MockDomainsRepo {
            has_verified: has_verified_domain,
        });
        LinksService::new(
            repo,
            Some(domains),
            ThreatFeed::new(),
            "https://riftl.ink".to_string(),
        )
    }

    // ── Tests ──

    #[tokio::test]
    async fn create_link_generates_id() {
        let svc = make_service(vec![], false);
        let tenant_id = ObjectId::new();
        let req = CreateLinkRequest {
            custom_id: None,
            ios_deep_link: None,
            android_deep_link: None,
            web_url: Some("https://example.com".to_string()),
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        let resp = svc.create_link(tenant_id, req).await.unwrap();
        assert_eq!(resp.link_id.len(), 8);
        assert!(resp.url.contains(&resp.link_id));
    }

    #[tokio::test]
    async fn create_link_custom_id_requires_verified_domain() {
        let svc = make_service(vec![], false);
        let tenant_id = ObjectId::new();
        let req = CreateLinkRequest {
            custom_id: Some("my-link".to_string()),
            ios_deep_link: None,
            android_deep_link: None,
            web_url: Some("https://example.com".to_string()),
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        let err = svc.create_link(tenant_id, req).await.unwrap_err();
        assert!(matches!(err, LinkError::NoVerifiedDomain));
    }

    #[tokio::test]
    async fn create_link_custom_id_with_verified_domain() {
        let svc = make_service(vec![], true);
        let tenant_id = ObjectId::new();
        let req = CreateLinkRequest {
            custom_id: Some("my-link".to_string()),
            ios_deep_link: None,
            android_deep_link: None,
            web_url: Some("https://example.com".to_string()),
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        let resp = svc.create_link(tenant_id, req).await.unwrap();
        assert_eq!(resp.link_id, "my-link");
        assert_eq!(resp.url, "https://example.com/my-link");
    }

    #[tokio::test]
    async fn create_link_invalid_custom_id() {
        let svc = make_service(vec![], true);
        let tenant_id = ObjectId::new();
        let req = CreateLinkRequest {
            custom_id: Some("ab".to_string()), // too short
            ios_deep_link: None,
            android_deep_link: None,
            web_url: None,
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        let err = svc.create_link(tenant_id, req).await.unwrap_err();
        assert!(matches!(err, LinkError::InvalidCustomId(_)));
    }

    #[tokio::test]
    async fn create_link_duplicate() {
        let tenant_id = ObjectId::new();
        let existing = make_link(tenant_id, "EXISTING");
        let svc = make_service(vec![existing], false);

        let req = CreateLinkRequest {
            custom_id: None,
            ios_deep_link: None,
            android_deep_link: None,
            web_url: Some("https://example.com".to_string()),
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        // First create should succeed (random ID won't collide with "EXISTING")
        let resp = svc.create_link(tenant_id, req).await.unwrap();
        assert_ne!(resp.link_id, "EXISTING");
    }

    #[tokio::test]
    async fn get_link_existing() {
        let tenant_id = ObjectId::new();
        let link = make_link(tenant_id, "ABC123");
        let svc = make_service(vec![link], false);

        let detail = svc.get_link(&tenant_id, "ABC123").await.unwrap();
        assert_eq!(detail.link_id, "ABC123");
        assert!(detail.url.contains("ABC123"));
    }

    #[tokio::test]
    async fn get_link_not_found() {
        let svc = make_service(vec![], false);
        let tenant_id = ObjectId::new();

        let err = svc.get_link(&tenant_id, "NOPE").await.unwrap_err();
        assert!(matches!(err, LinkError::NotFound));
    }

    #[tokio::test]
    async fn list_links_returns_page() {
        let tenant_id = ObjectId::new();
        let links: Vec<Link> = (0..3)
            .map(|i| make_link(tenant_id, &format!("L{i}")))
            .collect();
        let svc = make_service(links, false);

        let resp = svc.list_links(&tenant_id, Some(10), None).await.unwrap();
        assert_eq!(resp.links.len(), 3);
        assert!(resp.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_links_empty() {
        let svc = make_service(vec![], false);
        let tenant_id = ObjectId::new();

        let resp = svc.list_links(&tenant_id, None, None).await.unwrap();
        assert!(resp.links.is_empty());
        assert!(resp.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_links_clamps_limit() {
        let tenant_id = ObjectId::new();
        let links: Vec<Link> = (0..5)
            .map(|i| make_link(tenant_id, &format!("L{i}")))
            .collect();
        let svc = make_service(links, false);

        // Limit > 100 should be clamped
        let resp = svc.list_links(&tenant_id, Some(200), None).await.unwrap();
        assert_eq!(resp.links.len(), 5);

        // Limit < 1 should be clamped to 1
        let resp = svc.list_links(&tenant_id, Some(0), None).await.unwrap();
        assert_eq!(resp.links.len(), 1);
    }

    #[tokio::test]
    async fn update_link_success() {
        let tenant_id = ObjectId::new();
        let link = make_link(tenant_id, "UPD123");
        let svc = make_service(vec![link], false);

        let req = UpdateLinkRequest {
            ios_deep_link: None,
            android_deep_link: None,
            web_url: Some("https://updated.com".to_string()),
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        let detail = svc.update_link(&tenant_id, "UPD123", req).await.unwrap();
        assert_eq!(detail.web_url.as_deref(), Some("https://updated.com"));
    }

    #[tokio::test]
    async fn update_link_not_found() {
        let svc = make_service(vec![], false);
        let tenant_id = ObjectId::new();

        let req = UpdateLinkRequest {
            ios_deep_link: None,
            android_deep_link: None,
            web_url: Some("https://example.com".to_string()),
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        let err = svc.update_link(&tenant_id, "NOPE", req).await.unwrap_err();
        assert!(matches!(err, LinkError::NotFound));
    }

    #[tokio::test]
    async fn update_link_empty() {
        let tenant_id = ObjectId::new();
        let link = make_link(tenant_id, "EMPTY1");
        let svc = make_service(vec![link], false);

        let req = UpdateLinkRequest {
            ios_deep_link: None,
            android_deep_link: None,
            web_url: None,
            ios_store_url: None,
            android_store_url: None,
            metadata: None,
            agent_context: None,
            social_preview: None,
        };

        let err = svc
            .update_link(&tenant_id, "EMPTY1", req)
            .await
            .unwrap_err();
        assert!(matches!(err, LinkError::EmptyUpdate));
    }

    #[tokio::test]
    async fn delete_link_success() {
        let tenant_id = ObjectId::new();
        let link = make_link(tenant_id, "DEL123");
        let svc = make_service(vec![link], false);

        svc.delete_link(&tenant_id, "DEL123").await.unwrap();
    }

    #[tokio::test]
    async fn delete_link_not_found() {
        let svc = make_service(vec![], false);
        let tenant_id = ObjectId::new();

        let err = svc.delete_link(&tenant_id, "NOPE").await.unwrap_err();
        assert!(matches!(err, LinkError::NotFound));
    }
}

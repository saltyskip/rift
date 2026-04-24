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
        _retention_bucket: String,
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

    async fn delete_domain(&self, _tenant_id: &ObjectId, _domain: &str) -> Result<bool, String> {
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
        None,
        None,
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

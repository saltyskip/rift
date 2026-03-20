use async_trait::async_trait;
use mongodb::bson::{oid::ObjectId, DateTime, Document};
use std::sync::Mutex;

use relay::api::links::models::{Click, Link};
use relay::api::links::repo::LinksRepository;

struct Attribution {
    tenant_id: ObjectId,
    link_id: String,
    install_id: String,
    user_id: Option<String>,
}

#[derive(Default)]
pub struct MockLinksRepo {
    pub links: Mutex<Vec<Link>>,
    clicks: Mutex<Vec<Click>>,
    attributions: Mutex<Vec<Attribution>>,
}

#[async_trait]
impl LinksRepository for MockLinksRepo {
    async fn create_link(
        &self,
        tenant_id: ObjectId,
        link_id: String,
        ios_deep_link: Option<String>,
        android_deep_link: Option<String>,
        web_url: Option<String>,
        ios_store_url: Option<String>,
        android_store_url: Option<String>,
        metadata: Option<Document>,
    ) -> Result<Link, String> {
        let mut links = self.links.lock().unwrap();
        if links.iter().any(|l| l.link_id == link_id) {
            return Err("E11000 duplicate key".to_string());
        }
        let link = Link {
            id: ObjectId::new(),
            tenant_id,
            link_id,
            ios_deep_link,
            android_deep_link,
            web_url,
            ios_store_url,
            android_store_url,
            metadata,
            created_at: DateTime::now(),
        };
        links.push(link.clone());
        Ok(link)
    }

    async fn find_link_by_id(&self, link_id: &str) -> Result<Option<Link>, String> {
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .find(|l| l.link_id == link_id)
            .cloned())
    }

    async fn list_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<Link>, String> {
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .filter(|l| &l.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn record_click(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        user_agent: Option<String>,
        referer: Option<String>,
        platform: Option<String>,
        token: Option<String>,
    ) -> Result<(), String> {
        self.clicks.lock().unwrap().push(Click {
            id: ObjectId::new(),
            tenant_id,
            link_id: link_id.to_string(),
            clicked_at: DateTime::now(),
            user_agent,
            referer,
            platform,
            token,
        });
        Ok(())
    }

    async fn find_click_by_token(&self, token: &str) -> Result<Option<Click>, String> {
        Ok(self
            .clicks
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.token.as_deref() == Some(token))
            .cloned())
    }

    async fn count_clicks(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String> {
        Ok(self
            .clicks
            .lock()
            .unwrap()
            .iter()
            .filter(|c| &c.tenant_id == tenant_id && c.link_id == link_id)
            .count() as u64)
    }

    async fn upsert_attribution(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        install_id: &str,
        _app_version: &str,
    ) -> Result<(), String> {
        let mut attrs = self.attributions.lock().unwrap();
        if !attrs
            .iter()
            .any(|a| a.tenant_id == tenant_id && a.install_id == install_id)
        {
            attrs.push(Attribution {
                tenant_id,
                link_id: link_id.to_string(),
                install_id: install_id.to_string(),
                user_id: None,
            });
        }
        Ok(())
    }

    async fn link_attribution_to_user(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
    ) -> Result<bool, String> {
        let mut attrs = self.attributions.lock().unwrap();
        if let Some(attr) = attrs
            .iter_mut()
            .find(|a| &a.tenant_id == tenant_id && a.install_id == install_id)
        {
            attr.user_id = Some(user_id.to_string());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn count_attributions(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String> {
        Ok(self
            .attributions
            .lock()
            .unwrap()
            .iter()
            .filter(|a| &a.tenant_id == tenant_id && a.link_id == link_id)
            .count() as u64)
    }
}

use async_trait::async_trait;
use mongodb::bson::{oid::ObjectId, DateTime, Document};
use std::collections::BTreeMap;
use std::sync::Mutex;

use rift::services::links::models::{
    Attribution as RealAttribution, ClickEvent, ClickMeta, CreateLinkInput, Link, LinkStatus,
    TimeseriesDataPoint,
};
use rift::services::links::repo::LinksRepository;

struct Attribution {
    tenant_id: ObjectId,
    link_id: String,
    install_id: String,
    user_id: Option<String>,
}

#[derive(Default)]
pub struct MockLinksRepo {
    pub links: Mutex<Vec<Link>>,
    pub clicks: Mutex<Vec<ClickEvent>>,
    attributions: Mutex<Vec<Attribution>>,
}

#[async_trait]
impl LinksRepository for MockLinksRepo {
    async fn create_link(&self, input: CreateLinkInput) -> Result<Link, String> {
        let mut links = self.links.lock().unwrap();
        if links
            .iter()
            .any(|l| l.tenant_id == input.tenant_id && l.link_id == input.link_id)
        {
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
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .find(|l| l.link_id == link_id)
            .cloned())
    }

    async fn find_link_by_tenant_and_id(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
    ) -> Result<Option<Link>, String> {
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .find(|l| &l.tenant_id == tenant_id && l.link_id == link_id)
            .cloned())
    }

    async fn update_link(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        set: Document,
        unset: Document,
    ) -> Result<bool, String> {
        let mut links = self.links.lock().unwrap();
        let Some(link) = links
            .iter_mut()
            .find(|l| &l.tenant_id == tenant_id && l.link_id == link_id)
        else {
            return Ok(false);
        };
        // Apply $set fields.
        if let Ok(v) = set.get_str("ios_deep_link") {
            link.ios_deep_link = Some(v.to_string());
        }
        if let Ok(v) = set.get_str("android_deep_link") {
            link.android_deep_link = Some(v.to_string());
        }
        if let Ok(v) = set.get_str("web_url") {
            link.web_url = Some(v.to_string());
        }
        if let Ok(v) = set.get_str("ios_store_url") {
            link.ios_store_url = Some(v.to_string());
        }
        if let Ok(v) = set.get_str("android_store_url") {
            link.android_store_url = Some(v.to_string());
        }
        if let Ok(v) = set.get_document("metadata") {
            link.metadata = Some(v.clone());
        }
        if let Ok(v) = set.get_document("agent_context") {
            link.agent_context = mongodb::bson::from_document(v.clone()).ok();
        }
        if let Ok(v) = set.get_document("social_preview") {
            link.social_preview = mongodb::bson::from_document(v.clone()).ok();
        }
        // Apply $unset fields.
        for key in unset.keys() {
            match key.as_str() {
                "ios_deep_link" => link.ios_deep_link = None,
                "android_deep_link" => link.android_deep_link = None,
                _ => {}
            }
        }
        Ok(true)
    }

    async fn delete_link(&self, tenant_id: &ObjectId, link_id: &str) -> Result<bool, String> {
        let mut links = self.links.lock().unwrap();
        let len_before = links.len();
        links.retain(|l| !(&l.tenant_id == tenant_id && l.link_id == link_id));
        Ok(links.len() < len_before)
    }

    async fn count_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
        let links = self.links.lock().unwrap();
        Ok(links.iter().filter(|l| &l.tenant_id == tenant_id).count() as u64)
    }

    async fn list_links_by_tenant(
        &self,
        tenant_id: &ObjectId,
        limit: i64,
        cursor: Option<ObjectId>,
    ) -> Result<Vec<Link>, String> {
        let links = self.links.lock().unwrap();
        let mut filtered: Vec<Link> = links
            .iter()
            .filter(|l| &l.tenant_id == tenant_id && cursor.is_none_or(|c| l.id < c))
            .cloned()
            .collect();
        // Sort by _id descending (ObjectIds are monotonically increasing).
        filtered.sort_by_key(|l| std::cmp::Reverse(l.id));
        filtered.truncate(limit as usize);
        Ok(filtered)
    }

    async fn record_click(
        &self,
        tenant_id: ObjectId,
        link_id: &str,
        user_agent: Option<String>,
        referer: Option<String>,
        platform: Option<String>,
    ) -> Result<(), String> {
        self.clicks.lock().unwrap().push(ClickEvent {
            meta: ClickMeta {
                tenant_id,
                link_id: link_id.to_string(),
            },
            clicked_at: DateTime::now(),
            user_agent,
            referer,
            platform,
        });
        Ok(())
    }

    async fn count_clicks(&self, tenant_id: &ObjectId, link_id: &str) -> Result<u64, String> {
        Ok(self
            .clicks
            .lock()
            .unwrap()
            .iter()
            .filter(|c| &c.meta.tenant_id == tenant_id && c.meta.link_id == link_id)
            .count() as u64)
    }

    async fn get_click_timeseries(
        &self,
        tenant_id: &ObjectId,
        link_id: &str,
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<TimeseriesDataPoint>, String> {
        let clicks = self.clicks.lock().unwrap();
        let mut buckets: BTreeMap<String, u64> = BTreeMap::new();

        for click in clicks.iter() {
            if &click.meta.tenant_id == tenant_id
                && click.meta.link_id == link_id
                && click.clicked_at >= from
                && click.clicked_at <= to
            {
                let date = click
                    .clicked_at
                    .try_to_rfc3339_string()
                    .unwrap_or_default()
                    .chars()
                    .take(10)
                    .collect::<String>();
                *buckets.entry(date).or_insert(0) += 1;
            }
        }

        Ok(buckets
            .into_iter()
            .map(|(date, clicks)| TimeseriesDataPoint { date, clicks })
            .collect())
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
        // Mirrors the real repo's re-bind protection: only update when user_id
        // is None or already matches the new value. Never overwrite a different
        // user_id with this method.
        let mut attrs = self.attributions.lock().unwrap();
        if let Some(attr) = attrs
            .iter_mut()
            .find(|a| &a.tenant_id == tenant_id && a.install_id == install_id)
        {
            match attr.user_id.as_deref() {
                None => {
                    attr.user_id = Some(user_id.to_string());
                    Ok(true)
                }
                Some(existing) if existing == user_id => Ok(true),
                Some(_) => Ok(false),
            }
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

    async fn find_attribution_by_user(
        &self,
        tenant_id: &ObjectId,
        user_id: &str,
    ) -> Result<Option<RealAttribution>, String> {
        let attrs = self.attributions.lock().unwrap();
        Ok(attrs
            .iter()
            .find(|a| &a.tenant_id == tenant_id && a.user_id.as_deref() == Some(user_id))
            .map(|a| RealAttribution {
                id: ObjectId::new(),
                tenant_id: a.tenant_id,
                link_id: a.link_id.clone(),
                install_id: a.install_id.clone(),
                user_id: a.user_id.clone(),
                app_version: "test".to_string(),
                attributed_at: DateTime::now(),
            }))
    }
}

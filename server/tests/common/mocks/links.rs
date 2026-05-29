use async_trait::async_trait;
use mongodb::bson::{oid::ObjectId, DateTime, Document};
use std::sync::Mutex;

use rift::services::links::models::BulkInsertError;
use rift::services::links::models::{ClickEvent, ClickMeta, CreateLinkInput, Link, LinkStatus};
use rift::services::links::repo::LinksRepository;

#[derive(Default)]
pub struct MockLinksRepo {
    pub links: Mutex<Vec<Link>>,
    pub clicks: Mutex<Vec<ClickEvent>>,
}

#[async_trait]
impl LinksRepository for MockLinksRepo {
    async fn create_link(&self, input: CreateLinkInput) -> Result<Link, String> {
        let mut links = self.links.lock().unwrap();
        if links
            .iter()
            .any(|l| l.tenant_id.to_object_id() == input.tenant_id && l.link_id == input.link_id)
        {
            return Err("E11000 duplicate key".to_string());
        }
        let link = Link {
            id: rift::core::public_id::LinkInternalId::new(),
            tenant_id: rift::core::public_id::TenantId::from_object_id(input.tenant_id),
            link_id: input.link_id,
            ios_deep_link: input.ios_deep_link,
            android_deep_link: input.android_deep_link,
            web_url: input.web_url,
            ios_store_url: input.ios_store_url,
            android_store_url: input.android_store_url,
            metadata: input.metadata,
            affiliate_id: input
                .affiliate_id
                .map(rift::core::public_id::AffiliateId::from_object_id),
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

    async fn create_many_in_txn(
        &self,
        inputs: Vec<CreateLinkInput>,
    ) -> Result<Vec<Link>, BulkInsertError> {
        let mut links = self.links.lock().unwrap();
        let mut dupes: Vec<usize> = Vec::new();
        for (i, input) in inputs.iter().enumerate() {
            if links.iter().any(|l| {
                l.tenant_id.to_object_id() == input.tenant_id && l.link_id == input.link_id
            }) {
                dupes.push(i);
            }
        }
        for i in 0..inputs.len() {
            for j in (i + 1)..inputs.len() {
                if inputs[i].tenant_id == inputs[j].tenant_id
                    && inputs[i].link_id == inputs[j].link_id
                    && !dupes.contains(&j)
                {
                    dupes.push(j);
                }
            }
        }
        if !dupes.is_empty() {
            dupes.sort();
            return Err(BulkInsertError::DuplicateLinkIds(dupes));
        }
        let now = DateTime::now();
        let new_links: Vec<Link> = inputs
            .into_iter()
            .map(|input| Link {
                id: rift::core::public_id::LinkInternalId::new(),
                tenant_id: rift::core::public_id::TenantId::from_object_id(input.tenant_id),
                link_id: input.link_id,
                ios_deep_link: input.ios_deep_link,
                android_deep_link: input.android_deep_link,
                web_url: input.web_url,
                ios_store_url: input.ios_store_url,
                android_store_url: input.android_store_url,
                metadata: input.metadata,
                affiliate_id: input
                    .affiliate_id
                    .map(rift::core::public_id::AffiliateId::from_object_id),
                created_at: now,
                status: LinkStatus::Active,
                flag_reason: None,
                expires_at: input.expires_at,
                agent_context: input.agent_context,
                social_preview: input.social_preview,
            })
            .collect();
        links.extend(new_links.iter().cloned());
        Ok(new_links)
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
            .find(|l| l.tenant_id.to_object_id() == *tenant_id && l.link_id == link_id)
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
            .find(|l| l.tenant_id.to_object_id() == *tenant_id && l.link_id == link_id)
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
        links.retain(|l| !(l.tenant_id.to_object_id() == *tenant_id && l.link_id == link_id));
        Ok(links.len() < len_before)
    }

    async fn count_links_by_tenant(&self, tenant_id: &ObjectId) -> Result<u64, String> {
        let links = self.links.lock().unwrap();
        Ok(links
            .iter()
            .filter(|l| l.tenant_id.to_object_id() == *tenant_id)
            .count() as u64)
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
            .filter(|l| {
                l.tenant_id.to_object_id() == *tenant_id
                    && cursor.is_none_or(|c| l.id.to_object_id() < c)
            })
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
        retention_bucket: String,
    ) -> Result<(), String> {
        self.clicks.lock().unwrap().push(ClickEvent {
            meta: ClickMeta {
                tenant_id,
                link_id: link_id.to_string(),
                retention_bucket,
            },
            clicked_at: DateTime::now(),
            user_agent,
            referer,
            platform,
        });
        Ok(())
    }

    async fn record_attribute_event(
        &self,
        _tenant_id: ObjectId,
        _link_id: &str,
        _install_id: &str,
        _app_version: &str,
        _user_id: Option<&str>,
        _retention_bucket: String,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn backfill_user_id_on_attribution_events(
        &self,
        _tenant_id: &ObjectId,
        _install_id: &str,
        _user_id: &str,
    ) -> Result<u64, String> {
        Ok(0)
    }

    async fn distinct_install_ids_credited_to_links(
        &self,
        _tenant_id: &ObjectId,
        _link_ids: &[String],
        _from: DateTime,
        _to: DateTime,
        _credit: rift::services::links::models::CreditModel,
    ) -> Result<Vec<String>, String> {
        Ok(vec![])
    }

    async fn count_clicks_for_links(
        &self,
        _tenant_id: &ObjectId,
        _link_ids: &[String],
        _from: DateTime,
        _to: DateTime,
    ) -> Result<u64, String> {
        Ok(0)
    }

    async fn credited_links_for_user(
        &self,
        _tenant_id: &ObjectId,
        _user_id: &str,
        _at_or_before: DateTime,
    ) -> Result<rift::services::links::models::CreditedLinks, String> {
        Ok(Default::default())
    }
}

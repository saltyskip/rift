use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::{Collection, Database};

use super::models::{InstallContext, InstallEvent, InstallEventType};
use crate::ensure_index;

// ── Trait ──

#[async_trait]
pub trait InstallEventsRepository: Send + Sync {
    /// Write `install.created` for a fresh install_id, or `install.opened`
    /// for a known one. Returns the type that was written, so the caller
    /// (e.g. webhook dispatcher) can decide whether to fire downstream.
    async fn record_attribute_lifecycle(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        context: &InstallContext,
    ) -> Result<InstallEventType, String>;

    /// Write `install.identified` and, if the user has prior installs,
    /// also write `install.reinstalled` (same device_model) or
    /// `install.new_device` (different device_model).
    ///
    /// `prior_install_ids` is the user's install_ids BEFORE this identify
    /// added the current one. `device_model` is the current install's
    /// model from `install.created`. Both can be empty/None — the
    /// reinstall-vs-new-device split defaults to `Reinstalled` when
    /// device_model data is missing.
    async fn record_identify_lifecycle(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
        prior_install_ids: &[String],
        prior_device_models: &[String],
        current_device_model: Option<&str>,
    ) -> Result<Vec<InstallEventType>, String>;

    /// Look up the `install.created` device_model for a given install_id.
    /// Used by `record_identify_lifecycle` callers to assemble the
    /// reinstall-vs-new-device input.
    async fn get_device_model(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
    ) -> Result<Option<String>, String>;

    /// Count events of a given type within a time range, restricted to a
    /// set of install_ids. Powers each leaf of the funnel response.
    /// Returns 0 immediately if `install_ids` is empty.
    async fn count_events_by_type_for_installs(
        &self,
        tenant_id: &ObjectId,
        event_type: InstallEventType,
        install_ids: &[String],
        from: DateTime,
        to: DateTime,
    ) -> Result<u64, String>;
}

// ── Repository ──

crate::impl_container!(InstallEventsRepo);
#[derive(Clone)]
pub struct InstallEventsRepo {
    events: Collection<InstallEvent>,
}

impl InstallEventsRepo {
    pub async fn new(database: &Database) -> Self {
        let events = database.collection::<InstallEvent>("install_events");

        // Primary lookup: events for one install in time order.
        ensure_index!(
            events,
            doc! { "tenant_id": 1, "install_id": 1, "event_type": 1, "timestamp": -1 },
            "install_events_tenant_install_type_ts"
        );
        // Rollup queries: count events of type X in a time window.
        ensure_index!(
            events,
            doc! { "tenant_id": 1, "event_type": 1, "timestamp": -1 },
            "install_events_tenant_type_ts"
        );

        InstallEventsRepo { events }
    }
}

#[async_trait]
impl InstallEventsRepository for InstallEventsRepo {
    async fn record_attribute_lifecycle(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        context: &InstallContext,
    ) -> Result<InstallEventType, String> {
        // Check whether install.created has ever been written for this
        // install_id. Cheap indexed point lookup.
        let existing = self
            .events
            .find_one(doc! {
                "tenant_id": tenant_id,
                "install_id": install_id,
                "event_type": "created",
            })
            .await
            .map_err(|e| e.to_string())?;

        let event_type = if existing.is_some() {
            InstallEventType::Opened
        } else {
            InstallEventType::Created
        };

        let event = InstallEvent {
            id: Some(crate::core::public_id::InstallEventId::new()),
            tenant_id: crate::core::public_id::TenantId::from_object_id(*tenant_id),
            install_id: install_id.to_string(),
            event_type,
            timestamp: DateTime::now(),
            user_id: None,
            context: if matches!(event_type, InstallEventType::Created) {
                context.clone()
            } else {
                InstallContext::default()
            },
            prior_install_ids: None,
        };

        self.events
            .insert_one(&event)
            .await
            .map_err(|e| e.to_string())?;
        Ok(event_type)
    }

    async fn record_identify_lifecycle(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
        user_id: &str,
        prior_install_ids: &[String],
        prior_device_models: &[String],
        current_device_model: Option<&str>,
    ) -> Result<Vec<InstallEventType>, String> {
        let now = DateTime::now();
        let mut written = Vec::with_capacity(2);

        // Always write install.identified.
        let identified = InstallEvent {
            id: Some(crate::core::public_id::InstallEventId::new()),
            tenant_id: crate::core::public_id::TenantId::from_object_id(*tenant_id),
            install_id: install_id.to_string(),
            event_type: InstallEventType::Identified,
            timestamp: now,
            user_id: Some(user_id.to_string()),
            context: InstallContext::default(),
            prior_install_ids: None,
        };
        self.events
            .insert_one(&identified)
            .await
            .map_err(|e| e.to_string())?;
        written.push(InstallEventType::Identified);

        // If the user has prior installs (not counting this one), classify
        // as reinstall (same device model) or new_device (different).
        // Missing device_model data defaults to Reinstalled — historically
        // the more common case, and we avoid a misleading "new device"
        // classification when we genuinely don't know.
        let prior_excluding_self: Vec<&String> = prior_install_ids
            .iter()
            .filter(|i| *i != install_id)
            .collect();

        if !prior_excluding_self.is_empty() {
            let classification = match current_device_model {
                Some(m) if prior_device_models.iter().any(|p| p == m) => {
                    InstallEventType::Reinstalled
                }
                Some(_) => InstallEventType::NewDevice,
                None => InstallEventType::Reinstalled,
            };

            let event = InstallEvent {
                id: Some(crate::core::public_id::InstallEventId::new()),
                tenant_id: crate::core::public_id::TenantId::from_object_id(*tenant_id),
                install_id: install_id.to_string(),
                event_type: classification,
                timestamp: now,
                user_id: Some(user_id.to_string()),
                context: InstallContext::default(),
                prior_install_ids: Some(
                    prior_excluding_self.iter().map(|s| s.to_string()).collect(),
                ),
            };
            self.events
                .insert_one(&event)
                .await
                .map_err(|e| e.to_string())?;
            written.push(classification);
        }

        Ok(written)
    }

    async fn get_device_model(
        &self,
        tenant_id: &ObjectId,
        install_id: &str,
    ) -> Result<Option<String>, String> {
        let event = self
            .events
            .find_one(doc! {
                "tenant_id": tenant_id,
                "install_id": install_id,
                "event_type": "created",
            })
            .await
            .map_err(|e| e.to_string())?;
        Ok(event.and_then(|e| e.context.device_model))
    }

    async fn count_events_by_type_for_installs(
        &self,
        tenant_id: &ObjectId,
        event_type: InstallEventType,
        install_ids: &[String],
        from: DateTime,
        to: DateTime,
    ) -> Result<u64, String> {
        if install_ids.is_empty() {
            return Ok(0);
        }
        let bson_ids: Vec<mongodb::bson::Bson> = install_ids
            .iter()
            .map(|s| mongodb::bson::Bson::String(s.clone()))
            .collect();
        let type_str = match event_type {
            InstallEventType::Created => "created",
            InstallEventType::Opened => "opened",
            InstallEventType::Identified => "identified",
            InstallEventType::Reinstalled => "reinstalled",
            InstallEventType::NewDevice => "new_device",
        };
        self.events
            .count_documents(doc! {
                "tenant_id": tenant_id,
                "event_type": type_str,
                "install_id": { "$in": bson_ids },
                "timestamp": { "$gte": from, "$lte": to },
            })
            .await
            .map_err(|e| e.to_string())
    }
}

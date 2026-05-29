use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::options::{IndexOptions, ReturnDocument};
use mongodb::{Collection, Database};

use super::models::{AppUserDoc, AppUserUpsert};
use crate::core::public_id::TenantId;
use crate::ensure_index;

// ── Trait ──

#[async_trait]
pub trait AppUsersRepository: Send + Sync {
    /// Upsert the identity row for `user_id` and ensure `install_id` is in
    /// `install_ids`. Three outcomes:
    /// - `Created` — first identify ever for this user.
    /// - `InstallAdded` — user existed; this is a new install (multi-device
    ///   or reinstall).
    /// - `AlreadyPresent` — user existed with this install already bound.
    async fn upsert_with_install(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        install_id: &str,
    ) -> Result<AppUserUpsert, String>;

    /// Lookup the identity row for a user_id. Used by the identify
    /// orchestrator to capture `prior_install_ids` before the new install
    /// gets added, and by the journey endpoint to gather the user's
    /// devices.
    async fn find_by_user_id(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
    ) -> Result<Option<AppUserDoc>, String>;

    /// Resolve a single install_id back to the user_id it's bound to (if
    /// any). Used by `record_attribute_event` to stamp `user_id` onto the
    /// time-series row at write time — eliminating the OR-clause on the
    /// read side and removing the need for a separate `installs` collection.
    async fn find_user_id_for_install(
        &self,
        tenant_id: &TenantId,
        install_id: &str,
    ) -> Result<Option<String>, String>;
}

// ── Repository ──

crate::impl_container!(AppUsersRepo);
#[derive(Clone)]
pub struct AppUsersRepo {
    app_users: Collection<AppUserDoc>,
}

impl AppUsersRepo {
    pub async fn new(database: &Database) -> Self {
        let app_users = database.collection::<AppUserDoc>("app_users");

        ensure_index!(
            app_users,
            doc! { "tenant_id": 1, "user_id": 1 },
            IndexOptions::builder().unique(true).build(),
            "app_users_tenant_user_unique"
        );

        ensure_index!(
            app_users,
            doc! { "tenant_id": 1, "install_ids": 1 },
            "app_users_tenant_install_ids"
        );

        AppUsersRepo { app_users }
    }
}

#[async_trait]
impl AppUsersRepository for AppUsersRepo {
    async fn upsert_with_install(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        install_id: &str,
    ) -> Result<AppUserUpsert, String> {
        let now = DateTime::now();
        let before = self
            .app_users
            .find_one_and_update(
                doc! { "tenant_id": tenant_id, "user_id": user_id },
                doc! {
                    "$setOnInsert": {
                        "_id": ObjectId::new(),
                        "tenant_id": tenant_id,
                        "user_id": user_id,
                        "identified_at": now,
                    },
                    "$addToSet": { "install_ids": install_id },
                    "$set": { "last_seen_at": now },
                },
            )
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await
            .map_err(|e| e.to_string())?;

        Ok(match before {
            None => AppUserUpsert::Created,
            Some(prev) if prev.install_ids.iter().any(|i| i == install_id) => {
                AppUserUpsert::AlreadyPresent
            }
            Some(_) => AppUserUpsert::InstallAdded,
        })
    }

    async fn find_by_user_id(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
    ) -> Result<Option<AppUserDoc>, String> {
        self.app_users
            .find_one(doc! { "tenant_id": tenant_id, "user_id": user_id })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_user_id_for_install(
        &self,
        tenant_id: &TenantId,
        install_id: &str,
    ) -> Result<Option<String>, String> {
        let doc = self
            .app_users
            .find_one(doc! { "tenant_id": tenant_id, "install_ids": install_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(doc.map(|d| d.user_id))
    }
}

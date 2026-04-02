use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::Database;

pub struct M001AuthSplit;

#[async_trait]
impl super::Migration for M001AuthSplit {
    fn id(&self) -> &'static str {
        "m001_auth_split"
    }

    fn description(&self) -> &'static str {
        "Split api_keys into tenants + users + secret_keys collections"
    }

    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String> {
        let api_keys = db.collection::<Document>("api_keys");
        let tenants = db.collection::<Document>("tenants");
        let users = db.collection::<Document>("users");
        let secret_keys = db.collection::<Document>("secret_keys");

        let mut cursor = api_keys
            .find(doc! {})
            .await
            .map_err(|e| format!("Failed to query api_keys: {e}"))?;

        let mut migrated = 0u64;
        let mut skipped = 0u64;

        while let Some(api_key_doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("Cursor error: {e}"))?
        {
            let old_id = match api_key_doc.get_object_id("_id") {
                Ok(id) => id,
                Err(_) => {
                    eprintln!("  Skipping document with missing _id");
                    skipped += 1;
                    continue;
                }
            };

            let email = match api_key_doc.get_str("email") {
                Ok(e) => e.to_string(),
                Err(_) => {
                    eprintln!("  Skipping {old_id}: missing email");
                    skipped += 1;
                    continue;
                }
            };

            let verified = api_key_doc.get_bool("verified").unwrap_or(false);
            let key_hash = api_key_doc.get_str("key_hash").unwrap_or("").to_string();
            let key_prefix = api_key_doc.get_str("key_prefix").unwrap_or("").to_string();
            let monthly_quota = api_key_doc.get_i64("monthly_quota").unwrap_or(100);
            let created_at = api_key_doc
                .get_datetime("created_at")
                .cloned()
                .unwrap_or_else(|_| DateTime::now());

            // Check if already migrated (tenant with this _id exists)
            if tenants
                .find_one(doc! { "_id": old_id })
                .await
                .map_err(|e| format!("Tenant lookup failed: {e}"))?
                .is_some()
            {
                skipped += 1;
                continue;
            }

            let has_key = verified && !key_hash.is_empty();

            if dry_run {
                println!(
                    "  Would migrate: {email} (verified={verified}, has_key={has_key}, quota={monthly_quota})"
                );
                migrated += 1;
                continue;
            }

            // Reuse old ApiKeyDoc._id as tenant _id — all existing resources
            // (links, domains, webhooks, sdk_keys) already reference this ID.
            let tenant_doc = doc! {
                "_id": old_id,
                "monthly_quota": monthly_quota,
                "created_at": created_at,
            };

            tenants
                .insert_one(&tenant_doc)
                .await
                .map_err(|e| format!("Failed to create tenant for {email}: {e}"))?;

            let user_id = ObjectId::new();
            let user_doc = doc! {
                "_id": user_id,
                "tenant_id": old_id,
                "email": &email,
                "verified": verified,
                "is_owner": true,
                "created_at": created_at,
            };

            users
                .insert_one(&user_doc)
                .await
                .map_err(|e| format!("Failed to create user for {email}: {e}"))?;

            // Only create a secret key if the account was verified and has a key hash
            if has_key {
                let sk_doc = doc! {
                    "_id": ObjectId::new(),
                    "tenant_id": old_id,
                    "created_by": user_id,
                    "key_hash": &key_hash,
                    "key_prefix": &key_prefix,
                    "created_at": created_at,
                };

                secret_keys
                    .insert_one(&sk_doc)
                    .await
                    .map_err(|e| format!("Failed to create secret key for {email}: {e}"))?;
            }

            migrated += 1;
            println!("  Migrated: {email} (verified={verified})");
        }

        println!("\nDone. Migrated: {migrated}, Skipped: {skipped}");
        Ok(())
    }
}

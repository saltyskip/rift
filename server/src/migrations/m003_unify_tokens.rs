//! m003_unify_tokens — one-shot destructive cutover to the unified `tokens`
//! collection.
//!
//! Three legacy places to remove:
//!   1. `users.verify_token` + `users.verify_token_expires_at` — unset on
//!      every row that has them. Users with in-flight verification links
//!      will need to re-request; product is days old, zero deployed cost.
//!   2. `secret_key_create_requests` collection — drop entirely.
//!   3. `billing_magic_links` collection — drop entirely.
//!
//! The new `tokens` collection + its indexes materialize on first server
//! boot via `TokensRepoMongo::new`. No schema bootstrapping needed here.
//!
//! Dry run counts the rows that would change. Apply actually writes.

use async_trait::async_trait;
use mongodb::bson::{doc, Document};
use mongodb::Database;

crate::impl_container!(M003UnifyTokens);
pub struct M003UnifyTokens;

#[async_trait]
impl super::Migration for M003UnifyTokens {
    fn id(&self) -> &'static str {
        "m003_unify_tokens"
    }

    fn description(&self) -> &'static str {
        "Retire users.verify_token + secret_key_create_requests + billing_magic_links in favor of the unified `tokens` collection"
    }

    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String> {
        let users = db.collection::<Document>("users");
        let sk_requests = db.collection::<Document>("secret_key_create_requests");
        let magic_links = db.collection::<Document>("billing_magic_links");

        let users_with_token = users
            .count_documents(doc! { "verify_token": { "$exists": true } })
            .await
            .map_err(|e| format!("count users.verify_token: {e}"))?;

        let sk_request_count = sk_requests
            .count_documents(doc! {})
            .await
            .map_err(|e| format!("count secret_key_create_requests: {e}"))?;

        let magic_link_count = magic_links
            .count_documents(doc! {})
            .await
            .map_err(|e| format!("count billing_magic_links: {e}"))?;

        if dry_run {
            println!(
                "  Would unset verify_token/verify_token_expires_at on {users_with_token} user(s)"
            );
            println!("  Would drop secret_key_create_requests ({sk_request_count} doc(s))");
            println!("  Would drop billing_magic_links ({magic_link_count} doc(s))");
            println!(
                "  (In-flight tokens die — users re-request email verification / rotation / billing magic links)"
            );
            return Ok(());
        }

        let users_result = users
            .update_many(
                doc! { "verify_token": { "$exists": true } },
                doc! {
                    "$unset": {
                        "verify_token": "",
                        "verify_token_expires_at": "",
                    }
                },
            )
            .await
            .map_err(|e| format!("unset verify_token: {e}"))?;
        println!(
            "  Unset verify_token on {} user(s) (matched {})",
            users_result.modified_count, users_result.matched_count
        );

        // `drop` is destructive; it returns Ok even if the collection
        // doesn't exist (which is the case on a fresh DB after this
        // migration's second run — desired idempotency).
        sk_requests
            .drop()
            .await
            .map_err(|e| format!("drop secret_key_create_requests: {e}"))?;
        println!("  Dropped secret_key_create_requests");

        magic_links
            .drop()
            .await
            .map_err(|e| format!("drop billing_magic_links: {e}"))?;
        println!("  Dropped billing_magic_links");

        Ok(())
    }
}

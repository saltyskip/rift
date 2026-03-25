use std::sync::Arc;

use rmcp::ServiceExt;

use rift::api::auth::keys;
use rift::api::auth::repo::{AuthRepo, AuthRepository};
use rift::api::links::repo::LinksRepo;
use rift::api::links::service::LinksService;
use rift::core::threat_feed::ThreatFeed;
use rift::mcp::RiftMcp;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let api_key = std::env::var("RIFT_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("Error: RIFT_API_KEY environment variable is required");
        std::process::exit(1);
    }

    let mongo_uri = std::env::var("MONGO_URI").unwrap_or_default();
    if mongo_uri.is_empty() {
        eprintln!("Error: MONGO_URI environment variable is required");
        std::process::exit(1);
    }

    let mongo_db = std::env::var("MONGO_DB").unwrap_or_else(|_| "rift".to_string());
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "https://riftl.ink".to_string());

    // Connect to MongoDB.
    let database = rift::core::db::connect(&mongo_uri, &mongo_db)
        .await
        .unwrap_or_else(|| {
            eprintln!("Error: Failed to connect to MongoDB at {mongo_uri}");
            std::process::exit(1);
        });

    // Authenticate: hash the API key and look up the tenant.
    let auth_repo = AuthRepo::new(&database).await;
    let key_hash = keys::hash_key(&api_key);
    let key_doc = auth_repo
        .find_key_by_hash(&key_hash)
        .await
        .unwrap_or_else(|| {
            eprintln!("Error: Invalid or unverified API key");
            std::process::exit(1);
        });
    let tenant_id = key_doc.id.unwrap_or_else(mongodb::bson::oid::ObjectId::new);

    // Build the links service.
    let links_repo = Arc::new(LinksRepo::new(&database).await);
    let threat_feed = ThreatFeed::new();
    threat_feed.clone().start_background_refresh(30 * 60);

    let service = Arc::new(LinksService::new(
        links_repo,
        None, // No domains repo — custom IDs will fail gracefully.
        threat_feed,
        public_url,
    ));

    let server = RiftMcp::new(service, tenant_id);

    // Start stdio transport.
    let transport = rmcp::transport::io::stdio();
    let running = server.serve(transport).await.unwrap_or_else(|e| {
        eprintln!("Error: Failed to start MCP server: {e}");
        std::process::exit(1);
    });

    running.waiting().await.unwrap_or_else(|e| {
        eprintln!("Error: MCP server error: {e}");
        std::process::exit(1);
    });
}

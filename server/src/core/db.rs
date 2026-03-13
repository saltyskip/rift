use mongodb::{Client, Database};

/// Connect to MongoDB and return the database handle.
/// Collection setup and indexes are owned by each slice's repository.
pub async fn connect(uri: &str, db_name: &str) -> Option<Database> {
    match Client::with_uri_str(uri).await {
        Ok(client) => Some(client.database(db_name)),
        Err(e) => {
            tracing::error!("MongoDB connection failed: {e}");
            None
        }
    }
}

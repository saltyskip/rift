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

/// Create a MongoDB index, logging on failure.
#[macro_export]
macro_rules! ensure_index {
    ($col:expr, $keys:expr, $opts:expr, $name:expr) => {
        if let Err(e) = $col
            .create_index(
                mongodb::IndexModel::builder()
                    .keys($keys)
                    .options($opts)
                    .build(),
            )
            .await
        {
            tracing::error!(index = $name, "Failed to create index: {e}");
        }
    };
    ($col:expr, $keys:expr, $name:expr) => {
        if let Err(e) = $col
            .create_index(mongodb::IndexModel::builder().keys($keys).build())
            .await
        {
            tracing::error!(index = $name, "Failed to create index: {e}");
        }
    };
}

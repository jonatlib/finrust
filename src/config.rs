use crate::schemas::AppState;
use anyhow::Result;
use moka::future::Cache;
use sea_orm::Database;
use std::time::Duration;

/// Initialize application configuration and state with provided database URL
pub async fn initialize_app_state_with_url(database_url: &str) -> Result<AppState> {
    // Load configuration
    dotenvy::dotenv().ok();

    // Connect to database
    tracing::info!("Connecting to database: {}", database_url);
    let db = Database::connect(database_url).await?;

    // Initialize cache
    let cache = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300)) // 5 minutes
        .build();

    Ok(AppState { db, cache })
}

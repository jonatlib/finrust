use anyhow::Result;
use moka::future::Cache;
use sea_orm::Database;
use std::time::Duration;
use crate::schemas::AppState;

/// Initialize application configuration and state
pub async fn initialize_app_state() -> Result<AppState> {
    // Load configuration
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://finrust.db".to_string());

    // Connect to database
    tracing::info!("Connecting to database: {}", database_url);
    let db = Database::connect(&database_url).await?;

    // Initialize cache
    let cache = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300)) // 5 minutes
        .build();

    Ok(AppState { db, cache })
}

/// Get bind address from environment or use default
pub fn get_bind_address() -> String {
    std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
}

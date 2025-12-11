use crate::schemas::AppState;
use anyhow::Result;
use moka::future::Cache;
use sea_orm::Database;
use std::time::Duration;
use tracing::{debug, error, info, trace};

/// Initialize application configuration and state with provided database URL
pub async fn initialize_app_state_with_url(database_url: &str) -> Result<AppState> {
    trace!("Entering initialize_app_state_with_url function");
    info!("Initializing application state");
    debug!("Database URL: {}", database_url);

    // Load configuration
    trace!("Loading environment variables from .env file");
    match dotenvy::dotenv() {
        Ok(path) => debug!("Loaded environment variables from: {:?}", path),
        Err(_) => debug!("No .env file found or failed to load, using system environment variables"),
    }

    // Connect to database
    info!("Connecting to database");
    trace!("Attempting database connection to: {}", database_url);
    let db = match Database::connect(database_url).await {
        Ok(connection) => {
            info!("Successfully connected to database");
            debug!("Database connection established and ready");
            connection
        }
        Err(e) => {
            error!("Failed to connect to database '{}': {}", database_url, e);
            return Err(e.into());
        }
    };

    // Initialize cache
    trace!("Initializing application cache");
    let cache = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(5)) // 5 minutes
        .build();
    debug!("Cache initialized with max_capacity=1000, ttl=5s");

    let app_state = AppState { db, cache };
    info!("Application state initialized successfully");
    trace!("initialize_app_state_with_url function completed");

    Ok(app_state)
}

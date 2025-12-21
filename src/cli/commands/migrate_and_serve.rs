use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{debug, error, info, trace};

use crate::config::initialize_app_state_with_url;
use crate::router::create_router;

pub async fn migrate_and_serve(database_url: &str, bind_address: &str) -> Result<()> {
    trace!("Entering migrate_and_serve function");
    info!("Applying database migrations and starting server");
    debug!("Database URL: {}", database_url);
    debug!("Bind address: {}", bind_address);

    // Apply migrations
    trace!("Attempting to connect to database for migrations");
    let db = match Database::connect(database_url).await {
        Ok(connection) => {
            info!("Successfully connected to database");
            debug!("Database connection established");
            connection
        }
        Err(e) => {
            error!("Failed to connect to database '{}': {}", database_url, e);
            return Err(e.into());
        }
    };

    info!("Running database migrations");
    trace!("Executing migration up command");
    match Migrator::up(&db, None).await {
        Ok(_) => {
            info!("Database migrations completed successfully");
            debug!("All pending migrations have been applied");
        }
        Err(e) => {
            error!("Failed to run database migrations: {}", e);
            return Err(e.into());
        }
    }

    // Initialize application state
    trace!("Initializing application state");
    let state = match initialize_app_state_with_url(database_url).await {
        Ok(state) => {
            debug!("Application state initialized successfully");
            state
        }
        Err(e) => {
            error!("Failed to initialize application state: {}", e);
            return Err(e);
        }
    };

    // Create router
    trace!("Creating application router");
    let app = create_router(state);
    debug!("Router created successfully");

    // Start server
    info!("Starting server on {}", bind_address);
    trace!("Attempting to bind TCP listener to {}", bind_address);
    let listener = match TcpListener::bind(&bind_address).await {
        Ok(listener) => {
            debug!("Successfully bound to address: {}", bind_address);
            listener
        }
        Err(e) => {
            error!("Failed to bind to address {}: {}", bind_address, e);
            return Err(e.into());
        }
    };

    info!("FinRust API server running on http://{}", bind_address);
    info!("Swagger UI available at http://{}/swagger-ui", bind_address);
    debug!("Server is ready to accept connections");

    trace!("Starting axum server");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            e
        })?;

    info!("Server shutdown gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C), shutting down gracefully");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully");
        },
    }
}

use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{info, debug, trace, error};

use crate::config::initialize_app_state_with_url;
use crate::router::create_router;

pub async fn serve(database_url: &str, bind_address: &str) -> Result<()> {
    trace!("Entering serve function");
    info!("FinRust application starting up");
    debug!("Database URL: {}", database_url);
    debug!("Bind address: {}", bind_address);

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
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    info!("Server shutdown gracefully");
    Ok(())
}

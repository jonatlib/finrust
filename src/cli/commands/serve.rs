use anyhow::Result;
use tokio::net::TcpListener;
use tracing::info;

use crate::config::{get_bind_address, initialize_app_state};
use crate::router::create_router;

pub async fn serve() -> Result<()> {
    info!("FinRust application starting up");

    // Initialize application state
    let state = initialize_app_state().await?;

    // Create router
    let app = create_router(state);

    // Get bind address
    let bind_address = get_bind_address();

    // Start server
    info!("Starting server on {}", bind_address);
    let listener = TcpListener::bind(&bind_address).await?;

    info!("FinRust API server running on http://{}", bind_address);
    info!("Swagger UI available at http://{}/swagger-ui", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}
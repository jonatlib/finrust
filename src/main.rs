use anyhow::Result;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod schemas;
mod handlers;
mod helpers;
mod router;
mod config;

use config::{initialize_app_state, get_bind_address};
use router::create_router;



/// Main entry point for the FinRust application.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "finrust=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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

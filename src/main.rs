use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

/// Main entry point for the FinRust application.
///
/// Sets up the tracing subscriber for logging and starts the application.
fn main() {
    // Initialize the tracing subscriber for logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    // Log application startup
    info!("FinRust application starting up");
}

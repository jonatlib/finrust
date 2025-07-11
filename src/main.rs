use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod config;
mod handlers;
mod helpers;
mod router;
mod schemas;

use cli::Cli;

/// Main entry point for the FinRust application.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing before clap parsing (shared for all sub-commands)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "finrust=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse CLI arguments and run the appropriate command
    let cli = Cli::parse();
    cli.run().await?;

    Ok(())
}

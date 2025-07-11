use clap::Parser;
use anyhow::Result;

mod cli;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    cli.run().await?;

    Ok(())
}

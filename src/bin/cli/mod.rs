use clap::{Parser, Subcommand};
use anyhow::Result;

pub mod commands;

use commands::init_database;

#[derive(Parser)]
#[command(name = "finrust-cli")]
#[command(about = "FinRust CLI tool for database management and other operations")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize the database using migrations
    InitDb {
        /// Database URL
        #[arg(short, long, env = "DATABASE_URL")]
        database_url: String,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::InitDb { database_url } => {
                init_database(&database_url).await?;
            }
        }
        Ok(())
    }
}
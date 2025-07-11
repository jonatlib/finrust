use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod commands;

use commands::{init_database, serve};

#[derive(Parser)]
#[command(name = "finrust")]
#[command(about = "FinRust application with CLI tools and web server")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the web server
    Serve,
    /// Initialize the database using migrations
    ///
    /// Examples:
    ///   SQLite: sqlite:///path/to/database.sqlite
    ///   PostgreSQL: postgresql://user:password@localhost/dbname
    ///   MySQL: mysql://user:password@localhost/dbname
    InitDb {
        /// Database URL
        ///
        /// For SQLite databases, use:
        ///   - sqlite:///absolute/path/to/database.sqlite (absolute path)
        ///
        /// The parent directory will be created automatically if it doesn't exist.
        #[arg(short, long, env = "DATABASE_URL")]
        database_url: String,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Serve => {
                serve().await?;
            }
            Commands::InitDb { database_url } => {
                init_database(&database_url).await?;
            }
        }
        Ok(())
    }
}

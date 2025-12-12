use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod commands;

use commands::{import_django, init_database, serve};

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
    Serve {
        /// Database URL
        ///
        /// For SQLite databases, use:
        ///   - sqlite:///absolute/path/to/database.sqlite (absolute path)
        ///
        /// Examples:
        ///   SQLite: sqlite:///path/to/database.sqlite
        ///   PostgreSQL: postgresql://user:password@localhost/dbname
        ///   MySQL: mysql://user:password@localhost/dbname
        #[arg(short, long, env = "DATABASE_URL", default_value = "sqlite://finrust.db")]
        database_url: String,

        /// Bind address for the web server
        ///
        /// Format: IP:PORT (e.g., 0.0.0.0:3000, 127.0.0.1:8080)
        #[arg(short, long, env = "BIND_ADDRESS", default_value = "0.0.0.0:3000")]
        bind_address: String,
    },
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
    /// Import data from Django JSON dump
    ///
    /// Imports accounts, categories, tags, recurring transactions,
    /// one-off transactions, and manual account states from a Django
    /// application dump.
    ImportDjango {
        /// Path to the Django JSON dump file
        #[arg(short, long)]
        json_path: String,

        /// Database URL
        ///
        /// For SQLite databases, use:
        ///   - sqlite:///absolute/path/to/database.sqlite (absolute path)
        ///
        /// Examples:
        ///   SQLite: sqlite:///path/to/database.sqlite
        ///   PostgreSQL: postgresql://user:password@localhost/dbname
        ///   MySQL: mysql://user:password@localhost/dbname
        #[arg(short, long, env = "DATABASE_URL", default_value = "sqlite://finrust.db")]
        database_url: String,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Serve { database_url, bind_address } => {
                serve(&database_url, &bind_address).await?;
            }
            Commands::InitDb { database_url } => {
                init_database(&database_url).await?;
            }
            Commands::ImportDjango { json_path, database_url } => {
                import_django(&json_path, &database_url).await?;
            }
        }
        Ok(())
    }
}

use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tracing::{info, debug, trace, error};

pub async fn init_database(database_url: &str) -> Result<()> {
    trace!("Entering init_database function");
    info!("Initializing database");
    debug!("Database URL: {}", database_url);

    trace!("Attempting to connect to database");
    let db: DatabaseConnection = match Database::connect(database_url).await {
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

    info!("Database initialization completed successfully!");
    trace!("init_database function completed");

    Ok(())
}

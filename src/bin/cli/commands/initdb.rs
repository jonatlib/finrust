use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use anyhow::Result;

pub async fn init_database(database_url: &str) -> Result<()> {
    println!("Connecting to database: {}", database_url);
    
    let db: DatabaseConnection = Database::connect(database_url).await?;
    
    println!("Running migrations...");
    Migrator::up(&db, None).await?;
    
    println!("Database initialization completed successfully!");
    
    Ok(())
}
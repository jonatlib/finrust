pub mod scenario_balance;

pub use scenario_balance::ScenarioBalance;

use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};

use migration::{Migrator, MigratorTrait};
use model::entities::account;

async fn setup_db() -> Result<DatabaseConnection, DbErr> {
    // Connect to the SQLite database
    let db = Database::connect("sqlite::memory:").await?;

    // Enable foreign keys
    db.execute_unprepared("PRAGMA foreign_keys = ON;").await?;

    // Try to apply migrations first
    Migrator::up(&db, None).await.expect("Migrations failed.");
    Ok(db)
}

/// Type representing the expected result of a test scenario.
/// in the following schema (account_id, date, expected balance)
pub type AssertResult = Vec<(i32, NaiveDate, Decimal)>;

/// Prepared test scenario.
pub type TestScenario = (DatabaseConnection, Vec<account::Model>, AssertResult);

/// Trait for building test scenarios.
#[async_trait]
pub trait TestScenarioBuilder {
    async fn get_scenario(&self) -> Result<TestScenario, DbErr>;
}

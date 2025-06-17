pub mod scenario_balance;

pub use scenario_balance::ScenarioBalance;

use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};

use crate::account::AccountStateCalculator;
use crate::error::Result as ComputeResult;
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

pub async fn run_and_assert_scenario(builder: &dyn TestScenarioBuilder, computer: &dyn AccountStateCalculator) -> ComputeResult<()> {
    let (db, accounts, assert_result) = builder.get_scenario().await?;

    let min_date = assert_result.iter().map(|v| v.1.to_owned()).min().unwrap();
    let max_date = assert_result.iter().map(|v| v.1.to_owned()).max().unwrap();

    let computer_result = computer.compute_account_state(
        &db,
        &accounts,
        min_date,
        max_date,
    ).await?;

    println!("{:#?}", computer_result);

    Ok(())
}

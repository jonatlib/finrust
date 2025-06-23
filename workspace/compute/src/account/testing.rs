pub mod helpers;
pub mod scenario_balance;
pub mod scenario_balance_no_instances;
pub mod scenario_forecast;
pub mod scenario_forecast_no_instances;
pub mod scenario_merge_real;
pub mod scenario_merge_real_failing;
pub mod scenario_multiple_accounts;

pub use scenario_balance::ScenarioBalance;
pub use scenario_balance_no_instances::ScenarioBalanceNoInstances;
pub use scenario_forecast::ScenarioForecast;
pub use scenario_forecast_no_instances::ScenarioForecastNoInstances;
pub use scenario_merge_real::ScenarioMergeReal;
pub use scenario_merge_real_failing::ScenarioMergeRealFailing;
pub use scenario_multiple_accounts::ScenarioMultipleAccounts;

use async_trait::async_trait;
use chrono::NaiveDate;
use polars::prelude::*;
use polars::prelude::{col, lit};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};
use tracing::{debug, error, info};

use crate::account::AccountStateCalculator;
use crate::error::{ComputeError, Result as ComputeResult};
use migration::{Migrator, MigratorTrait};
use model::entities::account;

/// Sets up an in-memory SQLite database for testing.
///
/// Creates a new in-memory SQLite database, enables foreign keys,
/// and applies all migrations.
///
/// # Returns
///
/// A connection to the in-memory database.
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

/// Runs a test scenario and asserts that the computed results match the expected results.
///
/// This function takes a test scenario builder, an account state calculator, and a flag
/// indicating whether to use the full date range from the scenario or a narrower range.
/// It computes the account state using the calculator and asserts that the results
/// match the expected values from the scenario.
///
/// # Arguments
///
/// * `builder` - The test scenario builder that provides the test data
/// * `computer` - The account state calculator to test
/// * `use_scenario_date_range` - If true, uses the full date range from the scenario;
///   if false, uses a narrower range to test edge cases
///
/// # Returns
///
/// A result indicating success or an error with details about the failure
pub async fn run_and_assert_scenario(
    builder: &dyn TestScenarioBuilder,
    computer: &dyn AccountStateCalculator,
    use_scenario_date_range: bool,
) -> ComputeResult<()> {
    let (db, accounts, assert_result) = builder.get_scenario().await?;

    let min_date;
    let max_date;

    if use_scenario_date_range {
        min_date = assert_result.iter().map(|v| v.1.to_owned()).min().unwrap()
            - chrono::Duration::days(55);
        max_date = assert_result.iter().map(|v| v.1.to_owned()).max().unwrap()
            + chrono::Duration::days(55);
    } else {
        min_date = assert_result.iter().map(|v| v.1.to_owned()).min().unwrap()
            + chrono::Duration::days(10);
        max_date = assert_result.iter().map(|v| v.1.to_owned()).max().unwrap()
            - chrono::Duration::days(10);
    }

    if min_date >= max_date {
        error!("No date range selected, falling back to default");
        return Err(ComputeError::Date("Date range outside datapoints".to_owned()));
    }

    let mut computer_result = computer
        .compute_account_state(&db, &accounts, min_date, max_date)
        .await?;
    computer_result
        .sort_in_place(vec!["date"], SortMultipleOptions::new())
        .expect("Failed to sort result.");

    debug!("Computer result: {:#?}", computer_result);

    // Filter assertion results to only include dates within the requested date range
    // when use_scenario_date_range is false
    let filtered_assert_result = if use_scenario_date_range {
        assert_result
    } else {
        assert_result
            .into_iter()
            .filter(|(_, date, _)| *date >= min_date && *date <= max_date)
            .collect()
    };

    if filtered_assert_result.is_empty() {
        error!("No results to assert.");
        return Err(ComputeError::DataFrame(
            "No data returned for assertation".to_owned(),
        ));
    }
    assert_results(db, filtered_assert_result, computer_result).await?;

    Ok(())
}

/// Asserts that the computed results match the expected results.
///
/// This function compares each expected result (account_id, date, balance) with the
/// corresponding row in the computed results DataFrame. It verifies that exactly one
/// matching row exists for each expected result and that the balance values match.
///
/// # Arguments
///
/// * `db` - The database connection (consumed by this function)
/// * `assert_result` - The expected results as a vector of (account_id, date, balance) tuples
/// * `computer_result` - The computed results as a DataFrame
///
/// # Returns
///
/// A result indicating success or an error with details about the failure
async fn assert_results(
    db: DatabaseConnection,
    assert_result: AssertResult,
    computer_result: DataFrame,
) -> ComputeResult<()> {
    info!("Asserting results...");
    for (account_id, date, expected_balance) in assert_result {
        // Use lazy evaluation to find matching rows
        let account_id_str = account_id.to_string();
        let date_str = date.to_string();

        // Create a lazy DataFrame and apply filters
        let filtered_df = computer_result
            .clone()
            .lazy()
            .filter(
                col("account_id")
                    .cast(DataType::String)
                    .eq(lit(account_id_str))
                    .and(col("date").cast(DataType::String).eq(lit(date_str))),
            )
            .collect()?;

        // Check that exactly one row is found
        if filtered_df.height() != 1 {
            return Err(ComputeError::DataFrame(format!(
                "Expected exactly one row for account_id={}, date={}, but found {}",
                account_id,
                date,
                filtered_df.height()
            )));
        }

        // Extract the balance value from the filtered DataFrame
        let balance_str = filtered_df.column("balance")?.get(0).unwrap().str_value();
        let actual_balance = balance_str.parse::<Decimal>().map_err(|e| {
            ComputeError::DataFrame(format!("Failed to parse balance '{}': {}", balance_str, e))
        })?;

        // Assert that the balance equals the expected balance
        if actual_balance != expected_balance {
            return Err(ComputeError::DataFrame(format!(
                "Balance mismatch for account_id={}, date={}: expected {}, got {}",
                account_id, date, expected_balance, actual_balance
            )));
        }

        info!(
            "Assertion passed for account_id={}, date={}: balance={}",
            account_id, date, actual_balance
        );
    }

    Ok(())
}

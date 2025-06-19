use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use model::entities::account;
use polars::prelude::DataFrame;
use sea_orm::DatabaseConnection;
use tracing::{debug, instrument, trace};

use crate::error::Result;

/// Returns the number of days in the given month using chrono.
#[instrument]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    trace!(
        "Calculating days in month for year={}, month={}",
        year, month
    );

    // Create a date for the first day of the next month
    let next_month_year = year + (month / 12) as i32;
    let next_month = (month % 12) + 1;
    trace!(
        "Calculated next month: year={}, month={}",
        next_month_year, next_month
    );

    // Get the first day of the next month
    let first_day_next_month = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap();
    trace!("First day of next month: {}", first_day_next_month);

    // Go back one day to get the last day of the current month
    let last_day_current_month = first_day_next_month.pred_opt().unwrap();
    trace!("Last day of current month: {}", last_day_current_month);

    // The day of the month is the number of days in the month
    let days = last_day_current_month.day();
    debug!("Days in month {}-{}: {}", year, month, days);
    days
}

/// Enum defining the strategy for merging account states from different calculators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    /// Use the first calculator that returns a value for a given date.
    /// Other values for the same date will be logged with a warning and discarded.
    FirstWins,

    /// Sum the amounts from different calculators for each date.
    Sum,
}

/// Trait for standardizing account state calculation.
#[async_trait]
pub trait AccountStateCalculator {
    /// Computes the account state for the given accounts within the specified date range.
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame>;

    /// Returns the merge method to use when combining results from multiple calculators.
    fn merge_method(&self) -> MergeMethod;
}

pub mod balance;
pub mod forecast;
pub mod merge;
pub mod utils;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[tokio::test]
    async fn test_scenario_balance() {
        let scenario = ScenarioBalance::new();
        let computer = balance::BalanceCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_forecast_with_balance_calculator() {
        let scenario = ScenarioForecast::new();
        let computer = balance::BalanceCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_forecast() {
        let scenario = ScenarioForecast::new();
        let computer = forecast::ForecastCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_balance_merge_simple() {
        let scenario = ScenarioBalance::new();
        let computer1 = Box::new(balance::BalanceCalculator::new(MergeMethod::FirstWins));
        let computer2 = Box::new(forecast::ForecastCalculator::new(MergeMethod::FirstWins));

        let computer = merge::MergeCalculator::new(vec![computer1, computer2], MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer)
            .await
            .expect("Failed to run scenario");
    }
}

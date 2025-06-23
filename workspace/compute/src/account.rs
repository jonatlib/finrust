use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use polars::prelude::DataFrame;
use sea_orm::DatabaseConnection;
use tracing::{debug, instrument, trace};

use model::entities::account;

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
///
/// When multiple account state calculators are used together (e.g., in a MergeCalculator),
/// this enum determines how to handle cases where multiple calculators provide values
/// for the same account and date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    /// Use the first calculator that returns a value for a given date.
    ///
    /// Other values for the same date will be logged with a warning and discarded.
    /// This is useful when calculators have a priority order, and you want to use
    /// the highest-priority calculator that provides a value.
    FirstWins,

    /// Sum the amounts from different calculators for each date.
    ///
    /// This is useful when calculators provide complementary values that should be
    /// combined, such as when one calculator provides historical data and another
    /// provides forecasted data.
    Sum,
}

/// Trait for standardizing account state calculation.
///
/// This trait defines the interface for all account state calculators in the system.
/// Implementations of this trait can calculate account balances, forecasts, or other
/// account-related metrics over a specified date range.
///
/// Different implementations may use different strategies for calculating account state,
/// such as using historical transactions, recurring transactions, or forecasting models.
#[async_trait]
pub trait AccountStateCalculator {
    /// Computes the account state for the given accounts within the specified date range.
    ///
    /// This method calculates the state (typically balance) of each account for each day
    /// in the specified date range. The result is returned as a DataFrame with columns
    /// for account_id, date, and balance (at minimum).
    ///
    /// # Arguments
    ///
    /// * `db` - The database connection for retrieving account data
    /// * `accounts` - The accounts to calculate state for
    /// * `start_date` - The first date to include in the calculation
    /// * `end_date` - The last date to include in the calculation
    ///
    /// # Returns
    ///
    /// A DataFrame containing the account state for each account on each date in the range,
    /// or an error if the calculation fails.
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame>;

    /// Returns the merge method to use when combining results from multiple calculators.
    ///
    /// This method defines how this calculator's results should be merged with results
    /// from other calculators when used in a composite calculator like MergeCalculator.
    ///
    /// # Returns
    ///
    /// The merge method to use for this calculator.
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
    use crate::error::ComputeError;

    #[tokio::test]
    async fn test_scenario_balance() {
        let scenario = ScenarioBalance::new();
        let computer = balance::BalanceCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_balance_outside_range() {
        let scenario = ScenarioBalance::new();
        let computer = balance::BalanceCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, false)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_forecast_with_balance_calculator() {
        let scenario = ScenarioForecast::new();
        let computer = balance::BalanceCalculator::new(MergeMethod::FirstWins);

        let result = run_and_assert_scenario(&scenario, &computer, true).await;

        assert!(
            result.is_err(),
            "This shuold fail because no manual state for balance"
        );
    }

    #[tokio::test]
    async fn test_scenario_forecast() {
        let scenario = ScenarioForecast::new();
        let computer = forecast::ForecastCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_forecast_outside_range() {
        let scenario = ScenarioForecast::new();
        // Use initial balance of -$1000 (the balance on Feb 28) when testing outside the range
        let computer = forecast::ForecastCalculator::new_with_initial_balance(
            MergeMethod::FirstWins,
            rust_decimal::Decimal::new(-100000, 2), // -$1000.00
        );

        run_and_assert_scenario(&scenario, &computer, false)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_balance_merge_simple() {
        let scenario = ScenarioBalance::new();
        let computer1 = Box::new(balance::BalanceCalculator::new(MergeMethod::FirstWins));
        let computer2 = Box::new(forecast::ForecastCalculator::new(MergeMethod::FirstWins));

        let computer =
            merge::MergeCalculator::new(vec![computer1, computer2], MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_balance_merge_simple_outside_range() {
        let scenario = ScenarioBalance::new();
        let computer1 = Box::new(balance::BalanceCalculator::new(MergeMethod::FirstWins));
        let computer2 = Box::new(balance::BalanceCalculator::new(MergeMethod::FirstWins));

        let computer =
            merge::MergeCalculator::new(vec![computer1, computer2], MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, false)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_multiple_accounts() {
        let scenario = ScenarioMultipleAccounts::new();
        let computer = balance::BalanceCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_multiple_accounts_outside_range() {
        let scenario = ScenarioMultipleAccounts::new();
        let computer = balance::BalanceCalculator::new(MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, false)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_multiple_accounts_merge_simple() {
        let scenario = ScenarioMultipleAccounts::new();
        let computer1 = Box::new(balance::BalanceCalculator::new(MergeMethod::FirstWins));
        let computer2 = Box::new(forecast::ForecastCalculator::new(MergeMethod::FirstWins));

        let computer =
            merge::MergeCalculator::new(vec![computer1, computer2], MergeMethod::FirstWins);

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_balance_no_instances() {
        let scenario = ScenarioBalanceNoInstances::new();
        // Use March 15, 2023 as "today" so that April 1 transactions are treated as future
        let today = chrono::NaiveDate::from_ymd_opt(2023, 3, 15).unwrap();
        debug!("Using today date: {}", today);
        let computer = balance::BalanceCalculator::new_with_today(MergeMethod::FirstWins, today);

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_balance_no_instances_outside_range() {
        let scenario = ScenarioBalanceNoInstances::new();
        // Use March 15, 2023 as "today" so that April 1 transactions are treated as future
        let today = chrono::NaiveDate::from_ymd_opt(2023, 3, 15).unwrap();
        let computer = balance::BalanceCalculator::new_with_today(MergeMethod::FirstWins, today);

        run_and_assert_scenario(&scenario, &computer, false)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_forecast_no_instances() {
        let scenario = ScenarioForecastNoInstances::new();
        // Use March 15, 2023 as "today" so that March 16 is today + future_offset
        let today = chrono::NaiveDate::from_ymd_opt(2023, 3, 15).unwrap();
        let future_offset = chrono::Duration::days(1);
        debug!(
            "Using today date: {} with future_offset: {} days",
            today,
            future_offset.num_days()
        );
        let computer = forecast::ForecastCalculator::new_with_params(
            MergeMethod::FirstWins,
            rust_decimal::Decimal::new(0, 2), // $0.00
            today,
            future_offset,
        );

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_forecast_no_instances_outside_range() {
        let scenario = ScenarioForecastNoInstances::new();
        // Use initial balance of 0 when testing outside the range
        let computer = forecast::ForecastCalculator::new_with_initial_balance(
            MergeMethod::FirstWins,
            rust_decimal::Decimal::new(-2200 * 100, 2), // $0.00
        );

        run_and_assert_scenario(&scenario, &computer, false)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_merge_real_failing_outside_range() {
        let scenario = ScenarioMergeRealFailing::new();
        let computer = balance::BalanceCalculator::new_with_today(
            MergeMethod::FirstWins,
            NaiveDate::from_ymd_opt(2026, 06, 01).unwrap(),
        );

        let result = run_and_assert_scenario(&scenario, &computer, false).await;
        assert!(
            result.is_err(),
            "This should fail because no manual state for balance"
        );

        if let Err(ComputeError::DataFrame(ref s)) = result {
            if s.contains("Balance mismatch") {
                return;
            }
        }

        assert!(false, "This should fail because balance mismatch instead = {:?}", result);
    }

    #[tokio::test]
    async fn test_scenario_merge_real_outside_range() {
        let scenario = ScenarioMergeReal::new();
        // TODO we need a new type of merger which will take a date
        // and from that day will use a different computer
        let computer = balance::BalanceCalculator::new_with_today(
            MergeMethod::FirstWins,
            NaiveDate::from_ymd_opt(2026, 06, 01).unwrap(),
        );

        run_and_assert_scenario(&scenario, &computer, false)
            .await
            .expect("Failed to run scenario");
    }

    #[tokio::test]
    async fn test_scenario_merge_real_failing() {
        let scenario = ScenarioMergeRealFailing::new();
        let computer = balance::BalanceCalculator::new_with_today(
            MergeMethod::FirstWins,
            NaiveDate::from_ymd_opt(2026, 06, 01).unwrap(),
        );

        let result = run_and_assert_scenario(&scenario, &computer, true).await;
        assert!(
            result.is_err(),
            "This should fail because no manual state for balance"
        );

        if let Err(ComputeError::DataFrame(ref s)) = result {
            if s.contains("Balance mismatch") {
                return;
            }
        }

        assert!(false, "This should fail because balance mismatch instead = {:?}", result);
    }

    #[tokio::test]
    async fn test_scenario_merge_real() {
        let scenario = ScenarioMergeReal::new();
        // TODO we need a new type of merger which will take a date
        // and from that day will use a different computer
        let computer = balance::BalanceCalculator::new_with_today(
            MergeMethod::FirstWins,
            NaiveDate::from_ymd_opt(2026, 06, 01).unwrap(),
        );

        run_and_assert_scenario(&scenario, &computer, true)
            .await
            .expect("Failed to run scenario");
    }
}

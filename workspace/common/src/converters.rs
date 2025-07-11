//! Converter functions for bridging compute and common modules
//!
//! This module provides conversion functions that can transform data structures
//! from the compute module (which uses polars) into the transport-friendly
//! wrapper structures in the common module.
//!
//! Note: This module is designed to be used by the compute module or other
//! modules that have access to both compute and common dependencies.

use crate::statistics::{AccountStatistics, AccountStatisticsCollection, TimePeriod};
use crate::timeseries::{AccountStatePoint, AccountStateTimeseries, DateRange};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Converts a polars DataFrame from compute module to AccountStateTimeseries
///
/// Expected DataFrame columns:
/// - "account_id": i32 values
/// - "date": NaiveDate values  
/// - "balance": String values (will be parsed to Decimal)
///
/// # Arguments
/// * `df` - The polars DataFrame from compute module
///
/// # Returns
/// * `Result<AccountStateTimeseries, String>` - The converted timeseries or error message
///
/// # Note
/// This function signature uses generic types to avoid direct polars dependency.
/// The actual implementation should be provided by the compute module or a bridge module.
pub fn dataframe_to_timeseries<T>(df: T) -> Result<AccountStateTimeseries, String>
where
    T: DataFrameConverter,
{
    df.to_timeseries()
}

/// Trait for converting DataFrame-like structures to timeseries
///
/// This trait allows the common module to define the conversion interface
/// without depending on polars directly. The compute module can implement
/// this trait for polars DataFrame.
pub trait DataFrameConverter {
    /// Convert to AccountStateTimeseries
    fn to_timeseries(self) -> Result<AccountStateTimeseries, String>;

    /// Get the date range covered by this data
    fn get_date_range(&self) -> Result<DateRange, String>;

    /// Get unique account IDs in this data
    fn get_account_ids(&self) -> Result<Vec<i32>, String>;
}

/// Converts compute module AccountStats to common module AccountStatistics
///
/// This function converts a single AccountStats from the compute module
/// to the transport-friendly AccountStatistics structure.
pub fn compute_stats_to_common_stats(
    account_id: i32,
    min_state: Option<Decimal>,
    max_state: Option<Decimal>,
    average_expense: Option<Decimal>,
    average_income: Option<Decimal>,
    upcoming_expenses: Option<Decimal>,
    end_of_period_state: Option<Decimal>,
) -> AccountStatistics {
    AccountStatistics {
        account_id,
        min_state,
        max_state,
        average_expense,
        average_income,
        upcoming_expenses,
        end_of_period_state,
    }
}

/// Converts a vector of compute AccountStats to AccountStatisticsCollection
///
/// This function takes individual statistics components and creates a collection
/// with the specified time period.
pub fn compute_stats_vec_to_collection(
    stats_data: Vec<(
        i32,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
    )>,
    period: TimePeriod,
) -> AccountStatisticsCollection {
    let statistics: Vec<AccountStatistics> = stats_data
        .into_iter()
        .map(
            |(
                account_id,
                min_state,
                max_state,
                average_expense,
                average_income,
                upcoming_expenses,
                end_of_period_state,
            )| {
                compute_stats_to_common_stats(
                    account_id,
                    min_state,
                    max_state,
                    average_expense,
                    average_income,
                    upcoming_expenses,
                    end_of_period_state,
                )
            },
        )
        .collect();

    AccountStatisticsCollection::new(period, statistics)
}

/// Helper function to create AccountStatePoint from raw data
///
/// This is useful when converting from various data sources to the common format.
pub fn create_account_state_point(
    account_id: i32,
    date: NaiveDate,
    balance_str: &str,
) -> Result<AccountStatePoint, String> {
    let balance = Decimal::from_str(balance_str)
        .map_err(|e| format!("Failed to parse balance '{}': {}", balance_str, e))?;

    Ok(AccountStatePoint::new(account_id, date, balance))
}

/// Helper function to create multiple AccountStatePoints from raw data
///
/// This is useful for batch conversion from various data sources.
pub fn create_account_state_points(
    data: Vec<(i32, NaiveDate, String)>,
) -> Result<Vec<AccountStatePoint>, String> {
    data.into_iter()
        .map(|(account_id, date, balance_str)| {
            create_account_state_point(account_id, date, &balance_str)
        })
        .collect()
}

/// Creates a TimePeriod from year
pub fn create_year_period(year: i32) -> TimePeriod {
    TimePeriod::year(year)
}

/// Creates a TimePeriod from year and month
pub fn create_month_period(year: i32, month: u32) -> TimePeriod {
    TimePeriod::month(year, month)
}

/// Creates a TimePeriod from date range
pub fn create_date_range_period(start: NaiveDate, end: NaiveDate) -> TimePeriod {
    TimePeriod::date_range(start, end)
}

/// Utility function to extract data from timeseries for external use
///
/// This function converts the timeseries back to raw data that can be
/// easily consumed by other systems or serialized.
pub fn timeseries_to_raw_data(
    timeseries: &AccountStateTimeseries,
) -> Vec<(i32, NaiveDate, String)> {
    timeseries
        .data_points
        .iter()
        .map(|point| (point.account_id, point.date, point.balance.to_string()))
        .collect()
}

/// Utility function to extract statistics data for external use
///
/// This function converts the statistics collection back to raw data.
pub fn statistics_to_raw_data(
    collection: &AccountStatisticsCollection,
) -> Vec<(
    i32,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    collection
        .statistics
        .iter()
        .map(|stats| {
            (
                stats.account_id,
                stats.min_state.map(|d| d.to_string()),
                stats.max_state.map(|d| d.to_string()),
                stats.average_expense.map(|d| d.to_string()),
                stats.average_income.map(|d| d.to_string()),
                stats.upcoming_expenses.map(|d| d.to_string()),
                stats.end_of_period_state.map(|d| d.to_string()),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_stats_to_common_stats() {
        let stats = compute_stats_to_common_stats(
            1,
            Some(Decimal::new(100, 2)),
            Some(Decimal::new(500, 2)),
            Some(Decimal::new(200, 2)),
            Some(Decimal::new(300, 2)),
            Some(Decimal::new(150, 2)),
            Some(Decimal::new(400, 2)),
        );

        assert_eq!(stats.account_id, 1);
        assert_eq!(stats.min_state, Some(Decimal::new(100, 2)));
        assert_eq!(stats.max_state, Some(Decimal::new(500, 2)));
    }

    #[test]
    fn test_create_account_state_point() {
        let point =
            create_account_state_point(1, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), "100.50")
                .expect("Should create point successfully");

        assert_eq!(point.account_id, 1);
        assert_eq!(point.date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(point.balance, Decimal::new(10050, 2));
    }

    #[test]
    fn test_create_account_state_point_invalid_balance() {
        let result =
            create_account_state_point(1, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), "invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse balance"));
    }

    #[test]
    fn test_create_account_state_points() {
        let data = vec![
            (
                1,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                "100.00".to_string(),
            ),
            (
                2,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                "200.00".to_string(),
            ),
        ];

        let points = create_account_state_points(data).expect("Should create points successfully");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].account_id, 1);
        assert_eq!(points[1].account_id, 2);
    }

    #[test]
    fn test_compute_stats_vec_to_collection() {
        let stats_data = vec![
            (
                1,
                Some(Decimal::new(100, 2)),
                Some(Decimal::new(500, 2)),
                None,
                None,
                None,
                None,
            ),
            (
                2,
                Some(Decimal::new(200, 2)),
                Some(Decimal::new(600, 2)),
                None,
                None,
                None,
                None,
            ),
        ];

        let collection = compute_stats_vec_to_collection(stats_data, TimePeriod::year(2024));
        assert_eq!(collection.account_count(), 2);
        assert_eq!(collection.period, TimePeriod::year(2024));
    }

    #[test]
    fn test_timeseries_to_raw_data() {
        let points = vec![
            AccountStatePoint::new(
                1,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                Decimal::new(1000, 2),
            ),
            AccountStatePoint::new(
                2,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                Decimal::new(2000, 2),
            ),
        ];
        let timeseries = AccountStateTimeseries::new(points);

        let raw_data = timeseries_to_raw_data(&timeseries);
        assert_eq!(raw_data.len(), 2);
        assert_eq!(raw_data[0].0, 1);
        assert_eq!(raw_data[0].2, "10.00");
    }

    #[test]
    fn test_period_creation_functions() {
        assert_eq!(create_year_period(2024), TimePeriod::year(2024));
        assert_eq!(create_month_period(2024, 6), TimePeriod::month(2024, 6));

        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        assert_eq!(
            create_date_range_period(start, end),
            TimePeriod::date_range(start, end)
        );
    }
}

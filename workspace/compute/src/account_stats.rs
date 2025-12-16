//! Account statistics module for computing yearly and monthly account statistics.
//!
//! This module provides functions to calculate various statistics about accounts
//! such as minimum and maximum states, average expenses and income, upcoming expenses,
//! and end-of-period states.

use chrono::{NaiveDate, Datelike};
use polars::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::collections::HashMap;
use sea_orm::DatabaseConnection;
use tracing::instrument;

use model::entities::account;

use crate::account::AccountStateCalculator;
use crate::error::Result;

/// Statistics for a specific time period (year or month)
#[derive(Debug, Clone)]
pub struct AccountStats {
    pub account_id: i32,
    pub min_state: Option<Decimal>,
    pub max_state: Option<Decimal>,
    pub average_expense: Option<Decimal>,
    pub average_income: Option<Decimal>,
    pub upcoming_expenses: Option<Decimal>,
    pub end_of_period_state: Option<Decimal>,
}

/// Computes minimum account state for the specified year
#[instrument(skip(calculator, db, accounts))]
pub async fn min_state_in_year(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_min_state_from_dataframe(df)
}

/// Computes minimum account state for the specified month
#[instrument(skip(calculator, db, accounts))]
pub async fn min_state_in_month(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
    month: u32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end_date = get_last_day_of_month(year, month);

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_min_state_from_dataframe(df)
}

/// Computes maximum account state for the specified year
#[instrument(skip(calculator, db, accounts))]
pub async fn max_state_in_year(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_max_state_from_dataframe(df)
}

/// Computes maximum account state for the specified month
#[instrument(skip(calculator, db, accounts))]
pub async fn max_state_in_month(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
    month: u32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end_date = get_last_day_of_month(year, month);

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_max_state_from_dataframe(df)
}

/// Computes average expense for the specified year
#[instrument(skip(calculator, db, accounts))]
pub async fn average_expense_in_year(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_basic_stats_from_dataframe(df, StatType::AverageExpense)
}

/// Computes average expense for the specified month
#[instrument(skip(calculator, db, accounts))]
pub async fn average_expense_in_month(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
    month: u32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end_date = get_last_day_of_month(year, month);

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_basic_stats_from_dataframe(df, StatType::AverageExpense)
}

/// Computes average income for the specified year
#[instrument(skip(calculator, db, accounts))]
pub async fn average_income_in_year(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_basic_stats_from_dataframe(df, StatType::AverageIncome)
}

/// Computes average income for the specified month
#[instrument(skip(calculator, db, accounts))]
pub async fn average_income_in_month(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
    month: u32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end_date = get_last_day_of_month(year, month);

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_basic_stats_from_dataframe(df, StatType::AverageIncome)
}

/// Computes upcoming expenses until the end of the specified year
#[instrument(skip(calculator, db, accounts))]
pub async fn upcoming_expenses_until_year_end(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
    from_date: NaiveDate,
) -> Result<Vec<AccountStats>> {
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    let df = calculator
        .compute_account_state(db, accounts, from_date, end_date)
        .await?;
    compute_basic_stats_from_dataframe(df, StatType::UpcomingExpenses)
}

/// Computes upcoming expenses until the end of the specified month
#[instrument(skip(calculator, db, accounts))]
pub async fn upcoming_expenses_until_month_end(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
    month: u32,
    from_date: NaiveDate,
) -> Result<Vec<AccountStats>> {
    let end_date = get_last_day_of_month(year, month);

    let df = calculator
        .compute_account_state(db, accounts, from_date, end_date)
        .await?;
    compute_basic_stats_from_dataframe(df, StatType::UpcomingExpenses)
}

/// Computes end of year state for the specified year
#[instrument(skip(calculator, db, accounts))]
pub async fn end_of_year_state(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_end_of_period_state_from_dataframe(df, end_date)
}

/// Computes end of month state for the specified month
#[instrument(skip(calculator, db, accounts))]
pub async fn end_of_month_state(
    calculator: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
    month: u32,
) -> Result<Vec<AccountStats>> {
    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end_date = get_last_day_of_month(year, month);

    let df = calculator
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;
    compute_end_of_period_state_from_dataframe(df, end_date)
}

// Helper types and functions

#[derive(Debug, Clone, Copy)]
enum StatType {
    AverageExpense,
    AverageIncome,
    UpcomingExpenses,
}

fn compute_min_state_from_dataframe(df: DataFrame) -> Result<Vec<AccountStats>> {
    // Map each account_id to its minimum Decimal balance found in the frame
    let account_col = df
        .column("account_id")
        .or_else(|_| df.column("account"))
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing account column: {e}")))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    let mut mins: HashMap<i32, Decimal> = HashMap::new();

    for i in 0..df.height() {
        let account_id = account_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting account at row {i}: {e}")))?
            .try_extract::<i32>()
            .map_err(|e| crate::error::ComputeError::Series(format!("Error extracting account as i32 at row {i}: {e}")))?;

        let bal_any = balance_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting balance at row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| crate::error::ComputeError::Decimal(format!("Invalid balance '{bal_str}' at row {i}: {e}")))?;

        mins
            .entry(account_id)
            .and_modify(|m| {
                if bal < *m {
                    *m = bal;
                }
            })
            .or_insert(bal);
    }

    let mut stats = Vec::with_capacity(mins.len());
    for (account_id, min_state) in mins.into_iter() {
        stats.push(AccountStats {
            account_id,
            min_state: Some(min_state),
            max_state: None,
            average_expense: None,
            average_income: None,
            upcoming_expenses: None,
            end_of_period_state: None,
        });
    }
    Ok(stats)
}

fn compute_max_state_from_dataframe(df: DataFrame) -> Result<Vec<AccountStats>> {
    let account_col = df
        .column("account_id")
        .or_else(|_| df.column("account"))
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing account column: {e}")))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    let mut maxs: HashMap<i32, Decimal> = HashMap::new();

    for i in 0..df.height() {
        let account_id = account_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting account at row {i}: {e}")))?
            .try_extract::<i32>()
            .map_err(|e| crate::error::ComputeError::Series(format!("Error extracting account as i32 at row {i}: {e}")))?;

        let bal_any = balance_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting balance at row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| crate::error::ComputeError::Decimal(format!("Invalid balance '{bal_str}' at row {i}: {e}")))?;

        maxs
            .entry(account_id)
            .and_modify(|m| {
                if bal > *m {
                    *m = bal;
                }
            })
            .or_insert(bal);
    }

    let mut stats = Vec::with_capacity(maxs.len());
    for (account_id, max_state) in maxs.into_iter() {
        stats.push(AccountStats {
            account_id,
            min_state: None,
            max_state: Some(max_state),
            average_expense: None,
            average_income: None,
            upcoming_expenses: None,
            end_of_period_state: None,
        });
    }
    Ok(stats)
}

fn compute_basic_stats_from_dataframe(
    df: DataFrame,
    stat_type: StatType,
) -> Result<Vec<AccountStats>> {
    // Extract required columns
    let account_col = df
        .column("account_id")
        .or_else(|_| df.column("account"))
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing account column: {e}")))?;
    let date_col = df
        .column("date")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing date column: {e}")))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    // Build per-account time series of (date, balance)
    let mut series_map: HashMap<i32, Vec<(i64, Decimal)>> = HashMap::new();

    for i in 0..df.height() {
        let account_id = account_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting account at row {i}: {e}")))?
            .try_extract::<i32>()
            .map_err(|e| crate::error::ComputeError::Series(format!("Error extracting account as i32 at row {i}: {e}")))?;

        let date = date_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting date at row {i}: {e}")))?
            .try_extract::<i64>()
            .map_err(|e| crate::error::ComputeError::Series(format!("Error extracting date as i64 at row {i}: {e}")))?;

        let bal_any = balance_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting balance at row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| crate::error::ComputeError::Decimal(format!("Invalid balance '{bal_str}' at row {i}: {e}")))?;

        series_map.entry(account_id).or_default().push((date, bal));
    }

    // Compute stats per account
    let mut out: Vec<AccountStats> = Vec::with_capacity(series_map.len());
    for (account_id, mut points) in series_map {
        points.sort_by_key(|(d, _)| *d);

        let mut sum_pos = Decimal::ZERO;
        let mut cnt_pos: u32 = 0;
        let mut sum_neg_abs = Decimal::ZERO;
        let mut cnt_neg: u32 = 0;

        for w in points.windows(2) {
            let (_, prev) = w[0];
            let (_, curr) = w[1];
            let delta = curr - prev;
            if delta > Decimal::ZERO {
                sum_pos += delta;
                cnt_pos += 1;
            } else if delta < Decimal::ZERO {
                sum_neg_abs += -delta;
                cnt_neg += 1;
            }
        }

        let mut stat = AccountStats {
            account_id,
            min_state: None,
            max_state: None,
            average_expense: None,
            average_income: None,
            upcoming_expenses: None,
            end_of_period_state: None,
        };

        match stat_type {
            StatType::AverageExpense => {
                let avg = if cnt_neg > 0 {
                    sum_neg_abs / Decimal::from(cnt_neg as i64)
                } else {
                    Decimal::ZERO
                };
                stat.average_expense = Some(avg);
            }
            StatType::AverageIncome => {
                let avg = if cnt_pos > 0 {
                    sum_pos / Decimal::from(cnt_pos as i64)
                } else {
                    Decimal::ZERO
                };
                stat.average_income = Some(avg);
            }
            StatType::UpcomingExpenses => {
                // We interpret the provided df as the future window (from_date..=end_date)
                // Sum all negative deltas (as positive amounts) as upcoming expenses.
                stat.upcoming_expenses = Some(sum_neg_abs);
            }
        }

        out.push(stat);
    }

    Ok(out)
}

fn compute_end_of_period_state_from_dataframe(
    df: DataFrame,
    end_date: NaiveDate,
) -> Result<Vec<AccountStats>> {
    let account_col = df
        .column("account_id")
        .or_else(|_| df.column("account"))
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing account column: {e}")))?;
    let date_col = df
        .column("date")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing date column: {e}")))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    let end_num: i64 = end_date.num_days_from_ce() as i64;

    // Build per-account points filtered by end_date
    let mut latest_map: HashMap<i32, (i64, Decimal)> = HashMap::new();

    for i in 0..df.height() {
        let account_id = account_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting account at row {i}: {e}")))?
            .try_extract::<i32>()
            .map_err(|e| crate::error::ComputeError::Series(format!("Error extracting account as i32 at row {i}: {e}")))?;

        let date = date_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting date at row {i}: {e}")))?
            .try_extract::<i64>()
            .map_err(|e| crate::error::ComputeError::Series(format!("Error extracting date as i64 at row {i}: {e}")))?;

        if date > end_num {
            continue;
        }

        let bal_any = balance_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting balance at row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str)
            .map_err(|e| crate::error::ComputeError::Decimal(format!("Invalid balance '{bal_str}' at row {i}: {e}")))?;

        latest_map
            .entry(account_id)
            .and_modify(|(d, b)| {
                if date >= *d {
                    *d = date;
                    *b = bal;
                }
            })
            .or_insert((date, bal));
    }

    let mut out = Vec::with_capacity(latest_map.len());
    for (account_id, (_, bal)) in latest_map {
        out.push(AccountStats {
            account_id,
            min_state: None,
            max_state: None,
            average_expense: None,
            average_income: None,
            upcoming_expenses: None,
            end_of_period_state: Some(bal),
        });
    }

    Ok(out)
}

/// Extract unique account IDs from a Polars DataFrame.
///
/// This helper expects a column with account identifiers. It prefers the
/// canonical "account_id" column and falls back to "account" to be
/// compatible with some intermediate DataFrames.
fn get_unique_account_ids(df: &DataFrame) -> Result<Vec<i32>> {
    // Prefer the canonical "account_id" column; fall back to "account".
    let series = match df.column("account_id") {
        Ok(s) => s.clone(),
        Err(_) => df.column("account")?.clone(),
    };

    // Obtain unique values and collect all non-null i32 IDs.
    let uniques = series.unique()?;
    let ids = uniques.i32()?;
    Ok(ids.into_no_null_iter().collect())
}

fn get_last_day_of_month(year: i32, month: u32) -> NaiveDate {
    // Get the first day of the next month, then subtract one day
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };

    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}

/// Calculates the date when a goal target amount is reached based on forecast data.
///
/// This function analyzes the forecast DataFrame and finds the first date where
/// the balance reaches or exceeds the target amount.
///
/// # Arguments
/// * `forecast_df` - DataFrame with columns: date (i64), balance (Decimal as string)
/// * `target_amount` - The target amount to reach
///
/// # Returns
/// * `Ok(Some(date))` - The first date when the target is reached
/// * `Ok(None)` - If the target is never reached within the forecast range
/// * `Err(_)` - If there's an error processing the DataFrame
pub fn calculate_goal_reached_date(
    forecast_df: &DataFrame,
    target_amount: Decimal,
) -> Result<Option<NaiveDate>> {
    eprintln!("DEBUG: calculate_goal_reached_date called");
    eprintln!("DEBUG: DataFrame shape: {} rows x {} cols", forecast_df.height(), forecast_df.width());
    eprintln!("DEBUG: DataFrame columns: {:?}", forecast_df.get_column_names());
    eprintln!("DEBUG: Target amount: {}", target_amount);

    // Sort by date to ensure chronological order
    let sorted_df = forecast_df
        .sort(["date"], Default::default())
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Failed to sort by date: {e}")))?;

    let date_col = sorted_df
        .column("date")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing date column: {e}")))?;
    let balance_col = sorted_df
        .column("balance")
        .map_err(|e| crate::error::ComputeError::DataFrame(format!("Missing balance column: {e}")))?;

    let today = chrono::Utc::now().date_naive();
    eprintln!("DEBUG: Today's date: {}", today);

    // Iterate through rows to find first date where balance >= target_amount
    for i in 0..sorted_df.height() {
        let date_any = date_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting date at row {i}: {e}")))?;

        // Parse date - could be Date (i64), Int64, or String
        let date = match date_any {
            AnyValue::Date(days) => {
                // Polars Date type stores dates as days since Unix epoch (1970-01-01)
                let unix_epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                unix_epoch + chrono::Duration::days(days as i64)
            }
            AnyValue::Int64(days) => {
                // Also handle as days since Unix epoch
                let unix_epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                unix_epoch + chrono::Duration::days(days)
            }
            AnyValue::String(s) => {
                NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map_err(|e| crate::error::ComputeError::Date(format!("Invalid date string '{s}' at row {i}: {e}")))?
            }
            AnyValue::StringOwned(s) => {
                NaiveDate::parse_from_str(s.as_str(), "%Y-%m-%d")
                    .map_err(|e| crate::error::ComputeError::Date(format!("Invalid date string '{s}' at row {i}: {e}")))?
            }
            other => {
                return Err(crate::error::ComputeError::Series(format!("Unexpected date type at row {i}: {other:?}")));
            }
        };

        let bal_any = balance_col
            .get(i)
            .map_err(|e| crate::error::ComputeError::Series(format!("Error getting balance at row {i}: {e}")))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let balance = Decimal::from_str(&bal_str)
            .map_err(|e| crate::error::ComputeError::Decimal(format!("Invalid balance '{bal_str}' at row {i}: {e}")))?;

        // Skip dates in the past
        if date < today {
            continue;
        }

        // Debug first few and around target
        if i < 5 || (balance >= target_amount - Decimal::from(5000) && balance <= target_amount + Decimal::from(5000)) {
            eprintln!("DEBUG row {}: date={}, balance={}, target={}", i, date, balance, target_amount);
        }

        // Check if we've reached the goal
        if balance >= target_amount {
            eprintln!("DEBUG: Goal reached at row {} on date {} with balance {}", i, date, balance);
            return Ok(Some(date));
        }
    }

    // Goal not reached within forecast range
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;

    #[test]
    fn test_get_last_day_of_month() {
        assert_eq!(
            get_last_day_of_month(2023, 1),
            NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()
        );
        assert_eq!(
            get_last_day_of_month(2023, 2),
            NaiveDate::from_ymd_opt(2023, 2, 28).unwrap()
        );
        assert_eq!(
            get_last_day_of_month(2024, 2),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        ); // Leap year
        assert_eq!(
            get_last_day_of_month(2023, 12),
            NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()
        );
    }

    #[test]
    fn test_get_unique_account_ids_prefers_account_id() {
        let df = df! {
            "account_id" => &[1i32, 2, 1, 3],
            "balance" => &[10i32, 20, 15, 30],
        }
        .unwrap();
        let mut ids = get_unique_account_ids(&df).unwrap();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_get_unique_account_ids_fallback_to_account() {
        let df = df! {
            "account" => &[7i32, 7, 8],
            "balance" => &[1i32, 2, 3],
        }
        .unwrap();
        let mut ids = get_unique_account_ids(&df).unwrap();
        ids.sort();
        assert_eq!(ids, vec![7, 8]);
    }
}

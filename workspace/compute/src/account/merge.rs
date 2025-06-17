use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use sea_orm::DatabaseConnection;

use crate::error::{ComputeError, Result};
use super::{balance, forecast};

/// Merges balance and forecast data for accounts within a specified date range.
///
/// This function computes the balance up to today and the forecast for future dates,
/// then merges them together into a single DataFrame.
pub async fn compute_merged(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<DataFrame> {
    // Get the current date
    let today = chrono::Local::now().date_naive();

    // Compute balance up to today
    let balance_end_date = if today < end_date { today } else { end_date };
    let balance_df = if start_date <= balance_end_date {
        balance::compute_balance(db, accounts, start_date, balance_end_date).await
            .map_err(|e| ComputeError::BalanceComputation(format!("Failed to compute balance: {}", e)))?
    } else {
        // Create an empty DataFrame if the start date is after today
        let empty_df = DataFrame::new(vec![
            Series::new("account_id".into(), Vec::<i32>::new()).into(),
            Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
            Series::new("balance".into(), Vec::<String>::new()).into(),
        ])?;
        empty_df
    };

    // Compute forecast from tomorrow onwards
    let forecast_start_date = balance_end_date.succ_opt().unwrap_or(balance_end_date);
    let forecast_df = if forecast_start_date <= end_date {
        forecast::compute_forecast(db, accounts, forecast_start_date, end_date).await
            .map_err(|e| ComputeError::ForecastComputation(format!("Failed to compute forecast: {}", e)))?
    } else {
        // Create an empty DataFrame if the forecast start date is after the end date
        let empty_df = DataFrame::new(vec![
            Series::new("account_id".into(), Vec::<i32>::new()).into(),
            Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
            Series::new("balance".into(), Vec::<String>::new()).into(),
        ])?;
        empty_df
    };

    // Merge the two DataFrames
    let merged_df = if balance_df.height() > 0 && forecast_df.height() > 0 {
        // Both DataFrames have data, so concatenate them
        let merged = balance_df.vstack(&forecast_df)?;
        merged
    } else if balance_df.height() > 0 {
        // Only balance has data
        balance_df
    } else if forecast_df.height() > 0 {
        // Only forecast has data
        forecast_df
    } else {
        // Neither has data, return an empty DataFrame
        let empty_df = DataFrame::new(vec![
            Series::new("account_id".into(), Vec::<i32>::new()).into(),
            Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
            Series::new("balance".into(), Vec::<String>::new()).into(),
        ])?;
        empty_df
    };

    Ok(merged_df)
}

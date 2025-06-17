use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use sea_orm::DatabaseConnection;
use tracing::{debug, info, instrument, trace};

use crate::error::{ComputeError, Result};
use super::{balance, forecast};

/// Merges balance and forecast data for accounts within a specified date range.
///
/// This function computes the balance up to today and the forecast for future dates,
/// then merges them together into a single DataFrame.
#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date))]
pub async fn compute_merged(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<DataFrame> {
    info!("Computing merged balance and forecast for {} accounts from {} to {}", accounts.len(), start_date, end_date);

    // Get the current date
    let today = chrono::Local::now().date_naive();
    debug!("Current date is {}", today);

    // Compute balance up to today
    let balance_end_date = if today < end_date { today } else { end_date };
    debug!("Computing balance up to {}", balance_end_date);

    let balance_df = if start_date <= balance_end_date {
        trace!("Calling compute_balance from {} to {}", start_date, balance_end_date);
        balance::compute_balance(db, accounts, start_date, balance_end_date).await
            .map_err(|e| {
                debug!("Balance computation failed: {}", e);
                ComputeError::BalanceComputation(format!("Failed to compute balance: {}", e))
            })?
    } else {
        // Create an empty DataFrame if the start date is after today
        debug!("Start date {} is after balance end date {}, creating empty balance DataFrame", start_date, balance_end_date);
        let empty_df = DataFrame::new(vec![
            Series::new("account_id".into(), Vec::<i32>::new()).into(),
            Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
            Series::new("balance".into(), Vec::<String>::new()).into(),
        ])?;
        empty_df
    };
    debug!("Balance DataFrame has {} rows", balance_df.height());

    // Compute forecast from tomorrow onwards
    let forecast_start_date = balance_end_date.succ_opt().unwrap_or(balance_end_date);
    debug!("Computing forecast from {} to {}", forecast_start_date, end_date);

    let forecast_df = if forecast_start_date <= end_date {
        trace!("Calling compute_forecast from {} to {}", forecast_start_date, end_date);
        forecast::compute_forecast(db, accounts, forecast_start_date, end_date).await
            .map_err(|e| {
                debug!("Forecast computation failed: {}", e);
                ComputeError::ForecastComputation(format!("Failed to compute forecast: {}", e))
            })?
    } else {
        // Create an empty DataFrame if the forecast start date is after the end date
        debug!("Forecast start date {} is after end date {}, creating empty forecast DataFrame", forecast_start_date, end_date);
        let empty_df = DataFrame::new(vec![
            Series::new("account_id".into(), Vec::<i32>::new()).into(),
            Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
            Series::new("balance".into(), Vec::<String>::new()).into(),
        ])?;
        empty_df
    };
    debug!("Forecast DataFrame has {} rows", forecast_df.height());

    // Merge the two DataFrames
    trace!("Merging balance and forecast DataFrames");
    let merged_df = if balance_df.height() > 0 && forecast_df.height() > 0 {
        // Both DataFrames have data, so concatenate them
        debug!("Both balance and forecast have data, concatenating");
        let merged = balance_df.vstack(&forecast_df)?;
        merged
    } else if balance_df.height() > 0 {
        // Only balance has data
        debug!("Only balance has data, using balance DataFrame");
        balance_df
    } else if forecast_df.height() > 0 {
        // Only forecast has data
        debug!("Only forecast has data, using forecast DataFrame");
        forecast_df
    } else {
        // Neither has data, return an empty DataFrame
        debug!("Neither balance nor forecast has data, creating empty DataFrame");
        let empty_df = DataFrame::new(vec![
            Series::new("account_id".into(), Vec::<i32>::new()).into(),
            Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
            Series::new("balance".into(), Vec::<String>::new()).into(),
        ])?;
        empty_df
    };

    info!("Merged computation completed successfully with {} data points", merged_df.height());
    Ok(merged_df)
}

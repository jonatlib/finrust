use async_trait::async_trait;
use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use tracing::{debug, info, instrument, trace, warn};

use super::{AccountStateCalculator, MergeMethod, balance, forecast};
use crate::error::{ComputeError, Result};

/// A calculator that merges the results of multiple account state calculators.
pub struct MergeCalculator {
    /// The calculators to use for computing account states.
    calculators: Vec<Box<dyn AccountStateCalculator + Send + Sync>>,
    /// The merge method to use when combining results from multiple calculators.
    merge_method: MergeMethod,
}

impl MergeCalculator {
    /// Creates a new merge calculator with the specified calculators and merge method.
    pub fn new(
        calculators: Vec<Box<dyn AccountStateCalculator + Send + Sync>>,
        merge_method: MergeMethod,
    ) -> Self {
        Self {
            calculators,
            merge_method,
        }
    }

    /// Creates a new merge calculator with the default balance and forecast calculators.
    pub fn default() -> Self {
        let balance_calculator = Box::new(balance::BalanceCalculator::default());
        let forecast_calculator = Box::new(forecast::ForecastCalculator::default());

        Self {
            calculators: vec![balance_calculator, forecast_calculator],
            merge_method: MergeMethod::FirstWins,
        }
    }

    /// Merges DataFrames from multiple calculators according to the merge method.
    async fn merge_dataframes(&self, dfs: Vec<DataFrame>) -> Result<DataFrame> {
        if dfs.is_empty() {
            debug!("No DataFrames to merge, creating empty DataFrame");
            let empty_df = DataFrame::new(vec![
                Series::new("account_id".into(), Vec::<i32>::new()).into(),
                Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
                Series::new("balance".into(), Vec::<String>::new()).into(),
            ])?;
            return Ok(empty_df);
        }

        if dfs.len() == 1 {
            debug!("Only one DataFrame to merge, returning it directly");
            return Ok(dfs[0].clone());
        }

        debug!(
            "Merging {} DataFrames using method {:?}",
            dfs.len(),
            self.merge_method
        );

        // Create maps to store data by (account_id, date) pair
        let mut sum_map: HashMap<(i32, NaiveDate), rust_decimal::Decimal> = HashMap::new();
        let mut first_wins_map: HashMap<(i32, NaiveDate), String> = HashMap::new();

        // Process each calculator and its DataFrame
        for (i, (calculator, df)) in self.calculators.iter().zip(dfs.iter()).enumerate() {
            debug!(
                "Processing DataFrame {} with merge method {:?}",
                i,
                calculator.merge_method()
            );

            // Get the columns we need
            let account_id_col = df.column("account_id")?;
            let date_col = df.column("date")?;
            let balance_col = df.column("balance")?;

            // Process each row in the DataFrame
            for row_idx in 0..df.height() {
                // Get account_id
                let account_id = match account_id_col.get(row_idx) {
                    Ok(value) => match value.try_extract::<i32>() {
                        Ok(id) => id,
                        Err(_) => {
                            warn!("Invalid account_id type in row {}", row_idx);
                            continue;
                        }
                    },
                    Err(e) => {
                        warn!("Error getting account_id in row {}: {}", row_idx, e);
                        continue;
                    }
                };

                // Get date
                let date = match date_col.get(row_idx) {
                    Ok(value) => {
                        match value.try_extract::<i32>() {
                            Ok(d) => {
                                // Convert i32 days since epoch to NaiveDate
                                NaiveDate::from_ymd_opt(1970, 1, 1)
                                    .unwrap()
                                    .checked_add_days(chrono::Days::new(d as u64))
                                    .unwrap_or_default()
                            }
                            Err(_) => {
                                warn!("Invalid date type in row {}", row_idx);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error getting date in row {}: {}", row_idx, e);
                        continue;
                    }
                };

                // Get balance
                let balance_str = match balance_col.get(row_idx) {
                    Ok(value) => {
                        // Extract the string value directly
                        value.str_value().to_string()
                    }
                    Err(e) => {
                        warn!("Error getting balance in row {}: {}", row_idx, e);
                        continue;
                    }
                };

                let key = (account_id, date);

                match calculator.merge_method() {
                    MergeMethod::Sum => {
                        // Parse balance and add to sum
                        if let Ok(balance) = balance_str.parse::<rust_decimal::Decimal>() {
                            let entry = sum_map.entry(key).or_insert(rust_decimal::Decimal::ZERO);
                            *entry += balance;
                        } else {
                            warn!("Failed to parse balance '{}' as Decimal", balance_str);
                        }
                    }
                    MergeMethod::FirstWins => {
                        // Insert only if this (account_id, date) pair doesn't exist yet
                        first_wins_map.entry(key).or_insert(balance_str);
                    }
                }
            }
        }

        // Prepare result DataFrame
        let mut account_ids = Vec::new();
        let mut dates = Vec::new();
        let mut balances = Vec::new();

        // Add Sum results
        for ((account_id, date), balance) in sum_map {
            account_ids.push(account_id);
            dates.push(date);
            balances.push(balance.to_string());
        }

        // Add FirstWins results
        for ((account_id, date), balance) in first_wins_map {
            // Skip if this (account_id, date) pair was already added from Sum
            if !account_ids
                .iter()
                .zip(dates.iter())
                .any(|(&id, &d)| id == account_id && d == date)
            {
                account_ids.push(account_id);
                dates.push(date);
                balances.push(balance);
            }
        }

        // Create result DataFrame
        let result_df = DataFrame::new(vec![
            Series::new("account_id".into(), account_ids).into(),
            Series::new("date".into(), dates).into(),
            Series::new("balance".into(), balances).into(),
        ])?;

        debug!("Merged DataFrame has {} rows", result_df.height());
        Ok(result_df)
    }
}

#[async_trait]
impl AccountStateCalculator for MergeCalculator {
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
        today: Option<NaiveDate>,
    ) -> Result<DataFrame> {
        debug!(
            "Computing merged account state for {} accounts from {} to {}",
            accounts.len(),
            start_date,
            end_date
        );

        // Compute account state using each calculator
        let mut dataframes = Vec::new();

        for (i, calculator) in self.calculators.iter().enumerate() {
            debug!("Computing account state using calculator {}", i);
            let df = calculator
                .compute_account_state(db, accounts, start_date, end_date, today)
                .await?;
            debug!(
                "Calculator {} returned DataFrame with {} rows",
                i,
                df.height()
            );
            dataframes.push(df);
        }

        // Merge the DataFrames according to the merge method
        self.merge_dataframes(dataframes).await
    }

    fn merge_method(&self) -> MergeMethod {
        self.merge_method
    }
}

/// Merges balance and forecast data for accounts within a specified date range.
///
/// This function computes the balance up to today and the forecast for future dates,
/// then merges them together into a single DataFrame.
/// 
/// The `today` parameter is used to determine what is "past" or "future" for recurring transactions.
/// For the balance model, recurring transactions without a linked one-off transaction are ignored.
/// For the forecast model, past recurring transactions without a linked one-off transaction
/// are moved forward in time, as they are considered "not paid yet".
#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date, today = ?today))]
async fn compute_merged(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: Option<NaiveDate>,
) -> Result<DataFrame> {
    info!(
        "Computing merged balance and forecast for {} accounts from {} to {}",
        accounts.len(),
        start_date,
        end_date
    );

    // Create a merge calculator with default calculators
    let merge_calculator = MergeCalculator::default();

    // Use the merge calculator to compute the account state
    let result = merge_calculator
        .compute_account_state(db, accounts, start_date, end_date, today)
        .await?;

    info!(
        "Merged computation completed successfully with {} data points",
        result.height()
    );
    Ok(result)
}

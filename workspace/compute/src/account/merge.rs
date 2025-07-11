use async_trait::async_trait;
use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

use super::{AccountStateCalculator, MergeMethod, balance, forecast};
use crate::error::Result;

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

        // Create a map to store all data by (account_id, date) pair
        // We'll collect all balances for each key and then combine them according to the merge method
        let mut data_map: HashMap<(i32, NaiveDate), Vec<rust_decimal::Decimal>> = HashMap::new();

        // Process each DataFrame
        for (i, df) in dfs.iter().enumerate() {
            debug!("Processing DataFrame {}", i);

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

                // Parse balance
                if let Ok(balance) = balance_str.parse::<rust_decimal::Decimal>() {
                    let key = (account_id, date);
                    data_map.entry(key).or_default().push(balance);
                } else {
                    warn!("Failed to parse balance '{}' as Decimal", balance_str);
                }
            }
        }

        // Prepare result DataFrame
        let mut account_ids = Vec::new();
        let mut dates = Vec::new();
        let mut balances = Vec::new();

        // Combine balances according to the merge method
        for ((account_id, date), balances_vec) in data_map {
            if balances_vec.is_empty() {
                continue;
            }

            let combined_balance = match self.merge_method {
                MergeMethod::Sum => {
                    // Sum all balances
                    balances_vec.iter().sum()
                }
                MergeMethod::FirstWins => {
                    // Use the first balance
                    balances_vec[0]
                }
                MergeMethod::DateSplit => {
                    return Err(crate::error::ComputeError::Runtime(
                        "Not implemented DateSplit for regular merge".to_owned(),
                    ));
                }
            };

            account_ids.push(account_id);
            dates.push(date);
            balances.push(combined_balance.to_string());
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
    ) -> Result<DataFrame> {
        debug!(
            "Computing merged account state for {} accounts from {} to {}",
            accounts.len(),
            start_date,
            end_date
        );

        // If there are no calculators, return an empty DataFrame
        if self.calculators.is_empty() {
            debug!("No calculators to merge, creating empty DataFrame");
            let empty_df = DataFrame::new(vec![
                Series::new("account_id".into(), Vec::<i32>::new()).into(),
                Series::new("date".into(), Vec::<NaiveDate>::new()).into(),
                Series::new("balance".into(), Vec::<String>::new()).into(),
            ])?;
            return Ok(empty_df);
        }

        // If there's only one calculator, use it directly
        if self.calculators.len() == 1 {
            debug!("Only one calculator to merge, using it directly");
            return self.calculators[0]
                .compute_account_state(db, accounts, start_date, end_date)
                .await;
        }

        // For Sum merge method, compute each calculator and then sum the results
        if self.merge_method == MergeMethod::Sum {
            debug!("Using Sum merge method");

            // Create a map to store all data by (account_id, date) pair
            let mut sum_map: HashMap<(i32, NaiveDate), rust_decimal::Decimal> = HashMap::new();

            // Process each calculator
            for (i, calculator) in self.calculators.iter().enumerate() {
                debug!("Computing account state using calculator {}", i);
                let df = calculator
                    .compute_account_state(db, accounts, start_date, end_date)
                    .await?;
                debug!(
                    "Calculator {} returned DataFrame with {} rows",
                    i,
                    df.height()
                );

                // Process each row in the DataFrame
                let account_id_col = df.column("account_id")?;
                let date_col = df.column("date")?;
                let balance_col = df.column("balance")?;

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

                    // Parse balance and add to sum
                    if let Ok(balance) = balance_str.parse::<rust_decimal::Decimal>() {
                        let key = (account_id, date);
                        let entry = sum_map.entry(key).or_insert(rust_decimal::Decimal::ZERO);
                        *entry += balance;
                    } else {
                        warn!("Failed to parse balance '{}' as Decimal", balance_str);
                    }
                }
            }

            // Create result DataFrame
            let mut account_ids = Vec::new();
            let mut dates = Vec::new();
            let mut balances = Vec::new();

            for ((account_id, date), balance) in sum_map {
                account_ids.push(account_id);
                dates.push(date);
                balances.push(balance.to_string());
            }

            let result_df = DataFrame::new(vec![
                Series::new("account_id".into(), account_ids).into(),
                Series::new("date".into(), dates).into(),
                Series::new("balance".into(), balances).into(),
            ])?;

            debug!("Merged DataFrame has {} rows", result_df.height());
            return Ok(result_df);
        }

        // For FirstWins merge method, use the last calculator's result for each (account_id, date) pair
        if self.merge_method == MergeMethod::FirstWins {
            debug!("Using FirstWins merge method");

            // Create a map to store data by (account_id, date) pair
            let mut first_wins_map: HashMap<(i32, NaiveDate), String> = HashMap::new();

            // Process each calculator in reverse order
            // This ensures that the last calculator in the list is processed first
            // and its values will be overwritten by earlier calculators
            for (i, calculator) in self.calculators.iter().enumerate().rev() {
                debug!("Computing account state using calculator {}", i);
                let df = calculator
                    .compute_account_state(db, accounts, start_date, end_date)
                    .await?;
                debug!(
                    "Calculator {} returned DataFrame with {} rows",
                    i,
                    df.height()
                );

                // Process each row in the DataFrame
                let account_id_col = df.column("account_id")?;
                let date_col = df.column("date")?;
                let balance_col = df.column("balance")?;

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

                    // Always insert or overwrite
                    // This ensures that the last calculator in the original list wins
                    first_wins_map.insert(key, balance_str);
                }
            }

            // Create result DataFrame
            let mut account_ids = Vec::new();
            let mut dates = Vec::new();
            let mut balances = Vec::new();

            for ((account_id, date), balance) in first_wins_map {
                account_ids.push(account_id);
                dates.push(date);
                balances.push(balance);
            }

            let result_df = DataFrame::new(vec![
                Series::new("account_id".into(), account_ids).into(),
                Series::new("date".into(), dates).into(),
                Series::new("balance".into(), balances).into(),
            ])?;

            debug!("Merged DataFrame has {} rows", result_df.height());
            return Ok(result_df);
        }

        // For DateSplit merge method, not implemented
        return Err(crate::error::ComputeError::Runtime(
            "Not implemented DateSplit for regular merge".to_owned(),
        ));
    }

    fn merge_method(&self) -> MergeMethod {
        self.merge_method
    }
}

/// Merges balance and forecast data for accounts within a specified date range.
///
/// This function computes the balance up to today and the forecast for future dates,
/// then merges them together into a single DataFrame.
#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date
))]
async fn compute_merged(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
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
        .compute_account_state(db, accounts, start_date, end_date)
        .await?;

    info!(
        "Merged computation completed successfully with {} data points",
        result.height()
    );
    Ok(result)
}

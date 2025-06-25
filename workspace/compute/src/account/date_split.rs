use async_trait::async_trait;
use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::cell::RefCell;
use std::str::FromStr;
use tracing::{debug, info, instrument};

use super::{AccountStateCalculator, MergeMethod};
use crate::error::Result;

/// A calculator that uses different calculators before and after a specific split date.
pub struct DateSplitCalculator {
    /// The calculator to use for dates before the split date.
    first_calculator: Box<dyn AccountStateCalculator + Send + Sync>,
    /// The calculator to use for dates on or after the split date.
    second_calculator: Box<dyn AccountStateCalculator + Send + Sync>,
    /// The date at which to switch from the first calculator to the second calculator.
    split_date: NaiveDate,
    /// Flag indicating whether to transfer the balance from the first calculator to the second calculator.
    transfer_balance: bool,
}

impl DateSplitCalculator {
    /// Creates a new date split calculator with the specified calculators and split date.
    /// By default, balance transfer is disabled.
    pub fn new(
        first_calculator: Box<dyn AccountStateCalculator + Send + Sync>,
        second_calculator: Box<dyn AccountStateCalculator + Send + Sync>,
        split_date: NaiveDate,
    ) -> Self {
        Self {
            first_calculator,
            second_calculator,
            split_date,
            transfer_balance: false,
        }
    }

    /// Creates a new date split calculator with the specified calculators and split date.
    /// Enables balance transfer from the first calculator to the second calculator.
    pub fn new_with_balance_transfer(
        first_calculator: Box<dyn AccountStateCalculator + Send + Sync>,
        second_calculator: Box<dyn AccountStateCalculator + Send + Sync>,
        split_date: NaiveDate,
    ) -> Self {
        Self {
            first_calculator,
            second_calculator,
            split_date,
            transfer_balance: true,
        }
    }

    /// Creates a new date split calculator with the specified first calculator and a factory function
    /// for creating the second calculator.
    ///
    /// This method computes the balance on the split date using the first calculator and passes it
    /// to the factory function, which can then use it to create the second calculator with the
    /// appropriate initial balance.
    ///
    /// # Arguments
    ///
    /// * `first_calculator` - The calculator to use for dates before the split date
    /// * `second_calculator_factory` - A function that takes a Decimal balance and returns a boxed calculator
    /// * `split_date` - The date at which to switch from the first calculator to the second calculator
    /// * `db` - The database connection for retrieving account data
    /// * `accounts` - The accounts to calculate state for
    ///
    /// # Returns
    ///
    /// A new DateSplitCalculator with balance transfer enabled, or an error if the computation fails
    pub async fn new_with_balance_factory<F>(
        first_calculator: Box<dyn AccountStateCalculator + Send + Sync>,
        second_calculator_factory: F,
        split_date: NaiveDate,
        db: &DatabaseConnection,
        accounts: &[account::Model],
    ) -> Result<Self>
    where
        F: FnOnce(Decimal) -> Box<dyn AccountStateCalculator + Send + Sync>,
    {
        // Compute the account state on the split date using the first calculator
        let split_date_df = first_calculator
            .compute_account_state(db, accounts, split_date, split_date)
            .await?;

        debug!("First calculator returned DataFrame with {} rows for split date", split_date_df.height());

        // Extract the balance on the split date for each account
        let mut account_balances = std::collections::HashMap::new();
        for i in 0..split_date_df.height() {
            let account_id = split_date_df.column("account_id")?.get(i).unwrap().try_extract::<i32>().unwrap();
            let date_val = split_date_df.column("date")?.get(i).unwrap();
            let date_str = date_val.to_string();
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap();

            // If this is the balance on the split date, store it
            if date == split_date {
                let balance_str = split_date_df.column("balance")?.get(i).unwrap().to_string();
                let balance_str = balance_str.trim_matches('"');
                if let Ok(balance) = Decimal::from_str(balance_str) {
                    account_balances.insert(account_id, balance);
                    debug!("Extracted balance for account {} on split date {}: {}", account_id, split_date, balance);
                }
            }
        }

        // Determine the balance to use for the second calculator
        let balance = if accounts.len() == 1 && account_balances.len() == 1 {
            // If there's only one account, use its balance directly
            let account_id = accounts[0].id;
            if let Some(balance) = account_balances.get(&account_id) {
                debug!("Using balance {} for second calculator", balance);
                *balance
            } else {
                debug!("No balance found for account {}, using zero", account_id);
                Decimal::ZERO
            }
        } else if !account_balances.is_empty() {
            // If there are multiple accounts, use the sum of their balances
            let total_balance: Decimal = account_balances.values().sum();
            debug!("Using total balance {} for second calculator (sum of {} accounts)", total_balance, account_balances.len());
            total_balance
        } else {
            debug!("No balances found, using zero");
            Decimal::ZERO
        };

        // Create the second calculator using the factory function and the computed balance
        let second_calculator = second_calculator_factory(balance);

        Ok(Self {
            first_calculator,
            second_calculator,
            split_date,
            transfer_balance: true,
        })
    }
}

#[async_trait]
impl AccountStateCalculator for DateSplitCalculator {
    #[instrument(skip(self, db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date))]
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        debug!(
            "Computing date-split account state for {} accounts from {} to {} with split date {}",
            accounts.len(),
            start_date,
            end_date,
            self.split_date
        );

        // If the entire date range is before the split date, use only the first calculator
        if end_date < self.split_date {
            debug!("Entire date range is before split date, using only first calculator");
            return self.first_calculator.compute_account_state(db, accounts, start_date, end_date).await;
        }

        // If the entire date range is on or after the split date, use only the second calculator
        if start_date >= self.split_date {
            debug!("Entire date range is on or after split date, using only second calculator");
            return self.second_calculator.compute_account_state(db, accounts, start_date, end_date).await;
        }

        // Otherwise, we need to use both calculators and merge the results
        debug!("Date range spans split date, using both calculators");

        // Compute account state using the first calculator for dates before the split date
        let first_df = self.first_calculator
            .compute_account_state(db, accounts, start_date, self.split_date.pred_opt().unwrap())
            .await?;
        debug!("First calculator returned DataFrame with {} rows", first_df.height());

        // Compute account state using the second calculator for dates on or after the split date
        let second_df = self.second_calculator
            .compute_account_state(db, accounts, self.split_date, end_date)
            .await?;
        debug!("Second calculator returned DataFrame with {} rows", second_df.height());

        // Concatenate the DataFrames by extracting and combining their data
        let mut account_ids = Vec::new();
        let mut dates = Vec::new();
        let mut balances = Vec::new();

        // Extract data from first DataFrame
        for i in 0..first_df.height() {
            let account_id = first_df.column("account_id")?.get(i).unwrap().try_extract::<i32>().unwrap();
            let date_val = first_df.column("date")?.get(i).unwrap();
            // Convert the date value to a string and then parse it
            let date_str = date_val.to_string();
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap();
            // Get the balance value and remove any surrounding quotes
            let balance_str = first_df.column("balance")?.get(i).unwrap().to_string();
            let balance = balance_str.trim_matches('"').to_string();

            account_ids.push(account_id);
            dates.push(date);
            balances.push(balance);
        }

        // Extract data from second DataFrame
        for i in 0..second_df.height() {
            let account_id = second_df.column("account_id")?.get(i).unwrap().try_extract::<i32>().unwrap();
            let date_val = second_df.column("date")?.get(i).unwrap();
            // Convert the date value to a string and then parse it
            let date_str = date_val.to_string();
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap();
            // Get the balance value and remove any surrounding quotes
            let balance_str = second_df.column("balance")?.get(i).unwrap().to_string();
            let balance = balance_str.trim_matches('"').to_string();

            account_ids.push(account_id);
            dates.push(date);
            balances.push(balance);
        }

        // Create a new DataFrame with the combined data
        let result_df = DataFrame::new(vec![
            Series::new("account_id".into(), account_ids).into(),
            Series::new("date".into(), dates).into(),
            Series::new("balance".into(), balances).into(),
        ])?;

        debug!("Merged DataFrame has {} rows", result_df.height());

        info!(
            "Date-split computation completed successfully with {} data points",
            result_df.height()
        );
        Ok(result_df)
    }

    fn merge_method(&self) -> MergeMethod {
        MergeMethod::DateSplit
    }
}

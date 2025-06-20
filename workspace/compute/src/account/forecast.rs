pub mod recurring;

use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use tracing::{debug, info, instrument, trace};

use super::{AccountStateCalculator, MergeMethod};
use crate::error::Result;

use self::recurring::{get_recurring_income, get_recurring_transactions};

/// A calculator that computes account forecasts based on recurring transactions and income.
pub struct ForecastCalculator {
    /// The merge method to use when combining results from multiple calculators.
    merge_method: MergeMethod,
    /// The initial balance to use when computing forecasts.
    initial_balance: Decimal,
    /// The date to use as "today" for determining which recurring transactions to include.
    today: Option<NaiveDate>,
    /// The offset to use for past recurring transactions without instances.
    future_offset: Option<Duration>,
}

impl ForecastCalculator {
    /// Creates a new forecast calculator with the specified merge method.
    pub fn new(merge_method: MergeMethod) -> Self {
        Self { 
            merge_method,
            initial_balance: Decimal::ZERO,
            today: None,
            future_offset: None,
        }
    }

    /// Creates a new forecast calculator with the specified merge method and initial balance.
    pub fn new_with_initial_balance(merge_method: MergeMethod, initial_balance: Decimal) -> Self {
        Self { 
            merge_method,
            initial_balance,
            today: None,
            future_offset: None,
        }
    }

    /// Creates a new forecast calculator with the specified merge method, initial balance, today date, and future offset.
    pub fn new_with_params(
        merge_method: MergeMethod, 
        initial_balance: Decimal,
        today: NaiveDate,
        future_offset: Duration,
    ) -> Self {
        Self { 
            merge_method,
            initial_balance,
            today: Some(today),
            future_offset: Some(future_offset),
        }
    }

    /// Creates a new forecast calculator with the default merge method (FirstWins) and zero initial balance.
    pub fn default() -> Self {
        Self {
            merge_method: MergeMethod::FirstWins,
            initial_balance: Decimal::ZERO,
            today: None,
            future_offset: None,
        }
    }
}

#[async_trait]
impl AccountStateCalculator for ForecastCalculator {
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        // Use the provided today date or default to the current date
        let today = self.today.unwrap_or_else(|| chrono::Local::now().date_naive());
        // Use the provided future offset or default to 7 days
        let future_offset = self.future_offset.unwrap_or_else(|| Duration::days(7));

        compute_forecast(db, accounts, start_date, end_date, self.initial_balance, today, future_offset).await
    }

    fn merge_method(&self) -> MergeMethod {
        self.merge_method
    }
}

/// Computes the forecast for accounts within a specified date range.
///
/// This function takes into account:
/// - Recurring transactions and income
///
/// It considers transactions where the account is both source and target.
/// 
/// The initial_balance parameter allows setting a starting balance for the forecast,
/// which is useful when computing forecasts for dates outside the range of available transaction data.
#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date, today = %today, future_offset = %future_offset.num_days()))]
async fn compute_forecast(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
    initial_balance: Decimal,
    today: NaiveDate,
    future_offset: Duration,
) -> Result<DataFrame> {
    info!(
        "Computing forecast for {} accounts from {} to {}",
        accounts.len(),
        start_date,
        end_date
    );
    // Create a DataFrame with account_id and date as index, and balance as value
    let mut forecast_data: HashMap<(i32, NaiveDate), Decimal> = HashMap::new();

    // Process each account
    for account in accounts {
        debug!(
            "Processing account: id={}, name={}",
            account.id, account.name
        );

        // Initialize balance with the provided initial balance
        let mut current_balance = initial_balance;
        let current_date = start_date;
        trace!(
            "Initialized balance for account {} to {}, starting from {}",
            account.id, current_balance, current_date
        );

        // Get all recurring transactions for this account within the date range
        trace!(
            "Getting recurring transactions for account {} from {} to {} (today={}, future_offset={}d)",
            account.id, current_date, end_date, today, future_offset.num_days()
        );
        let recurring_transactions =
            get_recurring_transactions(db, account.id, current_date, end_date, today, future_offset).await?;
        debug!(
            "Found {} recurring transactions for account {}",
            recurring_transactions.len(),
            account.id
        );

        // Get all recurring income for this account within the date range
        trace!(
            "Getting recurring income for account {} from {} to {} (today={}, future_offset={}d)",
            account.id, current_date, end_date, today, future_offset.num_days()
        );
        let recurring_income = get_recurring_income(db, account.id, current_date, end_date, today, future_offset).await?;
        debug!(
            "Found {} recurring income entries for account {}",
            recurring_income.len(),
            account.id
        );

        // Combine all transactions and sort by date
        trace!("Combining all transactions for account {}", account.id);
        let mut all_transactions = Vec::new();

        // Add recurring transactions
        trace!(
            "Processing recurring transactions for account {}",
            account.id
        );
        for (date, tx) in recurring_transactions {
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };

            trace!(
                "Adding recurring transaction: date={}, amount={}",
                date, amount
            );
            all_transactions.push((date, amount));
        }

        // Add recurring income
        trace!("Processing recurring income for account {}", account.id);
        for (date, income) in recurring_income {
            if income.target_account_id == account.id {
                trace!(
                    "Adding recurring income: date={}, amount={}",
                    date, income.amount
                );
                all_transactions.push((date, income.amount));
            }
        }

        // Sort transactions by date
        all_transactions.sort_by(|a, b| a.0.cmp(&b.0));
        debug!(
            "Combined {} transactions for account {}",
            all_transactions.len(),
            account.id
        );

        // Process transactions and update balance
        debug!(
            "Processing transactions and calculating forecast for account {}",
            account.id
        );
        let mut date = current_date;
        let mut tx_index = 0;

        while date <= end_date {
            // Process all transactions for this date
            let day_start_balance = current_balance;
            let day_start_tx_index = tx_index;

            while tx_index < all_transactions.len() && all_transactions[tx_index].0 == date {
                trace!(
                    "Processing transaction for account {} on {}: amount={}",
                    account.id, date, all_transactions[tx_index].1
                );
                current_balance += all_transactions[tx_index].1;
                tx_index += 1;
            }

            if tx_index > day_start_tx_index {
                trace!(
                    "Forecast for account {} on {} changed from {} to {}",
                    account.id, date, day_start_balance, current_balance
                );
            }

            // Store balance for this date
            trace!(
                "Storing forecast for account {} on {}: {}",
                account.id, date, current_balance
            );
            forecast_data.insert((account.id, date), current_balance);

            // Move to next date
            date = date.succ_opt().unwrap();
        }
    }

    // Convert the HashMap to a DataFrame
    debug!("Converting forecast data to DataFrame");
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();
    let mut balances = Vec::new();

    for ((account_id, date), balance) in forecast_data {
        trace!(
            "Adding forecast data point: account_id={}, date={}, balance={}",
            account_id, date, balance
        );
        account_ids.push(account_id);
        dates.push(date);
        balances.push(balance.to_string());
    }

    debug!(
        "Creating DataFrame with {} forecast data points",
        account_ids.len()
    );
    let df = DataFrame::new(vec![
        Series::new("account_id".into(), account_ids).into(),
        Series::new("date".into(), dates).into(),
        Series::new("balance".into(), balances).into(),
    ])?;

    info!(
        "Forecast computation completed successfully with {} data points",
        df.height()
    );
    Ok(df)
}

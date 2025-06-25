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

use super::forecast::recurring::{get_recurring_income, get_recurring_transactions};

/// A calculator that computes future non-paid recurring transactions and income.
/// 
/// This calculator is responsible for projecting recurring transactions and income
/// that have not yet been paid (no instance created) into the future.
pub struct UnpaidRecurringCalculator {
    /// The merge method to use when combining results from multiple calculators.
    merge_method: MergeMethod,
    /// The date to use as "today" for determining which recurring transactions to include.
    today: NaiveDate,
    /// The offset to use for past recurring transactions without instances.
    future_offset: Duration,
}

impl UnpaidRecurringCalculator {
    /// Creates a new unpaid recurring calculator with the specified merge method, today date, and future offset.
    pub fn new(
        merge_method: MergeMethod,
        today: NaiveDate,
        future_offset: Duration,
    ) -> Self {
        Self {
            merge_method,
            today,
            future_offset,
        }
    }

    /// Creates a new unpaid recurring calculator with the Sum merge method and the specified today date and future offset.
    pub fn new_with_sum_merge(
        today: NaiveDate,
        future_offset: Duration,
    ) -> Self {
        Self {
            merge_method: MergeMethod::Sum,
            today,
            future_offset,
        }
    }

    /// Creates a new unpaid recurring calculator with default values.
    pub fn default() -> Self {
        Self {
            merge_method: MergeMethod::Sum,
            today: chrono::Local::now().date_naive(),
            future_offset: Duration::days(7),
        }
    }
}

#[async_trait]
impl AccountStateCalculator for UnpaidRecurringCalculator {
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        compute_unpaid_recurring(
            db,
            accounts,
            start_date,
            end_date,
            self.today,
            self.future_offset,
        )
        .await
    }

    fn merge_method(&self) -> MergeMethod {
        self.merge_method
    }

    fn update_initial_balance(&mut self, _balance: rust_decimal::Decimal) -> bool {
        // This calculator doesn't use initial balance
        false
    }
}

/// Computes the unpaid recurring transactions and income for accounts within a specified date range.
///
/// This function takes into account:
/// - Past recurring transactions without instances, moving them to today + future_offset
/// - Past recurring income, moving it to today + future_offset
#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date, today = %today, future_offset = %future_offset.num_days()))]
async fn compute_unpaid_recurring(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    future_offset: Duration,
) -> Result<DataFrame> {
    info!(
        "Computing unpaid recurring transactions for {} accounts from {} to {}",
        accounts.len(),
        start_date,
        end_date
    );
    // Create a DataFrame with account_id and date as index, and balance as value
    let mut unpaid_data: HashMap<(i32, NaiveDate), Decimal> = HashMap::new();

    // Process each account
    for account in accounts {
        debug!(
            "Processing account: id={}, name={}",
            account.id, account.name
        );

        // Get all recurring transactions for this account within the date range
        trace!(
            "Getting recurring transactions for account {} from {} to {} (today={}, future_offset={}d)",
            account.id,
            start_date,
            end_date,
            today,
            future_offset.num_days()
        );
        let recurring_transactions = get_recurring_transactions(
            db,
            account.id,
            start_date,
            end_date,
            today,
            future_offset,
        )
        .await?;
        debug!(
            "Found {} recurring transactions for account {}",
            recurring_transactions.len(),
            account.id
        );

        // Get all recurring income for this account within the date range
        trace!(
            "Getting recurring income for account {} from {} to {} (today={}, future_offset={}d)",
            account.id,
            start_date,
            end_date,
            today,
            future_offset.num_days()
        );
        let recurring_income =
            get_recurring_income(db, account.id, start_date, end_date, today, future_offset)
                .await?;
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
            // Only include transactions that were moved to today + future_offset
            // This is the key difference from the forecast calculator
            if date == today + future_offset {
                let amount = if tx.target_account_id == account.id {
                    tx.amount
                } else if Some(account.id) == tx.source_account_id {
                    -tx.amount
                } else {
                    Decimal::ZERO
                };

                trace!(
                    "Adding unpaid recurring transaction: date={}, amount={}",
                    date, amount
                );
                all_transactions.push((date, amount));
            }
        }

        // Add recurring income
        trace!("Processing recurring income for account {}", account.id);
        for (date, income) in recurring_income {
            // Only include income that was moved to today + future_offset
            if date == today + future_offset && income.target_account_id == account.id {
                trace!(
                    "Adding unpaid recurring income: date={}, amount={}",
                    date, income.amount
                );
                all_transactions.push((date, income.amount));
            }
        }

        // Process transactions and update balances
        for (date, amount) in all_transactions {
            let key = (account.id, date);
            let entry = unpaid_data.entry(key).or_insert(Decimal::ZERO);
            *entry += amount;
            trace!(
                "Updated unpaid balance for account {} on {}: {}",
                account.id, date, entry
            );
        }
    }

    // Convert the HashMap to a DataFrame
    debug!("Converting unpaid recurring data to DataFrame");
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();
    let mut balances = Vec::new();

    for ((account_id, date), balance) in unpaid_data {
        trace!(
            "Adding unpaid data point: account_id={}, date={}, balance={}",
            account_id, date, balance
        );
        account_ids.push(account_id);
        dates.push(date);
        balances.push(balance.to_string());
    }

    debug!(
        "Creating DataFrame with {} unpaid recurring data points",
        account_ids.len()
    );
    let df = DataFrame::new(vec![
        Series::new("account_id".into(), account_ids).into(),
        Series::new("date".into(), dates).into(),
        Series::new("balance".into(), balances).into(),
    ])?;

    info!(
        "Unpaid recurring computation completed successfully with {} data points",
        df.height()
    );
    Ok(df)
}
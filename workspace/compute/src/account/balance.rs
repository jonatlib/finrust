pub mod account_state;
pub mod recurring;
pub mod transactions;

use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use tracing::{debug, info, instrument, trace};
use async_trait::async_trait;

use crate::error::{ComputeError, Result};
use super::{AccountStateCalculator, MergeMethod};

/// A calculator that computes account balances based on transactions and manual states.
pub struct BalanceCalculator {
    /// The merge method to use when combining results from multiple calculators.
    merge_method: MergeMethod,
}

impl BalanceCalculator {
    /// Creates a new balance calculator with the specified merge method.
    pub fn new(merge_method: MergeMethod) -> Self {
        Self { merge_method }
    }

    /// Creates a new balance calculator with the default merge method (FirstWins).
    pub fn default() -> Self {
        Self { merge_method: MergeMethod::FirstWins }
    }
}

#[async_trait]
impl AccountStateCalculator for BalanceCalculator {
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        compute_balance(db, accounts, start_date, end_date).await
    }

    fn merge_method(&self) -> MergeMethod {
        self.merge_method
    }
}

use self::{
    account_state::{get_latest_manual_state, get_manual_states_in_range},
    recurring::{get_recurring_income, get_recurring_transactions},
    transactions::{get_imported_transactions, get_transactions_for_account},
};

/// Computes the balance for accounts within a specified date range.
///
/// This function takes into account:
/// - Imported transactions
/// - Manual account states
/// - One-off transactions
/// - Recurring transactions and income
///
/// It considers transactions where the account is both source and target.
#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date))]
async fn compute_balance(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> crate::error::Result<DataFrame> {
    info!("Computing balance for {} accounts from {} to {}", accounts.len(), start_date, end_date);
    // Create a DataFrame with account_id and date as index, and balance as value
    let mut balance_data: HashMap<(i32, NaiveDate), Decimal> = HashMap::new();

    // Process each account
    for account in accounts {
        debug!("Processing account: id={}, name={}", account.id, account.name);

        // Get the latest manual account state before start_date
        trace!("Getting latest manual state for account {} before {}", account.id, start_date);
        let manual_state = get_latest_manual_state(db, account.id, start_date).await?;

        // Initialize balance with manual state or zero
        let mut current_balance = manual_state.as_ref().map(|s| s.amount).unwrap_or(Decimal::ZERO);
        let mut current_date = if let Some(state) = manual_state {
            debug!("Found manual state for account {}: date={}, amount={}", account.id, state.date, state.amount);
            state.date
        } else {
            debug!("No manual state found for account {}, starting from {}", account.id, start_date);
            // If no manual state, start from the beginning
            start_date
        };

        // Get all transactions for this account within the date range
        trace!("Getting transactions for account {} from {} to {}", account.id, current_date, end_date);
        let transactions = get_transactions_for_account(db, account.id, current_date, end_date).await?;
        debug!("Found {} transactions for account {}", transactions.len(), account.id);

        // Get all imported transactions for this account within the date range
        trace!("Getting imported transactions for account {} from {} to {}", account.id, current_date, end_date);
        let imported_transactions = get_imported_transactions(db, account.id, current_date, end_date).await?;
        debug!("Found {} imported transactions for account {}", imported_transactions.len(), account.id);

        // Get all recurring transactions for this account within the date range
        trace!("Getting recurring transactions for account {} from {} to {}", account.id, current_date, end_date);
        let recurring_transactions = get_recurring_transactions(db, account.id, current_date, end_date).await?;
        debug!("Found {} recurring transactions for account {}", recurring_transactions.len(), account.id);

        // Get all recurring income for this account within the date range
        trace!("Getting recurring income for account {} from {} to {}", account.id, current_date, end_date);
        let recurring_income = get_recurring_income(db, account.id, current_date, end_date).await?;
        debug!("Found {} recurring income entries for account {}", recurring_income.len(), account.id);

        // Combine all transactions and sort by date
        trace!("Combining all transactions for account {}", account.id);
        let mut all_transactions = Vec::new();

        // Add one-off transactions
        trace!("Processing one-off transactions for account {}", account.id);
        for tx in transactions {
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };

            trace!("Adding one-off transaction: date={}, amount={}", tx.date, amount);
            all_transactions.push((tx.date, amount));
        }

        // Add imported transactions that are not reconciled
        trace!("Processing imported transactions for account {}", account.id);
        for tx in imported_transactions {
            if tx.get_reconciled_transaction_type().is_none() {
                trace!("Adding imported transaction: date={}, amount={}", tx.date, tx.amount);
                all_transactions.push((tx.date, tx.amount));
            } else {
                trace!("Skipping reconciled imported transaction: date={}, amount={}", tx.date, tx.amount);
            }
        }

        // Add recurring transactions
        trace!("Processing recurring transactions for account {}", account.id);
        for (date, tx) in recurring_transactions {
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };

            trace!("Adding recurring transaction: date={}, amount={}", date, amount);
            all_transactions.push((date, amount));
        }

        // Add recurring income
        trace!("Processing recurring income for account {}", account.id);
        for (date, income) in recurring_income {
            if income.target_account_id == account.id {
                trace!("Adding recurring income: date={}, amount={}", date, income.amount);
                all_transactions.push((date, income.amount));
            }
        }

        // Sort transactions by date
        all_transactions.sort_by(|a, b| a.0.cmp(&b.0));
        debug!("Combined {} transactions for account {}", all_transactions.len(), account.id);

        // Process transactions and update balance
        debug!("Processing transactions and calculating balance for account {}", account.id);
        let mut date = current_date;
        let mut tx_index = 0;

        while date <= end_date {
            // Process all transactions for this date
            let day_start_balance = current_balance;
            let day_start_tx_index = tx_index;

            while tx_index < all_transactions.len() && all_transactions[tx_index].0 == date {
                trace!("Processing transaction for account {} on {}: amount={}", account.id, date, all_transactions[tx_index].1);
                current_balance += all_transactions[tx_index].1;
                tx_index += 1;
            }

            if tx_index > day_start_tx_index {
                trace!("Balance for account {} on {} changed from {} to {}", account.id, date, day_start_balance, current_balance);
            }

            // Store balance for this date
            trace!("Storing balance for account {} on {}: {}", account.id, date, current_balance);
            balance_data.insert((account.id, date), current_balance);

            // Move to next date
            date = date.succ_opt().unwrap();
        }

        // Check if we have any manual account states within the date range
        // These will override the computed balance
        trace!("Getting manual states for account {} from {} to {}", account.id, start_date, end_date);
        let manual_states = get_manual_states_in_range(db, account.id, start_date, end_date).await?;
        debug!("Found {} manual states for account {}", manual_states.len(), account.id);

        for state in manual_states {
            debug!("Processing manual state for account {}: date={}, amount={}", account.id, state.date, state.amount);
            balance_data.insert((account.id, state.date), state.amount);

            // Recalculate balances after this manual state
            let mut date = state.date.succ_opt().unwrap();
            let mut balance = state.amount;
            debug!("Recalculating balances for account {} after manual state on {}", account.id, state.date);

            while date <= end_date {
                // Find transactions for this date
                let day_transactions: Vec<_> = all_transactions
                    .iter()
                    .filter(|(tx_date, _)| *tx_date == date)
                    .collect();

                trace!("Found {} transactions for account {} on {}", day_transactions.len(), account.id, date);

                // Update balance with transactions
                let day_start_balance = balance;
                for (_, amount) in day_transactions {
                    trace!("Applying transaction: amount={}", amount);
                    balance += *amount;
                }

                if balance != day_start_balance {
                    trace!("Balance for account {} on {} changed from {} to {}", account.id, date, day_start_balance, balance);
                }

                // Store updated balance
                trace!("Storing updated balance for account {} on {}: {}", account.id, date, balance);
                balance_data.insert((account.id, date), balance);

                // Move to next date
                date = date.succ_opt().unwrap();
            }
        }
    }

    // Convert the HashMap to a DataFrame
    debug!("Converting balance data to DataFrame");
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();
    let mut balances = Vec::new();

    for ((account_id, date), balance) in balance_data {
        trace!("Adding balance data point: account_id={}, date={}, balance={}", account_id, date, balance);
        account_ids.push(account_id);
        dates.push(date);
        balances.push(balance.to_string());
    }

    debug!("Creating DataFrame with {} balance data points", account_ids.len());
    let df = DataFrame::new(vec![
        Series::new("account_id".into(), account_ids).into(),
        Series::new("date".into(), dates).into(),
        Series::new("balance".into(), balances).into(),
    ])?;

    info!("Balance computation completed successfully with {} data points", df.height());
    Ok(df)
}

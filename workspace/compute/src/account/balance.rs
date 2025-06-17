pub mod account_state;
pub mod recurring;
pub mod transactions;

use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

use crate::error::{ComputeError, Result};

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
pub async fn compute_balance(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> crate::error::Result<DataFrame> {
    // Create a DataFrame with account_id and date as index, and balance as value
    let mut balance_data: HashMap<(i32, NaiveDate), Decimal> = HashMap::new();

    // Process each account
    for account in accounts {
        // Get the latest manual account state before start_date
        let manual_state = get_latest_manual_state(db, account.id, start_date).await?;

        // Initialize balance with manual state or zero
        let mut current_balance = manual_state.as_ref().map(|s| s.amount).unwrap_or(Decimal::ZERO);
        let mut current_date = if let Some(state) = manual_state {
            state.date
        } else {
            // If no manual state, start from the beginning
            start_date
        };

        // Get all transactions for this account within the date range
        let transactions = get_transactions_for_account(db, account.id, current_date, end_date).await?;

        // Get all imported transactions for this account within the date range
        let imported_transactions = get_imported_transactions(db, account.id, current_date, end_date).await?;

        // Get all recurring transactions for this account within the date range
        let recurring_transactions = get_recurring_transactions(db, account.id, current_date, end_date).await?;

        // Get all recurring income for this account within the date range
        let recurring_income = get_recurring_income(db, account.id, current_date, end_date).await?;

        // Combine all transactions and sort by date
        let mut all_transactions = Vec::new();

        // Add one-off transactions
        for tx in transactions {
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };

            all_transactions.push((tx.date, amount));
        }

        // Add imported transactions that are not reconciled
        for tx in imported_transactions {
            if tx.get_reconciled_transaction_type().is_none() {
                all_transactions.push((tx.date, tx.amount));
            }
        }

        // Add recurring transactions
        for (date, tx) in recurring_transactions {
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };

            all_transactions.push((date, amount));
        }

        // Add recurring income
        for (date, income) in recurring_income {
            if income.target_account_id == account.id {
                all_transactions.push((date, income.amount));
            }
        }

        // Sort transactions by date
        all_transactions.sort_by(|a, b| a.0.cmp(&b.0));

        // Process transactions and update balance
        let mut date = current_date;
        let mut tx_index = 0;

        while date <= end_date {
            // Process all transactions for this date
            while tx_index < all_transactions.len() && all_transactions[tx_index].0 == date {
                current_balance += all_transactions[tx_index].1;
                tx_index += 1;
            }

            // Store balance for this date
            balance_data.insert((account.id, date), current_balance);

            // Move to next date
            date = date.succ_opt().unwrap();
        }

        // Check if we have any manual account states within the date range
        // These will override the computed balance
        let manual_states = get_manual_states_in_range(db, account.id, start_date, end_date).await?;
        for state in manual_states {
            balance_data.insert((account.id, state.date), state.amount);

            // Recalculate balances after this manual state
            let mut date = state.date.succ_opt().unwrap();
            let mut balance = state.amount;

            while date <= end_date {
                // Find transactions for this date
                let day_transactions: Vec<_> = all_transactions
                    .iter()
                    .filter(|(tx_date, _)| *tx_date == date)
                    .collect();

                // Update balance with transactions
                for (_, amount) in day_transactions {
                    balance += *amount;
                }

                // Store updated balance
                balance_data.insert((account.id, date), balance);

                // Move to next date
                date = date.succ_opt().unwrap();
            }
        }
    }

    // Convert the HashMap to a DataFrame
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();
    let mut balances = Vec::new();

    for ((account_id, date), balance) in balance_data {
        account_ids.push(account_id);
        dates.push(date);
        balances.push(balance.to_string());
    }

    let df = DataFrame::new(vec![
        Series::new("account_id".into(), account_ids).into(),
        Series::new("date".into(), dates).into(),
        Series::new("balance".into(), balances).into(),
    ])?;

    Ok(df)
}

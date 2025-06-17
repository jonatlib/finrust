pub mod recurring;

use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

use crate::error::Result;

use self::recurring::{get_recurring_income, get_recurring_transactions};

/// Computes the forecast for accounts within a specified date range.
///
/// This function takes into account:
/// - Recurring transactions and income
///
/// It considers transactions where the account is both source and target.
pub async fn compute_forecast(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<DataFrame> {
    // Create a DataFrame with account_id and date as index, and balance as value
    let mut forecast_data: HashMap<(i32, NaiveDate), Decimal> = HashMap::new();

    // Process each account
    for account in accounts {
        // Initialize balance with zero
        let mut current_balance = Decimal::ZERO;
        let current_date = start_date;

        // Get all recurring transactions for this account within the date range
        let recurring_transactions = get_recurring_transactions(db, account.id, current_date, end_date).await?;

        // Get all recurring income for this account within the date range
        let recurring_income = get_recurring_income(db, account.id, current_date, end_date).await?;

        // Combine all transactions and sort by date
        let mut all_transactions = Vec::new();

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
            forecast_data.insert((account.id, date), current_balance);

            // Move to next date
            date = date.succ_opt().unwrap();
        }
    }

    // Convert the HashMap to a DataFrame
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();
    let mut balances = Vec::new();

    for ((account_id, date), balance) in forecast_data {
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

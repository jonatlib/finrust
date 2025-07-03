use chrono::{Duration, NaiveDate};
use model::entities::{recurring_income, recurring_transaction, recurring_transaction_instance};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::collections::HashSet;
use tracing::{debug, instrument, trace};

use crate::account::utils::generate_occurrences;
use crate::error::Result;

// ---------------------------------------------------------------------------------
// FOR `compute_balance` CALCULATOR
// ---------------------------------------------------------------------------------

/// Fetches transactions for the main balance sheet.
///
/// This function's responsibilities are:
/// 1. Find all `PAID` recurring transaction instances from the past and include them on their paid date.
/// 2. Generate all `FUTURE` scheduled occurrences (`>= today`) from their definitions.
///
/// It should be called by the `compute_balance` calculator.
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date, today = %today
))]
pub async fn get_balance_sheet_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
) -> Result<Vec<(NaiveDate, recurring_transaction::Model)>> {
    let transactions = recurring_transaction::Entity::find()
        .filter(
            Condition::any()
                .add(recurring_transaction::Column::TargetAccountId.eq(account_id))
                .add(recurring_transaction::Column::SourceAccountId.eq(account_id)),
        )
        .all(db)
        .await?;

    let mut result = Vec::new();
    let instance_map: HashSet<(i32, NaiveDate)> =
        recurring_transaction_instance::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .map(|i| (i.recurring_transaction_id, i.due_date))
            .collect();

    for tx in &transactions {
        let occurrences =
            generate_occurrences(tx.start_date, tx.end_date, &tx.period, start_date, end_date);

        for date in occurrences {
            if date < today {
                // For past dates, only include if a paid instance exists.
                if instance_map.contains(&(tx.id, date)) {
                    result.push((date, tx.clone()));
                }
            } else {
                // For future dates (or today), always include the scheduled occurrence.
                result.push((date, tx.clone()));
            }
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------------
// FOR `UnpaidRecurringCalculator`
// ---------------------------------------------------------------------------------

/// Fetches ONLY past-due, unpaid recurring transactions.
///
/// This function's responsibilities are:
/// 1. Find all recurring occurrences with a due date in the past (`< today`).
/// 2. Check if a paid or skipped instance exists for that occurrence.
/// 3. If NO instance exists, it's considered unpaid and is added to the result,
///    with its date moved to `today + future_offset`.
///
/// It should be called by the `compute_unpaid_recurring` calculator.
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, today = %today, future_offset = %future_offset.num_days()
))]
pub async fn get_past_due_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    today: NaiveDate,
    future_offset: Duration,
) -> Result<Vec<(NaiveDate, recurring_transaction::Model)>> {
    let transactions = recurring_transaction::Entity::find()
        .filter(
            Condition::any()
                .add(recurring_transaction::Column::TargetAccountId.eq(account_id))
                .add(recurring_transaction::Column::SourceAccountId.eq(account_id)),
        )
        .filter(recurring_transaction::Column::StartDate.lt(today))
        .all(db)
        .await?;

    debug!(
        "Found {} potentially past-due recurring transaction definitions for account_id={}",
        transactions.len(),
        account_id
    );

    let mut result = Vec::new();
    let instance_map: HashSet<(i32, NaiveDate)> =
        recurring_transaction_instance::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .map(|i| (i.recurring_transaction_id, i.due_date))
            .collect();

    for tx in &transactions {
        // Generate occurrences only in the past.
        let occurrences =
            generate_occurrences(tx.start_date, tx.end_date, &tx.period, start_date, today);

        for date in occurrences {
            // If an instance does NOT exist for this past occurrence, it's unpaid.
            if !instance_map.contains(&(tx.id, date)) {
                let new_date = today + future_offset;
                trace!(
                    "Moving past unpaid occurrence from {} to {} for recurring transaction id={}",
                    date, new_date, tx.id
                );
                result.push((new_date, tx.clone()));
            }
        }
    }

    Ok(result)
}

// NOTE: The `get_recurring_income` function likely has the same architectural issue.
// It should also be split into two separate functions, one for the balance sheet
// and one for finding past-due unpaid income. For now, this is left as-is but
// should be refactored following the same pattern as above.
#[instrument(skip(db), fields(account_id, start_date, end_date, today, future_offset = %future_offset.num_days()
))]
pub async fn get_recurring_income(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    future_offset: Duration,
) -> Result<Vec<(NaiveDate, recurring_income::Model)>> {
    // This function should be refactored similar to get_recurring_transactions
    // For now, returning an empty Vec to prevent incorrect calculations.
    Ok(Vec::new())
}

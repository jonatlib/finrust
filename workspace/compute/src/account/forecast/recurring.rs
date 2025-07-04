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
// FUNCTION FOR `compute_balance` (Calculator 0)
// ---------------------------------------------------------------------------------

/// Fetches transactions for the main balance sheet.
///
/// This function's responsibilities are:
/// 1. Find all `PAID` recurring transaction instances from the past and include them on their due date.
/// 2. Generate all `FUTURE` scheduled occurrences (`>= today`) from their definitions.
#[instrument(skip(db), fields(account_id, start_date, end_date, today))]
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
            .filter(recurring_transaction_instance::Column::Status.eq(recurring_transaction_instance::InstanceStatus::Paid))
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
                if instance_map.contains(&(tx.id, date)) {
                    result.push((date, tx.clone()));
                }
            } else {
                result.push((date, tx.clone()));
            }
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------------
// FUNCTION FOR `UnpaidRecurringCalculator` (Calculator 1)
// ---------------------------------------------------------------------------------

/// Fetches ONLY past-due, unpaid recurring transactions.
///
/// This function's responsibilities are:
/// 1. Find all recurring occurrences with a due date in the past (`< today`).
/// 2. Check if a paid or skipped instance exists for that occurrence.
/// 3. If NO instance exists, it's considered unpaid and is added to the result,
///    with its date moved to `today + future_offset`.
#[instrument(skip(db), fields(account_id, start_date, today, future_offset = %future_offset.num_days()
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

    let mut result = Vec::new();
    let instance_map: HashSet<(i32, NaiveDate)> =
        recurring_transaction_instance::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .map(|i| (i.recurring_transaction_id, i.due_date))
            .collect();

    for tx in &transactions {
        // Generate occurrences only in the past, from the transaction's own start date.
        let occurrences =
            generate_occurrences(tx.start_date, tx.end_date, &tx.period, tx.start_date, today);

        for date in occurrences {
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

#[instrument(skip(db), fields(account_id, start_date, end_date, today, future_offset = %_future_offset.num_days()
))]
pub async fn get_recurring_income(
    db: &DatabaseConnection,
    _account_id: i32,
    _start_date: NaiveDate,
    _end_date: NaiveDate,
    _today: NaiveDate,
    _future_offset: Duration,
) -> Result<Vec<(NaiveDate, recurring_income::Model)>> {
    // This function should be refactored similar to the transaction functions.
    Ok(Vec::new())
}

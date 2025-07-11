use chrono::{Duration, NaiveDate};
use model::entities::{recurring_income, recurring_transaction, recurring_transaction_instance};
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
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
    let instance_map: HashSet<(i32, NaiveDate)> = recurring_transaction_instance::Entity::find()
        .filter(
            recurring_transaction_instance::Column::Status
                .eq(recurring_transaction_instance::InstanceStatus::Paid),
        )
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
    let instance_map: HashSet<(i32, NaiveDate)> = recurring_transaction_instance::Entity::find()
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

/// Gets all future recurring income for the account within the given date range.
/// As per your requirement, income cannot be past-due, so this function only
/// finds occurrences on or after `today`.
#[instrument(skip(db), fields(account_id, start_date, end_date, today))]
pub async fn get_recurring_income(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    _future_offset: Duration, // Not used for income
) -> Result<Vec<(NaiveDate, recurring_income::Model)>> {
    trace!(
        "Getting future recurring income for account_id={} from {} to {}",
        account_id, start_date, end_date,
    );

    let incomes = recurring_income::Entity::find()
        .filter(recurring_income::Column::TargetAccountId.eq(account_id))
        .filter(
            Condition::any()
                .add(recurring_income::Column::EndDate.is_null())
                .add(recurring_income::Column::EndDate.gte(start_date)),
        )
        .filter(recurring_income::Column::StartDate.lte(end_date))
        .all(db)
        .await?;

    debug!(
        "Found {} recurring income definitions for account_id={}",
        incomes.len(),
        account_id
    );

    let mut result = Vec::new();

    for income in &incomes {
        let occurrences = generate_occurrences(
            income.start_date,
            income.end_date,
            &income.period,
            start_date,
            end_date,
        );

        for date in occurrences {
            if date >= today {
                // Future recurring income is treated as if it were accounted on its date
                trace!(
                    "Adding future occurrence on {} for recurring income id={}",
                    date, income.id
                );
                result.push((date, income.clone()));
            }
        }
    }

    debug!(
        "Returning {} total recurring income occurrences for account_id={}",
        result.len(),
        account_id
    );
    Ok(result)
}

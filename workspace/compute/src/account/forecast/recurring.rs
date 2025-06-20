use chrono::{Datelike, NaiveDate};
use model::entities::{one_off_transaction, recurring_income, recurring_transaction};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use tracing::{debug, instrument, trace};

use crate::account::utils::generate_occurrences;
use crate::error::Result;

/// Gets all recurring transactions for the account within the given date range.
/// Returns a vector of (date, transaction) pairs for all occurrences within the range.
/// 
/// For the forecast model, past recurring transactions without a linked one-off transaction
/// are moved forward in time, as they are considered "not paid yet".
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date, today = %today
))]
pub async fn get_recurring_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
) -> Result<Vec<(NaiveDate, recurring_transaction::Model)>> {
    trace!(
        "Getting recurring transactions for account_id={} from {} to {}",
        account_id, start_date, end_date
    );

    let transactions = recurring_transaction::Entity::find()
        .filter(
            Condition::any()
                .add(recurring_transaction::Column::TargetAccountId.eq(account_id))
                .add(recurring_transaction::Column::SourceAccountId.eq(account_id)),
        )
        .filter(
            Condition::any()
                .add(recurring_transaction::Column::EndDate.is_null())
                .add(recurring_transaction::Column::EndDate.gte(start_date)),
        )
        .filter(recurring_transaction::Column::StartDate.lte(end_date))
        .all(db)
        .await?;

    debug!(
        "Found {} recurring transaction definitions for account_id={}",
        transactions.len(),
        account_id
    );

    let mut result = Vec::new();

    for tx in &transactions {
        trace!(
            "Processing recurring transaction: id={}, description={:?}, amount={}, period={:?}",
            tx.id, tx.description, tx.amount, tx.period
        );

        // Get all one-off transactions that are reconciled with this recurring transaction
        // Note: We don't filter by date range here because we need to know about all reconciled transactions,
        // even those outside the date range, to correctly handle unreconciled transactions
        let reconciled_transactions = one_off_transaction::Entity::find()
            .filter(one_off_transaction::Column::ReconciledRecurringTransactionId.eq(tx.id))
            .all(db)
            .await?;

        debug!(
            "Found {} reconciled one-off transactions for recurring transaction id={}",
            reconciled_transactions.len(),
            tx.id
        );

        // Generate all occurrences of this recurring transaction
        // We need to include occurrences from the start date of the recurring transaction
        // to ensure we catch any past transactions that might be moved to today
        let effective_start_date = std::cmp::min(tx.start_date, start_date);
        let occurrences =
            generate_occurrences(tx.start_date, tx.end_date, &tx.period, effective_start_date, end_date);

        debug!(
            "Generated {} occurrences for recurring transaction id={}",
            occurrences.len(),
            tx.id
        );

        // Group reconciled transactions by month to handle the case where a recurring transaction
        // might be reconciled with multiple one-off transactions in the same month
        let mut reconciled_months = std::collections::HashSet::new();
        for reconciled_tx in &reconciled_transactions {
            let month_key = (reconciled_tx.date.year(), reconciled_tx.date.month());
            reconciled_months.insert(month_key);

            // Add the reconciled transaction to the result
            // For the forecast model, we need to include these transactions
            // as they are not added separately in the compute_forecast function
            trace!(
                "Adding reconciled transaction on {} for recurring transaction id={}",
                reconciled_tx.date, tx.id
            );
            result.push((reconciled_tx.date, tx.clone()));
        }

        // Add occurrences that haven't been reconciled
        for date in occurrences {
            let month_key = (date.year(), date.month());
            if !reconciled_months.contains(&month_key) {
                // For past dates without a linked one-off transaction, move them forward in time
                // as they are considered "not paid yet"
                println!(
                    "DEBUG: Comparing date {} with today {} for recurring transaction id={}",
                    date, today, tx.id
                );
                let actual_date = if date <= today {
                    println!(
                        "DEBUG: Moving past occurrence on {} for recurring transaction id={} forward to today ({})",
                        date, tx.id, today
                    );
                    today
                } else {
                    println!(
                        "DEBUG: Keeping occurrence on {} for recurring transaction id={} as is",
                        date, tx.id
                    );
                    date
                };

                // Only add the occurrence if it's not on the same date as a reconciled transaction
                // This prevents double counting when a recurring transaction is reconciled
                if !reconciled_transactions.iter().any(|rt| rt.date == actual_date) {
                    trace!(
                        "Adding occurrence on {} for recurring transaction id={}",
                        actual_date, tx.id
                    );
                    result.push((actual_date, tx.clone()));
                } else {
                    trace!(
                        "Skipping occurrence on {} for recurring transaction id={} as there's already a reconciled transaction on this date",
                        actual_date, tx.id
                    );
                }
            } else {
                trace!(
                    "Skipping occurrence on {} for recurring transaction id={} as it's already reconciled",
                    date, tx.id
                );
            }
        }
    }

    debug!(
        "Returning {} total recurring transaction occurrences for account_id={}",
        result.len(),
        account_id
    );
    Ok(result)
}

/// Gets all recurring income for the account within the given date range.
/// Returns a vector of (date, income) pairs for all occurrences within the range.
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date
))]
pub async fn get_recurring_income(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<(NaiveDate, recurring_income::Model)>> {
    trace!(
        "Getting recurring income for account_id={} from {} to {}",
        account_id, start_date, end_date
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
        trace!(
            "Processing recurring income: id={}, description={:?}, amount={}, period={:?}",
            income.id, income.description, income.amount, income.period
        );

        let occurrences = generate_occurrences(
            income.start_date,
            income.end_date,
            &income.period,
            start_date,
            end_date,
        );

        debug!(
            "Generated {} occurrences for recurring income id={}",
            occurrences.len(),
            income.id
        );

        for date in occurrences {
            trace!(
                "Adding occurrence on {} for recurring income id={}",
                date, income.id
            );
            result.push((date, income.clone()));
        }
    }

    debug!(
        "Returning {} total recurring income occurrences for account_id={}",
        result.len(),
        account_id
    );
    Ok(result)
}

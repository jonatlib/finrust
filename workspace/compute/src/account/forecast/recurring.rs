use chrono::{Duration, NaiveDate};
use model::entities::{recurring_income, recurring_transaction, recurring_transaction_instance};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait,
};
use tracing::{debug, instrument, trace};

use crate::account::utils::generate_occurrences;
use crate::error::Result;

/// Gets all recurring transactions for the account within the given date range.
/// Returns a vector of (date, transaction) pairs for all occurrences within the range.
/// 
/// For forecast calculator:
/// - Future recurring transactions (date >= today) are treated as if they were accounted on their date
/// - Past recurring transactions (date < today) with instances are accounted according to those instances
/// - Past recurring transactions (date < today) without instances are moved to today + future_offset
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date, today = %today, future_offset = %future_offset.num_days()
))]
pub async fn get_recurring_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    future_offset: Duration,
) -> Result<Vec<(NaiveDate, recurring_transaction::Model)>> {
    trace!(
        "Getting recurring transactions for account_id={} from {} to {} (today={}, future_offset={}d)",
        account_id, start_date, end_date, today, future_offset.num_days()
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

        // Get instances for this recurring transaction
        let instances = recurring_transaction_instance::Entity::find()
            .filter(recurring_transaction_instance::Column::RecurringTransactionId.eq(tx.id))
            .all(db)
            .await?;

        debug!(
            "Found {} instances for recurring transaction id={}",
            instances.len(),
            tx.id
        );

        // Handle recurring transactions without instances
        if instances.is_empty() {
            trace!("Handling recurring transaction without instances (id={})", tx.id);

            // For past occurrences, we need to generate all occurrences from start_date to today
            if tx.start_date < today {
                // Generate all past occurrences from transaction start date to today
                let past_occurrences = generate_occurrences(tx.start_date, tx.end_date, &tx.period, tx.start_date, today);

                // Move all past occurrences to today + future_offset
                let new_date = today + future_offset;
                for date in past_occurrences {
                    if date < today {  // Only include dates before today
                        result.push((new_date, tx.clone()));
                    }
                }

                // For transactions that start before today, we need to find the next occurrence after today + future_offset
                let next_date_after_offset = new_date.succ_opt().unwrap(); // Start from the day after today + future_offset
                let future_occurrences = generate_occurrences(tx.start_date, tx.end_date, &tx.period, next_date_after_offset, end_date);

                // Add future occurrences on their original dates
                for date in future_occurrences {
                    trace!("Adding future occurrence on {} for recurring transaction id={}", date, tx.id);
                    result.push((date, tx.clone()));
                }
            } else {
                // For transactions that start on or after today, generate future occurrences from start_date to end_date
                let future_occurrences = generate_occurrences(tx.start_date, tx.end_date, &tx.period, tx.start_date, end_date);

                // Add future occurrences on their original dates
                for date in future_occurrences {
                    result.push((date, tx.clone()));
                }
            }

            // Skip the normal processing for this transaction
            continue;
        }

        let occurrences =
            generate_occurrences(tx.start_date, tx.end_date, &tx.period, start_date, end_date);

        debug!(
            "Generated {} occurrences for recurring transaction id={}",
            occurrences.len(),
            tx.id
        );

        for date in occurrences {
            if date >= today {
                // Future recurring transactions are treated as if they were accounted on their date
                trace!(
                    "Adding future occurrence on {} for recurring transaction id={}",
                    date, tx.id
                );
                result.push((date, tx.clone()));
            } else {
                // Past recurring transactions
                // Check if there's an instance for this date
                let instance = instances.iter().find(|i| i.due_date == date);

                if let Some(instance) = instance {
                    // If there's an instance, use its status to determine how to handle it
                    match instance.status {
                        recurring_transaction_instance::InstanceStatus::Paid => {
                            // If paid, use the paid date and amount if available
                            if let Some(paid_date) = instance.paid_date {
                                let amount = instance.paid_amount.unwrap_or(tx.amount);
                                trace!(
                                    "Adding paid instance on {} (paid on {}) for recurring transaction id={}",
                                    date, paid_date, tx.id
                                );
                                // Use the original transaction but with the paid amount
                                let mut paid_tx = tx.clone();
                                paid_tx.amount = amount;
                                result.push((paid_date, paid_tx));
                            } else {
                                // If no paid date, use the due date
                                trace!(
                                    "Adding paid instance on {} for recurring transaction id={}",
                                    date, tx.id
                                );
                                result.push((date, tx.clone()));
                            }
                        },
                        recurring_transaction_instance::InstanceStatus::Skipped => {
                            // If skipped, ignore it
                            trace!(
                                "Ignoring skipped instance on {} for recurring transaction id={}",
                                date, tx.id
                            );
                        },
                        recurring_transaction_instance::InstanceStatus::Pending => {
                            // If pending, keep it on its original due date
                            trace!(
                                "Adding pending instance on {} for recurring transaction id={}",
                                date, tx.id
                            );
                            result.push((date, tx.clone()));
                        },
                    }
                } else {
                    // If no instance, move it to today + future_offset
                    let new_date = today + future_offset;
                    trace!(
                        "Moving past occurrence without instance from {} to {} for recurring transaction id={}",
                        date, new_date, tx.id
                    );
                    result.push((new_date, tx.clone()));
                }
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
/// 
/// For forecast calculator:
/// - Future recurring income (date >= today) is treated as if it were accounted on its date
/// - Past recurring income (date < today) is moved to today + future_offset
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date, today = %today, future_offset = %future_offset.num_days()
))]
pub async fn get_recurring_income(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    future_offset: Duration,
) -> Result<Vec<(NaiveDate, recurring_income::Model)>> {
    trace!(
        "Getting recurring income for account_id={} from {} to {} (today={}, future_offset={}d)",
        account_id, start_date, end_date, today, future_offset.num_days()
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
            if date >= today {
                // Future recurring income is treated as if it were accounted on its date
                trace!(
                    "Adding future occurrence on {} for recurring income id={}",
                    date, income.id
                );
                result.push((date, income.clone()));
            } else {
                // Past recurring income is moved to today + future_offset
                let new_date = today + future_offset;
                trace!(
                    "Moving past occurrence from {} to {} for recurring income id={}",
                    date, new_date, income.id
                );
                result.push((new_date, income.clone()));
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

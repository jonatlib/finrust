use chrono::NaiveDate;
use model::entities::{recurring_income, recurring_transaction};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use tracing::{debug, instrument, trace};

use crate::account::utils::generate_occurrences;
use crate::error::Result;

/// Gets all recurring transactions for the account within the given date range.
/// Returns a vector of (date, transaction) pairs for all occurrences within the range.
/// 
/// For balance calculator:
/// - Future recurring transactions (date >= today) are treated as if they were accounted on their date
/// - Past recurring transactions (date < today) with instances are included on their due date
/// - Past recurring transactions (date < today) without instances are ignored
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
        "Getting recurring transactions for account_id={} from {} to {} (today={})",
        account_id, start_date, end_date, today
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
        println!(
            "Processing recurring transaction: id={}, name={}, description={:?}, amount={}, period={:?}, start_date={}",
            tx.id, tx.name, tx.description, tx.amount, tx.period, tx.start_date
        );
        trace!(
            "Processing recurring transaction: id={}, description={:?}, amount={}, period={:?}",
            tx.id, tx.description, tx.amount, tx.period
        );

        // Get instances for this recurring transaction
        let instances = model::entities::recurring_transaction_instance::Entity::find()
            .filter(model::entities::recurring_transaction_instance::Column::RecurringTransactionId.eq(tx.id))
            .all(db)
            .await?;

        println!(
            "Found {} instances for recurring transaction id={}",
            instances.len(),
            tx.id
        );
        debug!(
            "Found {} instances for recurring transaction id={}",
            instances.len(),
            tx.id
        );

        let occurrences =
            generate_occurrences(tx.start_date, tx.end_date, &tx.period, start_date, end_date);

        println!(
            "Generated {} occurrences for recurring transaction id={}: {:?}",
            occurrences.len(),
            tx.id,
            occurrences
        );
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

                if let Some(_instance) = instance {
                    // If there's an instance, include it on its due date
                    trace!(
                        "Adding past occurrence with instance on {} for recurring transaction id={}",
                        date, tx.id
                    );
                    result.push((date, tx.clone()));
                } else {
                    // If no instance, ignore it
                    trace!(
                        "Ignoring past occurrence without instance on {} for recurring transaction id={}",
                        date, tx.id
                    );
                    // Do not add to result
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
/// For balance calculator:
/// - Future recurring income (date >= today) is treated as if it were accounted on its date
/// - Past recurring income (date < today) with instances are included on their due date
/// - Past recurring income (date < today) without instances are ignored
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date, today = %today
))]
pub async fn get_recurring_income(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
) -> Result<Vec<(NaiveDate, recurring_income::Model)>> {
    trace!(
        "Getting recurring income for account_id={} from {} to {} (today={})",
        account_id, start_date, end_date, today
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

        // Get instances for this recurring income
        let instances = model::entities::recurring_transaction_instance::Entity::find()
            .filter(model::entities::recurring_transaction_instance::Column::RecurringTransactionId.eq(income.id))
            .all(db)
            .await?;

        debug!(
            "Found {} instances for recurring income id={}",
            instances.len(),
            income.id
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
                // Past recurring income
                // Check if there's an instance for this date
                let instance = instances.iter().find(|i| i.due_date == date);

                if let Some(_instance) = instance {
                    // If there's an instance, include it on its due date
                    trace!(
                        "Adding past occurrence with instance on {} for recurring income id={}",
                        date, income.id
                    );
                    result.push((date, income.clone()));
                } else {
                    // If no instance, ignore it
                    trace!(
                        "Ignoring past occurrence without instance on {} for recurring income id={}",
                        date, income.id
                    );
                    // Do not add to result
                }
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

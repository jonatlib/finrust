use chrono::NaiveDate;
use model::entities::{recurring_transaction, recurring_transaction_instance};
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{debug, instrument, trace};

use crate::account::utils::generate_occurrences;
use crate::error::Result;

use super::common::process_occurrences;

/// Gets all recurring transactions for the account within the given date range.
/// Returns a vector of (date, transaction) pairs for all occurrences within the range.
///
/// For balance calculator:
/// - Future recurring transactions (date >= today) are treated as if they were accounted on their date
/// - Past recurring transactions (date < today) with instances are included on their due date
/// - Past recurring transactions (date < today) without instances are ignored
///
/// # Scenario Context
/// - `None`: Fetch only real transactions (is_simulated = false)
/// - `Some(id)`: Fetch real transactions OR simulated transactions belonging to the scenario
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date, today = %today, scenario_context = ?scenario_context
))]
pub async fn get_recurring_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    scenario_context: Option<i32>,
) -> Result<Vec<(NaiveDate, recurring_transaction::Model)>> {
    trace!(
        "Getting recurring transactions for account_id={} from {} to {} (today={}, scenario_context={:?})",
        account_id, start_date, end_date, today, scenario_context
    );

    // Fetch recurring transaction definitions
    let transactions = fetch_recurring_transactions(db, account_id, start_date, end_date, scenario_context).await?;

    debug!(
        "Found {} recurring transaction definitions for account_id={}",
        transactions.len(),
        account_id
    );

    let mut result = Vec::new();

    // Process each transaction
    for tx in &transactions {
        debug!(
            "Processing recurring transaction: id={}, name={}, description={:?}, amount={}, period={:?}, start_date={}",
            tx.id, tx.name, tx.description, tx.amount, tx.period, tx.start_date
        );

        // Get instances and process occurrences
        let instances = fetch_transaction_instances(db, tx.id).await?;
        let valid_dates =
            process_transaction_occurrences(tx, &instances, start_date, end_date, today);

        // Add valid occurrences to result
        for date in valid_dates {
            result.push((date, tx.clone()));
        }
    }

    debug!(
        "Returning {} total recurring transaction occurrences for account_id={}",
        result.len(),
        account_id
    );
    Ok(result)
}

/// Fetches recurring transaction definitions from the database
async fn fetch_recurring_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    scenario_context: Option<i32>,
) -> Result<Vec<recurring_transaction::Model>> {
    let mut query = recurring_transaction::Entity::find()
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
        .filter(recurring_transaction::Column::StartDate.lte(end_date));

    // Apply scenario filtering
    query = match scenario_context {
        None => {
            // Standard mode: only real transactions
            query.filter(recurring_transaction::Column::IsSimulated.eq(false))
        }
        Some(scenario_id) => {
            // Scenario mode: real OR (simulated AND belongs to this scenario)
            query.filter(
                Condition::any()
                    .add(recurring_transaction::Column::IsSimulated.eq(false))
                    .add(
                        Condition::all()
                            .add(recurring_transaction::Column::IsSimulated.eq(true))
                            .add(recurring_transaction::Column::ScenarioId.eq(scenario_id)),
                    ),
            )
        }
    };

    let transactions = query.all(db).await?;

    Ok(transactions)
}

/// Fetches instances for a recurring transaction
/// Only fetches Paid instances - Pending and Skipped instances are not counted in balance
async fn fetch_transaction_instances(
    db: &DatabaseConnection,
    transaction_id: i32,
) -> Result<Vec<recurring_transaction_instance::Model>> {
    let instances = recurring_transaction_instance::Entity::find()
        .filter(recurring_transaction_instance::Column::RecurringTransactionId.eq(transaction_id))
        .filter(recurring_transaction_instance::Column::Status.eq(recurring_transaction_instance::InstanceStatus::Paid))
        .all(db)
        .await?;

    debug!(
        "Found {} paid instances for recurring transaction id={}",
        instances.len(),
        transaction_id
    );

    Ok(instances)
}

/// Processes occurrences for a recurring transaction
fn process_transaction_occurrences(
    tx: &recurring_transaction::Model,
    instances: &[recurring_transaction_instance::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
) -> Vec<NaiveDate> {
    let occurrences =
        generate_occurrences(tx.start_date, tx.end_date, &tx.period, start_date, end_date);

    debug!(
        "Generated {} occurrences for recurring transaction id={}",
        occurrences.len(),
        tx.id
    );
    trace!(
        "Occurrences for recurring transaction id={}: {:?}",
        tx.id, occurrences
    );

    // Process occurrences using the common function
    process_occurrences(
        occurrences,
        instances,
        today,
        tx.id,
        |instance| instance.due_date,
        |instance| instance.paid_date,
    )
}

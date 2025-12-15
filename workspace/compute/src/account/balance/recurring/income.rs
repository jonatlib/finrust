use chrono::NaiveDate;
use model::entities::{recurring_income, recurring_transaction_instance};
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{debug, instrument, trace};

use crate::account::utils::generate_occurrences;
use crate::error::Result;

use super::common::process_occurrences;

/// Gets all recurring income for the account within the given date range.
/// Returns a vector of (date, income) pairs for all occurrences within the range.
///
/// For balance calculator:
/// - Future recurring income (date >= today) is treated as if it were accounted on its date
/// - Past recurring income (date < today) with instances are included on their due date
/// - Past recurring income (date < today) without instances are ignored
///
/// # Scenario Context
/// - `None`: Fetch only real income (is_simulated = false)
/// - `Some(id)`: Fetch real income OR simulated income belonging to the scenario
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date, today = %today, scenario_context = ?scenario_context
))]
pub async fn get_recurring_income(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    scenario_context: Option<i32>,
) -> Result<Vec<(NaiveDate, recurring_income::Model)>> {
    trace!(
        "Getting recurring income for account_id={} from {} to {} (today={}, scenario_context={:?})",
        account_id, start_date, end_date, today, scenario_context
    );

    // Fetch recurring income definitions
    let incomes = fetch_recurring_income(db, account_id, start_date, end_date, scenario_context).await?;

    debug!(
        "Found {} recurring income definitions for account_id={}",
        incomes.len(),
        account_id
    );

    let mut result = Vec::new();

    // Process each income
    for income in &incomes {
        trace!(
            "Processing recurring income: id={}, description={:?}, amount={}, period={:?}",
            income.id, income.description, income.amount, income.period
        );

        // Get instances and process occurrences
        let instances = fetch_income_instances(db, income.id).await?;
        let valid_dates =
            process_income_occurrences(income, &instances, start_date, end_date, today);

        // Add valid occurrences to result
        for date in valid_dates {
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

/// Fetches recurring income definitions from the database
async fn fetch_recurring_income(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    scenario_context: Option<i32>,
) -> Result<Vec<recurring_income::Model>> {
    let mut query = recurring_income::Entity::find()
        .filter(recurring_income::Column::TargetAccountId.eq(account_id))
        .filter(
            Condition::any()
                .add(recurring_income::Column::EndDate.is_null())
                .add(recurring_income::Column::EndDate.gte(start_date)),
        )
        .filter(recurring_income::Column::StartDate.lte(end_date));

    // Apply scenario filtering
    query = match scenario_context {
        None => {
            // Standard mode: only real income
            query.filter(recurring_income::Column::IsSimulated.eq(false))
        }
        Some(scenario_id) => {
            // Scenario mode: real OR (simulated AND belongs to this scenario)
            query.filter(
                Condition::any()
                    .add(recurring_income::Column::IsSimulated.eq(false))
                    .add(
                        Condition::all()
                            .add(recurring_income::Column::IsSimulated.eq(true))
                            .add(recurring_income::Column::ScenarioId.eq(scenario_id)),
                    ),
            )
        }
    };

    let incomes = query.all(db).await?;

    Ok(incomes)
}

/// Fetches instances for a recurring income
/// Only fetches Paid instances - Pending and Skipped instances are not counted in balance
async fn fetch_income_instances(
    db: &DatabaseConnection,
    income_id: i32,
) -> Result<Vec<recurring_transaction_instance::Model>> {
    let instances = recurring_transaction_instance::Entity::find()
        .filter(recurring_transaction_instance::Column::RecurringTransactionId.eq(income_id))
        .filter(recurring_transaction_instance::Column::Status.eq(recurring_transaction_instance::InstanceStatus::Paid))
        .all(db)
        .await?;

    debug!(
        "Found {} paid instances for recurring income id={}",
        instances.len(),
        income_id
    );

    Ok(instances)
}

/// Processes occurrences for a recurring income
fn process_income_occurrences(
    income: &recurring_income::Model,
    instances: &[recurring_transaction_instance::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
) -> Vec<NaiveDate> {
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

    // Process occurrences using the common function
    process_occurrences(
        occurrences,
        instances,
        today,
        income.id,
        |instance| instance.due_date,
        |instance| instance.paid_date,
    )
}

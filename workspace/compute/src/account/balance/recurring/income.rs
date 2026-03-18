use std::collections::HashMap;

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

    let incomes = fetch_recurring_income(db, account_id, start_date, end_date, scenario_context).await?;

    debug!(
        "Found {} recurring income definitions for account_id={}",
        incomes.len(),
        account_id
    );

    if incomes.is_empty() {
        return Ok(vec![]);
    }

    // Batch-fetch all instances in one query instead of N+1
    let income_ids: Vec<i32> = incomes.iter().map(|i| i.id).collect();
    let instances_map = fetch_income_instances_batch(db, &income_ids).await?;

    let mut result = Vec::new();

    for income in &incomes {
        trace!(
            "Processing recurring income: id={}, description={:?}, amount={}, period={:?}",
            income.id, income.description, income.amount, income.period
        );

        let instances = instances_map.get(&income.id).map(|v| v.as_slice()).unwrap_or(&[]);
        let valid_dates =
            process_income_occurrences(income, instances, start_date, end_date, today);

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

/// Batch-fetches paid instances for multiple recurring income items in one query.
async fn fetch_income_instances_batch(
    db: &DatabaseConnection,
    income_ids: &[i32],
) -> Result<HashMap<i32, Vec<recurring_transaction_instance::Model>>> {
    if income_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let instances = recurring_transaction_instance::Entity::find()
        .filter(recurring_transaction_instance::Column::RecurringTransactionId.is_in(income_ids.to_vec()))
        .filter(recurring_transaction_instance::Column::Status.eq(recurring_transaction_instance::InstanceStatus::Paid))
        .all(db)
        .await?;

    debug!(
        "Batch-fetched {} paid instances for {} recurring incomes",
        instances.len(),
        income_ids.len()
    );

    let mut map: HashMap<i32, Vec<recurring_transaction_instance::Model>> = HashMap::new();
    for inst in instances {
        map.entry(inst.recurring_transaction_id).or_default().push(inst);
    }
    Ok(map)
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

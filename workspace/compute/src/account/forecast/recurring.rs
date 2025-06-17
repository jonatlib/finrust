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
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date
))]
pub async fn get_recurring_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<(NaiveDate, recurring_transaction::Model)>> {
    trace!("Getting recurring transactions for account_id={} from {} to {}", account_id, start_date, end_date);

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

    debug!("Found {} recurring transaction definitions for account_id={}", transactions.len(), account_id);

    let mut result = Vec::new();

    for tx in &transactions {
        trace!("Processing recurring transaction: id={}, description={:?}, amount={}, period={:?}", 
               tx.id, tx.description, tx.amount, tx.period);

        let occurrences = generate_occurrences(
            tx.start_date,
            tx.end_date,
            &tx.period,
            start_date,
            end_date,
        );

        debug!("Generated {} occurrences for recurring transaction id={}", occurrences.len(), tx.id);

        for date in occurrences {
            trace!("Adding occurrence on {} for recurring transaction id={}", date, tx.id);
            result.push((date, tx.clone()));
        }
    }

    debug!("Returning {} total recurring transaction occurrences for account_id={}", result.len(), account_id);
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
    trace!("Getting recurring income for account_id={} from {} to {}", account_id, start_date, end_date);

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

    debug!("Found {} recurring income definitions for account_id={}", incomes.len(), account_id);

    let mut result = Vec::new();

    for income in &incomes {
        trace!("Processing recurring income: id={}, description={:?}, amount={}, period={:?}", 
               income.id, income.description, income.amount, income.period);

        let occurrences = generate_occurrences(
            income.start_date,
            income.end_date,
            &income.period,
            start_date,
            end_date,
        );

        debug!("Generated {} occurrences for recurring income id={}", occurrences.len(), income.id);

        for date in occurrences {
            trace!("Adding occurrence on {} for recurring income id={}", date, income.id);
            result.push((date, income.clone()));
        }
    }

    debug!("Returning {} total recurring income occurrences for account_id={}", result.len(), account_id);
    Ok(result)
}

use chrono::NaiveDate;
use model::entities::{imported_transaction, one_off_transaction};
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{debug, instrument, trace};

use crate::error::Result;

/// Gets all one-off transactions for the account within the given date range.
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date
))]
pub async fn get_transactions_for_account(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<one_off_transaction::Model>> {
    trace!(
        "Getting one-off transactions for account_id={} from {} to {}",
        account_id, start_date, end_date
    );

    let transactions = one_off_transaction::Entity::find()
        .filter(
            Condition::any()
                .add(one_off_transaction::Column::TargetAccountId.eq(account_id))
                .add(one_off_transaction::Column::SourceAccountId.eq(account_id)),
        )
        .filter(
            Condition::all()
                .add(one_off_transaction::Column::Date.gte(start_date))
                .add(one_off_transaction::Column::Date.lte(end_date)),
        )
        .all(db)
        .await?;

    debug!(
        "Found {} one-off transactions for account_id={} from {} to {}",
        transactions.len(),
        account_id,
        start_date,
        end_date
    );

    for tx in &transactions {
        trace!(
            "One-off transaction: id={}, date={}, description={:?}, amount={}",
            tx.id, tx.date, tx.description, tx.amount
        );
    }

    Ok(transactions)
}

/// Gets all imported transactions for the account within the given date range.
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date
))]
pub async fn get_imported_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<imported_transaction::Model>> {
    trace!(
        "Getting imported transactions for account_id={} from {} to {}",
        account_id, start_date, end_date
    );

    let transactions = imported_transaction::Entity::find()
        .filter(
            Condition::all()
                .add(imported_transaction::Column::AccountId.eq(account_id))
                .add(imported_transaction::Column::Date.gte(start_date))
                .add(imported_transaction::Column::Date.lte(end_date)),
        )
        .all(db)
        .await?;

    debug!(
        "Found {} imported transactions for account_id={} from {} to {}",
        transactions.len(),
        account_id,
        start_date,
        end_date
    );

    for tx in &transactions {
        trace!(
            "Imported transaction: id={}, date={}, description={:?}, amount={}, reconciled={:?}",
            tx.id, tx.date, tx.description, tx.amount, tx.reconciled_transaction_id
        );
    }

    Ok(transactions)
}

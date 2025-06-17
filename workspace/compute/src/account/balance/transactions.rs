use chrono::NaiveDate;
use model::entities::{imported_transaction, one_off_transaction};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::error::Result;

/// Gets all one-off transactions for the account within the given date range.
pub async fn get_transactions_for_account(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<one_off_transaction::Model>> {
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

    Ok(transactions)
}

/// Gets all imported transactions for the account within the given date range.
pub async fn get_imported_transactions(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<imported_transaction::Model>> {
    let transactions = imported_transaction::Entity::find()
        .filter(
            Condition::all()
                .add(imported_transaction::Column::AccountId.eq(account_id))
                .add(imported_transaction::Column::Date.gte(start_date))
                .add(imported_transaction::Column::Date.lte(end_date)),
        )
        .all(db)
        .await?;

    Ok(transactions)
}

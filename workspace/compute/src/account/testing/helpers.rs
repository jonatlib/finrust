use std::sync::atomic::AtomicU64;

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, Set};

use model::entities::{
    account, manual_account_state, one_off_transaction, recurring_transaction,
    recurring_transaction_instance,
};

pub type Result<T> = std::result::Result<T, DbErr>;

pub async fn new_account(db: &DatabaseConnection) -> Result<account::Model> {
    static ACCOUNT_ID: AtomicU64 = AtomicU64::new(0);

    let current_id = ACCOUNT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    // Create a test user first
    let user = model::entities::user::ActiveModel {
        username: Set(format!("user_{}", current_id)),
        ..Default::default()
    }
    .insert(db)
    .await?;

    // Create a test account
    let account = account::ActiveModel {
        name: Set(format!("Test account {}", current_id)),
        description: Set(Some("Account for balance testing".to_string())),
        currency_code: Set("USD".to_string()),
        owner_id: Set(user.id),
        include_in_statistics: Set(true),
        ledger_name: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(account)
}

pub async fn new_manual_account_state(
    db: &DatabaseConnection,
    account: &account::Model,
    date: NaiveDate,
    amount: i64,
) -> Result<manual_account_state::Model> {
    manual_account_state::ActiveModel {
        account_id: Set(account.id),
        date: Set(date),
        amount: Set(Decimal::new(amount * 100, 2)),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn new_recurring_transaction(
    db: &DatabaseConnection,
    account: &account::Model,
    date: NaiveDate,
    amount: i64,
) -> Result<recurring_transaction::Model> {
    recurring_transaction::ActiveModel {
        name: Set("Monthly".to_string()),
        description: Set(Some("Monthly payment".to_string())),
        amount: Set(Decimal::new(amount * 100, 2)), // -$500.00
        start_date: Set(date),
        end_date: Set(None), // Indefinite
        period: Set(recurring_transaction::RecurrencePeriod::Monthly),
        include_in_statistics: Set(true),
        target_account_id: Set(account.id),
        source_account_id: Set(None),
        ledger_name: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn new_recurring_instance(
    db: &DatabaseConnection,
    transaction: &recurring_transaction::Model,
    date: NaiveDate,
) -> Result<recurring_transaction_instance::Model> {
    recurring_transaction_instance::ActiveModel {
        recurring_transaction_id: Set(transaction.id),
        status: Set(recurring_transaction_instance::InstanceStatus::Paid),
        due_date: Set(transaction.start_date.with_month(date.month()).unwrap()),
        expected_amount: Set(transaction.amount),
        paid_date: Set(Some(date)),
        paid_amount: Set(Some(transaction.amount)),
        reconciled_imported_transaction_id: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn new_one_off_trsansaction(
    db: &DatabaseConnection,
    account: &account::Model,
    date: NaiveDate,
    amount: i64,
) -> Result<one_off_transaction::Model> {
    one_off_transaction::ActiveModel {
        name: Set("One-off".to_string()),
        description: Set(Some("One-off".to_string())),
        amount: Set(Decimal::new(amount * 100, 2)),
        date: Set(date),
        include_in_statistics: Set(true),
        target_account_id: Set(account.id),
        source_account_id: Set(None),
        ledger_name: Set(None),
        linked_import_id: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn new_one_off_account_transfer(
    db: &DatabaseConnection,
    source_account: &account::Model,
    target_account: &account::Model,
    date: NaiveDate,
    amount: i64,
) -> Result<one_off_transaction::Model> {
    one_off_transaction::ActiveModel {
        name: Set("One-off transfer".to_string()),
        description: Set(Some("One-off transfer".to_string())),
        amount: Set(Decimal::new(amount * 100, 2)),
        date: Set(date),
        include_in_statistics: Set(true),
        target_account_id: Set(target_account.id),
        source_account_id: Set(Some(source_account.id)),
        ledger_name: Set(None),
        linked_import_id: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
}

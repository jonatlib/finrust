use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use super::{account, one_off_transaction};

/// Represents a transaction imported from a bank file (e.g., CSV, OFX).
/// This stores the raw data before it is reconciled and mapped to an internal transaction.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "imported_transactions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The account this transaction was imported for.
    pub account_id: i32,
    /// The date of the transaction as stated in the import file.
    pub date: NaiveDate,
    /// A description or name of the transaction from the import file.
    pub description: String,
    /// The transaction amount.
    #[sea_orm(column_type = "Decimal(Some((16, 4)))")]
    pub amount: Decimal,
    /// A unique hash or identifier of the raw imported row to prevent duplicate imports.
    #[sea_orm(unique)]
    pub import_hash: String,
    /// Stores the entire raw transaction data as JSON for auditing and debugging.
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub raw_data: Option<Json>,
    /// Link to the internal transaction created from this import.
    /// This is nullable because an imported transaction may not be reconciled immediately.
    pub reconciled_one_off_transaction_id: Option<i32>,
    // We could also add a link to a recurring transaction instance if we can map it.
    // pub reconciled_recurring_transaction_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "account::Entity",
        from = "Column::AccountId",
        to = "account::Column::Id",
        on_delete = "Cascade"
    )]
    Account,
    /// An optional link to the reconciled OneOffTransaction.
    #[sea_orm(
        belongs_to = "one_off_transaction::Entity",
        from = "Column::ReconciledOneOffTransactionId",
        to = "one_off_transaction::Column::Id",
        on_delete = "SetNull"
    )]
    ReconciledTransaction,
}

impl ActiveModelBehavior for ActiveModel {}
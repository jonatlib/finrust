use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use super::{account, tag};

/// A single, non-repeating transaction.
/// Corresponds to `ExtraTransactionModel`.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "one_off_transactions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    /// The value of the transaction. Positive for income, negative for expense.
    #[sea_orm(column_type = "Decimal(Some((16, 4)))")]
    pub amount: Decimal,
    /// The exact date of the transaction.
    pub date: NaiveDate,
    #[sea_orm(default_value = "true")]
    pub include_in_statistics: bool,
    /// The primary account affected by this transaction.
    pub target_account_id: i32,
    /// The optional source account for a transfer (double-entry).
    /// If set, this transaction represents a movement of funds.
    pub source_account_id: Option<i32>,
    /// The name to use when exporting to Ledger CLI format.
    pub ledger_name: Option<String>,
    // An optional field to link to an imported transaction to prevent duplication.
    pub linked_import_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "account::Entity",
        from = "Column::TargetAccountId",
        to = "account::Column::Id",
        on_delete = "Cascade"
    )]
    TargetAccount,
    #[sea_orm(
        belongs_to = "account::Entity",
        from = "Column::SourceAccountId",
        to = "account::Column::Id",
        on_delete = "SetNull"
    )]
    SourceAccount,
}

impl Related<tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::one_off_transaction_tag::Relation::Tag.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::one_off_transaction_tag::Relation::Transaction.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

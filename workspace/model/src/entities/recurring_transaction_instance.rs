use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use super::{imported_transaction, recurring_transaction};

/// Represents the status of a single recurring transaction instance.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(15))")]
pub enum InstanceStatus {
    #[sea_orm(string_value = "Pending")]
    Pending, // The transaction is expected but not yet paid.
    #[sea_orm(string_value = "Paid")]
    Paid, // The transaction has been paid and reconciled.
    #[sea_orm(string_value = "Skipped")]
    Skipped, // The user has marked this instance as skipped for this period.
}

/// Represents a single, concrete instance generated from a RecurringTransaction rule.
/// This entity is the 'checklist' item for a specific period (e.g., "June Rent").
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "recurring_transaction_instances")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// The rule that generated this instance.
    pub recurring_transaction_id: i32,

    /// The current status of this instance.
    pub status: InstanceStatus,

    /// The date this instance was scheduled to occur.
    pub due_date: NaiveDate,

    /// The amount expected to be paid, inherited from the rule.
    #[sea_orm(column_type = "Decimal(Some((16, 4)))")]
    pub expected_amount: Decimal,

    /// The date the instance was actually paid (nullable).
    pub paid_date: Option<NaiveDate>,

    /// The actual amount that was paid (nullable).
    #[sea_orm(column_type = "Decimal(Some((16, 4)))")]
    pub paid_amount: Option<Decimal>,

    /// A link to the bank-imported transaction that fulfilled this instance.
    pub reconciled_imported_transaction_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Each instance belongs to one recurring transaction rule.
    #[sea_orm(
        belongs_to = "recurring_transaction::Entity",
        from = "Column::RecurringTransactionId",
        to = "recurring_transaction::Column::Id",
        on_delete = "Cascade"
    )]
    RecurringTransaction,

    /// An instance can be reconciled with one imported transaction.
    #[sea_orm(
        belongs_to = "imported_transaction::Entity",
        from = "Column::ReconciledImportedTransactionId",
        to = "imported_transaction::Column::Id",
        on_delete = "SetNull"
    )]
    ImportedTransaction,
}

impl ActiveModelBehavior for ActiveModel {}

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use super::{account, tag};


/// Enum for recurrence periods.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(1))")]
pub enum RecurrencePeriod {
    #[sea_orm(string_value = "Daily")]
    Daily,
    #[sea_orm(string_value = "Weekly")]
    Weekly,
    #[sea_orm(string_value = "WorkDay")]
    WorkDay, // Monday-Friday
    #[sea_orm(string_value = "Monthly")]
    Monthly,
    #[sea_orm(string_value = "Quarterly")]
    Quarterly,
    #[sea_orm(string_value = "HalfYearly")]
    HalfYearly,
    #[sea_orm(string_value = "Yearly")]
    Yearly,
}


/// A transaction that repeats on a regular schedule.
/// Can be used for both income (salary) and expenses (rent, subscriptions).
/// Corresponds to `RegularTransactionModel`.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "recurring_transactions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    /// The value of each occurrence. Positive for income, negative for expense.
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub amount: Decimal,
    /// The date of the first occurrence.
    pub start_date: NaiveDate,
    /// The date of the last occurrence. If null, it repeats indefinitely.
    pub end_date: Option<NaiveDate>,
    /// The frequency of the transaction.
    pub period: RecurrencePeriod,
    #[sea_orm(default_value = "true")]
    pub include_in_statistics: bool,
    /// The primary account affected by this transaction.
    pub target_account_id: i32,
    /// The optional source account for a transfer (double-entry).
    pub source_account_id: Option<i32>,
    /// The name to use when exporting to Ledger CLI format.
    pub ledger_name: Option<String>,
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
        super::recurring_transaction_tag::Relation::Tag.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::recurring_transaction_tag::Relation::Transaction.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

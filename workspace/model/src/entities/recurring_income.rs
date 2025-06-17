use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use super::{account, recurring_transaction::RecurrencePeriod, tag};

/// Models a recurring income stream, like a salary or business revenue.
/// Structurally similar to RecurringTransaction but semantically distinct.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "recurring_incomes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    /// The value of each occurrence. Expected to be positive (income).
    #[sea_orm(column_type = "Decimal(Some((16, 4)))")]
    pub amount: Decimal,
    /// The date of the first occurrence.
    pub start_date: NaiveDate,
    /// The date of the last occurrence. If null, it repeats indefinitely.
    pub end_date: Option<NaiveDate>,
    /// The frequency of the income.
    pub period: RecurrencePeriod,
    #[sea_orm(default_value = "true")]
    pub include_in_statistics: bool,
    /// The account where the income is deposited.
    pub target_account_id: i32,
    /// Optional source, e.g., "Company XYZ". Could be a simple string or a more complex entity later.
    /// For now, a simple text field is sufficient.
    pub source_name: Option<String>,
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
}

impl Related<tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::recurring_income_tag::Relation::Tag.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::recurring_income_tag::Relation::Income.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

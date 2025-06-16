use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveModelBehavior;

use super::account;


/// Represents a manually set balance for an account at a specific point in time.
/// This is useful for initializing an account or correcting drift over time.
/// Corresponds to `ManualAccountStateModel`.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "manual_account_states")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The account this state belongs to.
    pub account_id: i32,
    /// The date the balance is valid for.
    pub date: NaiveDate,
    /// The amount in the account on the specified date.
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub amount: Decimal,
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
}

// Implement Related trait for account::Entity
impl Related<account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

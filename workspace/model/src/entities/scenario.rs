use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;

/// A what-if scenario containing hypothetical transactions.
/// Users can create scenarios like "Buy Tesla" or "Buy Toyota" with virtual transactions.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "scenarios")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    #[sea_orm(default_value = "false")]
    pub is_active: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::one_off_transaction::Entity")]
    OneOffTransaction,
    #[sea_orm(has_many = "super::recurring_transaction::Entity")]
    RecurringTransaction,
    #[sea_orm(has_many = "super::recurring_income::Entity")]
    RecurringIncome,
}

impl Related<super::one_off_transaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OneOffTransaction.def()
    }
}

impl Related<super::recurring_transaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RecurringTransaction.def()
    }
}

impl Related<super::recurring_income::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RecurringIncome.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

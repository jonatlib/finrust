use super::{recurring_income, tag};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "recurring_incomes_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub income_id: i32,
    #[sea_orm(primary_key)]
    pub tag_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "recurring_income::Entity",
        from = "Column::IncomeId",
        to = "recurring_income::Column::Id"
    )]
    Income,
    #[sea_orm(
        belongs_to = "tag::Entity",
        from = "Column::TagId",
        to = "tag::Column::Id"
    )]
    Tag,
}
impl ActiveModelBehavior for ActiveModel {}

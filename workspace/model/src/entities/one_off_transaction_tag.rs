use super::{one_off_transaction, tag};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "one_off_transactions_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub transaction_id: i32,
    #[sea_orm(primary_key)]
    pub tag_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "one_off_transaction::Entity",
        from = "Column::TransactionId",
        to = "one_off_transaction::Column::Id"
    )]
    Transaction,
    #[sea_orm(belongs_to = "tag::Entity", from = "Column::TagId", to = "tag::Column::Id")]
    Tag,
}
impl ActiveModelBehavior for ActiveModel {}

use super::{account, tag};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub account_id: i32,
    #[sea_orm(primary_key)]
    pub tag_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "account::Entity",
        from = "Column::AccountId",
        to = "account::Column::Id"
    )]
    Account,
    #[sea_orm(belongs_to = "tag::Entity", from = "Column::TagId", to = "tag::Column::Id")]
    Tag,
}

// Implement Related trait for account::Entity
impl Related<account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

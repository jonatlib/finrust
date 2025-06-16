use super::{account, user};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts_allowed_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub account_id: i32,
    #[sea_orm(primary_key)]
    pub user_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "account::Entity",
        from = "Column::AccountId",
        to = "account::Column::Id"
    )]
    Account,
    #[sea_orm(belongs_to = "user::Entity", from = "Column::UserId", to = "user::Column::Id")]
    User,
}

// Implement Related trait for account::Entity
impl Related<account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

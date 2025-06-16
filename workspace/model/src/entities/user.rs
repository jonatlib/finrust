use sea_orm::entity::prelude::*;

/// Represents a user of the system.
/// Corresponds to Django's `User` model.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub username: String,
    // Other fields like password_hash, email, etc., would go here.
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // A user can own multiple accounts.
    #[sea_orm(has_many = "super::account::Entity")]
    Account,
}

impl ActiveModelBehavior for ActiveModel {}
 
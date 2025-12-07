use super::{tag, user};
use sea_orm::entity::prelude::*;

/// The kind of account
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum AccountKind {
    #[sea_orm(string_value = "RealAccount")]
    RealAccount,
    #[sea_orm(string_value = "Savings")]
    Savings,
    #[sea_orm(string_value = "Investment")]
    Investment,
    #[sea_orm(string_value = "Debt")]
    Debt,
    #[sea_orm(string_value = "Other")]
    Other,
}

/// Represents a financial account, like a bank account, credit card, or cash wallet.
/// Corresponds to `MoneyAccountModel`.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    /// ISO 4217 currency code, e.g., "USD", "EUR".
    /// We'll use the `rusty_money` crate in the business logic layer to handle this.
    pub currency_code: String,
    /// The user who owns this account.
    pub owner_id: i32,
    /// If true, this account is ignored in all statistics and forecasts.
    /// Useful for error-correction or temporary accounts.
    #[sea_orm(default_value = "true")]
    pub include_in_statistics: bool,
    /// The name to use when exporting to Ledger CLI format.
    pub ledger_name: Option<String>,
    /// The kind of account
    pub account_kind: AccountKind,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// An account belongs to one owner.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Relation for the many-to-many relationship with Tags.
    #[sea_orm(has_many = "super::account_tag::Entity")]
    AccountTag,
    /// Relation for the many-to-many relationship for allowed users.
    #[sea_orm(has_many = "super::account_allowed_user::Entity")]
    AccountAllowedUser,
    #[sea_orm(has_many = "super::manual_account_state::Entity")]
    ManualAccountState,
}

impl Related<tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::account_tag::Relation::Tag.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::account_tag::Relation::Account.def().rev())
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        super::account_allowed_user::Relation::User.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::account_allowed_user::Relation::Account.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

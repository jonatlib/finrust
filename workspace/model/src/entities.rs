//! This file serves as the root for all SeaORM entity modules.
//! We define the data models for the finance tracking application here.
//! The structure is inspired by the provided Django models but adapted
//! for Rust's type system and the SeaORM framework.

pub mod account;
pub mod manual_account_state;
pub mod one_off_transaction;
pub mod recurring_transaction;
pub mod tag;
pub mod user;
pub mod account_tag;
pub mod one_off_transaction_tag;
pub mod recurring_transaction_tag;
pub mod account_allowed_user;

// Define join tables for many-to-many relationships.
// SeaORM uses these to understand how to link entities.
pub mod prelude {
    //! A prelude module for easy importing of all entities.
    pub use super::account::Entity as Account;
    pub use super::account_allowed_user::Entity as AccountAllowedUser;
    pub use super::account_tag::Entity as AccountTag;
    pub use super::manual_account_state::Entity as ManualAccountState;
    pub use super::one_off_transaction::Entity as OneOffTransaction;
    pub use super::one_off_transaction_tag::Entity as OneOffTransactionTag;
    pub use super::recurring_transaction::Entity as RecurringTransaction;
    pub use super::recurring_transaction_tag::Entity as RecurringTransactionTag;
    pub use super::tag::Entity as Tag;
    pub use super::user::Entity as User;
}

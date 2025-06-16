use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use std::fmt;

use super::{account, one_off_transaction, recurring_income, recurring_transaction};

/// Enum representing the type of transaction that an imported transaction is reconciled with.
/// This enum includes the ID of the reconciled transaction directly in its variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciledTransactionType {
    OneOff(i32),
    Recurring(i32),
    RecurringIncome(i32),
}

impl fmt::Display for ReconciledTransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReconciledTransactionType::OneOff(id) => write!(f, "OneOff({})", id),
            ReconciledTransactionType::Recurring(id) => write!(f, "Recurring({})", id),
            ReconciledTransactionType::RecurringIncome(id) => write!(f, "RecurringIncome({})", id),
        }
    }
}

/// Enum for the type of reconciled transaction stored in the database.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(1))")]
pub enum ReconciledTransactionEntityType {
    #[sea_orm(string_value = "O")]
    OneOff,
    #[sea_orm(string_value = "R")]
    Recurring,
    #[sea_orm(string_value = "I")]
    RecurringIncome,
}

/// Represents a transaction imported from a bank file (e.g., CSV, OFX).
/// This stores the raw data before it is reconciled and mapped to an internal transaction.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "imported_transactions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The account this transaction was imported for.
    pub account_id: i32,
    /// The date of the transaction as stated in the import file.
    pub date: NaiveDate,
    /// A description or name of the transaction from the import file.
    pub description: String,
    /// The transaction amount.
    #[sea_orm(column_type = "Decimal(Some((16, 4)))")]
    pub amount: Decimal,
    /// A unique hash or identifier of the raw imported row to prevent duplicate imports.
    #[sea_orm(unique)]
    pub import_hash: String,
    /// Stores the entire raw transaction data as JSON for auditing and debugging.
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub raw_data: Option<Json>,

    /// The type of reconciled transaction (OneOff, Recurring, RecurringIncome).
    /// This is nullable because an imported transaction may not be reconciled immediately.
    pub reconciled_transaction_type: Option<ReconciledTransactionEntityType>,
    /// The ID of the reconciled transaction.
    /// This is nullable because an imported transaction may not be reconciled immediately.
    pub reconciled_transaction_id: Option<i32>,
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

impl Model {
    /// Get the reconciled transaction type, if any.
    pub fn get_reconciled_transaction_type(&self) -> Option<ReconciledTransactionType> {
        // Construct from the database fields
        if let (Some(entity_type), Some(id)) = (self.reconciled_transaction_type.as_ref(), self.reconciled_transaction_id) {
            match entity_type {
                ReconciledTransactionEntityType::OneOff => Some(ReconciledTransactionType::OneOff(id)),
                ReconciledTransactionEntityType::Recurring => Some(ReconciledTransactionType::Recurring(id)),
                ReconciledTransactionEntityType::RecurringIncome => Some(ReconciledTransactionType::RecurringIncome(id)),
            }
        } else {
            None
        }
    }

    /// Set the reconciled transaction type.
    pub fn set_reconciled_transaction_type(&mut self, transaction_type: Option<ReconciledTransactionType>) {
        // Reset the fields first
        self.reconciled_transaction_type = None;
        self.reconciled_transaction_id = None;

        // Set the appropriate values based on the transaction type
        if let Some(transaction_type) = transaction_type {
            match transaction_type {
                ReconciledTransactionType::OneOff(id) => {
                    self.reconciled_transaction_type = Some(ReconciledTransactionEntityType::OneOff);
                    self.reconciled_transaction_id = Some(id);
                }
                ReconciledTransactionType::Recurring(id) => {
                    self.reconciled_transaction_type = Some(ReconciledTransactionEntityType::Recurring);
                    self.reconciled_transaction_id = Some(id);
                }
                ReconciledTransactionType::RecurringIncome(id) => {
                    self.reconciled_transaction_type = Some(ReconciledTransactionEntityType::RecurringIncome);
                    self.reconciled_transaction_id = Some(id);
                }
            }
        }
    }

}

impl ActiveModelBehavior for ActiveModel {}

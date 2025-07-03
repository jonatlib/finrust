use chrono::NaiveDate;
use rust_decimal::Decimal;

/// Represents a single transaction with a specific date, amount, and account.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    date: NaiveDate,
    amount: Decimal,
    account: i32,
}

impl Transaction {
    /// Creates a new Transaction.
    pub fn new(date: NaiveDate, amount: Decimal, account: i32) -> Self {
        Self {
            date,
            amount,
            account,
        }
    }

    /// Gets the date of the transaction.
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// Gets the amount of the transaction.
    pub fn amount(&self) -> Decimal {
        self.amount
    }

    /// Gets the account ID of the transaction.
    pub fn account(&self) -> i32 {
        self.account
    }
}

/// A trait for types that can generate transactions within a date range.
pub trait TransactionGenerator {
    /// Checks if there are any transactions within the given date range.
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool;

    /// Generates transactions within the given date range.
    fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> impl Iterator<Item = Transaction>;
}

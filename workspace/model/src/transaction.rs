use chrono::NaiveDate;
use rust_decimal::Decimal;
use async_trait::async_trait;

/// Represents a tag that can be applied to transactions.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

/// Represents a single transaction with a specific date, amount, and account.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    date: NaiveDate,
    amount: Decimal,
    account: i32,
    tag: Option<Tag>,
}

impl Transaction {
    /// Creates a new Transaction.
    pub fn new(date: NaiveDate, amount: Decimal, account: i32) -> Self {
        Self {
            date,
            amount,
            account,
            tag: None,
        }
    }

    /// Creates a new Transaction with a tag.
    pub fn new_with_tag(date: NaiveDate, amount: Decimal, account: i32, tag: Tag) -> Self {
        Self {
            date,
            amount,
            account,
            tag: Some(tag),
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

    /// Gets the tag of the transaction, if any.
    pub fn tag(&self) -> Option<&Tag> {
        self.tag.as_ref()
    }

    /// Sets the tag of the transaction.
    pub fn set_tag(&mut self, tag: Option<Tag>) {
        self.tag = tag;
    }
}

/// A trait for types that can generate transactions within a date range.
#[async_trait]
pub trait TransactionGenerator {
    /// Checks if there are any transactions within the given date range.
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool;

    /// Generates transactions within the given date range.
    async fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> Vec<Transaction>;

    /// Gets a tag for a transaction, if any.
    async fn get_tag_for_transaction(&self) -> Option<Tag>;
}

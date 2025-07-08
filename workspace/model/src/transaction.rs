use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;

/// Represents a tag that can be applied to transactions.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

/// Represents a single transaction with a specific date, amount, and account.
/// 
/// The `paid_on` field indicates when the transaction was actually paid.
/// If `paid_on` is None, the transaction is not yet paid.
/// If the transaction is not paid and we have amount and date, we know when it will be paid
/// (the transaction date represents the expected payment date).
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    date: NaiveDate,
    amount: Decimal,
    account: i32,
    tags: Vec<Tag>,
    paid_on: Option<NaiveDateTime>,
}

impl Transaction {
    /// Creates a new Transaction.
    pub fn new(date: NaiveDate, amount: Decimal, account: i32) -> Self {
        Self {
            date,
            amount,
            account,
            tags: Vec::new(),
            paid_on: None,
        }
    }

    /// Creates a new Transaction with a tag.
    pub fn new_with_tag(date: NaiveDate, amount: Decimal, account: i32, tag: Tag) -> Self {
        Self {
            date,
            amount,
            account,
            tags: vec![tag],
            paid_on: None,
        }
    }

    /// Creates a new Transaction with multiple tags.
    pub fn new_with_tags(date: NaiveDate, amount: Decimal, account: i32, tags: Vec<Tag>) -> Self {
        Self {
            date,
            amount,
            account,
            tags,
            paid_on: None,
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

    /// Gets all tags of the transaction.
    pub fn tags(&self) -> &Vec<Tag> {
        &self.tags
    }

    /// Gets the first tag of the transaction, if any (for backward compatibility).
    pub fn tag(&self) -> Option<&Tag> {
        self.tags.first()
    }

    /// Sets the tags of the transaction.
    pub fn set_tags(&mut self, tags: Vec<Tag>) {
        self.tags = tags;
    }

    /// Sets a single tag of the transaction (for backward compatibility).
    pub fn set_tag(&mut self, tag: Option<Tag>) {
        match tag {
            Some(t) => self.tags = vec![t],
            None => self.tags.clear(),
        }
    }

    /// Adds a tag to the transaction.
    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.push(tag);
    }

    /// Removes all instances of a tag from the transaction.
    pub fn remove_tag(&mut self, tag_id: i32) {
        self.tags.retain(|t| t.id != tag_id);
    }

    /// Gets the paid_on datetime of the transaction.
    pub fn paid_on(&self) -> Option<NaiveDateTime> {
        self.paid_on
    }

    /// Sets the paid_on datetime of the transaction.
    pub fn set_paid_on(&mut self, paid_on: Option<NaiveDateTime>) {
        self.paid_on = paid_on;
    }

    /// Checks if the transaction is paid.
    /// Returns true if paid_on is Some, false otherwise.
    pub fn is_paid(&self) -> bool {
        self.paid_on.is_some()
    }
}

/// A trait for types that can generate transactions within a date range.
#[async_trait]
pub trait TransactionGenerator {
    /// Checks if there are any transactions within the given date range.
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool;

    /// Generates transactions within the given date range.
    /// The `today` parameter is used to determine payment status for different transaction types.
    async fn generate_transactions(&self, start: NaiveDate, end: NaiveDate, today: NaiveDate, db: &DatabaseConnection) -> Vec<Transaction>;

    /// Gets tags for a transaction.
    /// If expand is true, expands all tags to include their parent hierarchy.
    async fn get_tag_for_transaction(&self, db: &DatabaseConnection, expand: bool) -> Vec<Tag>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn test_new_transaction() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;

        let transaction = Transaction::new(date, amount, account);

        assert_eq!(transaction.date(), date);
        assert_eq!(transaction.amount(), amount);
        assert_eq!(transaction.account(), account);
        assert!(transaction.tags().is_empty());
        assert!(transaction.tag().is_none());
    }

    #[test]
    fn test_new_with_tag() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;
        let tag = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };

        let transaction = Transaction::new_with_tag(date, amount, account, tag.clone());

        assert_eq!(transaction.date(), date);
        assert_eq!(transaction.amount(), amount);
        assert_eq!(transaction.account(), account);
        assert_eq!(transaction.tags().len(), 1);
        assert_eq!(transaction.tag().unwrap(), &tag);
        assert_eq!(transaction.tags()[0], tag);
    }

    #[test]
    fn test_new_with_tags() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;
        let tag1 = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };
        let tag2 = Tag {
            id: 2,
            name: "Essential".to_string(),
            description: None,
        };
        let tags = vec![tag1.clone(), tag2.clone()];

        let transaction = Transaction::new_with_tags(date, amount, account, tags.clone());

        assert_eq!(transaction.date(), date);
        assert_eq!(transaction.amount(), amount);
        assert_eq!(transaction.account(), account);
        assert_eq!(transaction.tags().len(), 2);
        assert_eq!(transaction.tag().unwrap(), &tag1); // First tag
        assert_eq!(transaction.tags(), &tags);
    }

    #[test]
    fn test_add_tag() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;
        let mut transaction = Transaction::new(date, amount, account);

        assert!(transaction.tags().is_empty());

        let tag1 = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };
        transaction.add_tag(tag1.clone());

        assert_eq!(transaction.tags().len(), 1);
        assert_eq!(transaction.tags()[0], tag1);

        let tag2 = Tag {
            id: 2,
            name: "Essential".to_string(),
            description: None,
        };
        transaction.add_tag(tag2.clone());

        assert_eq!(transaction.tags().len(), 2);
        assert_eq!(transaction.tags()[0], tag1);
        assert_eq!(transaction.tags()[1], tag2);
    }

    #[test]
    fn test_remove_tag() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;
        let tag1 = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };
        let tag2 = Tag {
            id: 2,
            name: "Essential".to_string(),
            description: None,
        };
        let tag3 = Tag {
            id: 1, // Same ID as tag1
            name: "Duplicate".to_string(),
            description: None,
        };
        let tags = vec![tag1.clone(), tag2.clone(), tag3.clone()];
        let mut transaction = Transaction::new_with_tags(date, amount, account, tags);

        assert_eq!(transaction.tags().len(), 3);

        // Remove all tags with ID 1 (should remove tag1 and tag3)
        transaction.remove_tag(1);

        assert_eq!(transaction.tags().len(), 1);
        assert_eq!(transaction.tags()[0], tag2);

        // Remove non-existent tag
        transaction.remove_tag(999);
        assert_eq!(transaction.tags().len(), 1);

        // Remove the last tag
        transaction.remove_tag(2);
        assert!(transaction.tags().is_empty());
    }

    #[test]
    fn test_set_tags() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;
        let mut transaction = Transaction::new(date, amount, account);

        let tag1 = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };
        let tag2 = Tag {
            id: 2,
            name: "Essential".to_string(),
            description: None,
        };

        // Set initial tags
        let tags = vec![tag1.clone(), tag2.clone()];
        transaction.set_tags(tags.clone());
        assert_eq!(transaction.tags(), &tags);

        // Replace with different tags
        let tag3 = Tag {
            id: 3,
            name: "Entertainment".to_string(),
            description: None,
        };
        let new_tags = vec![tag3.clone()];
        transaction.set_tags(new_tags.clone());
        assert_eq!(transaction.tags(), &new_tags);

        // Set empty tags
        transaction.set_tags(vec![]);
        assert!(transaction.tags().is_empty());
    }

    #[test]
    fn test_set_tag() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;
        let tag1 = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };
        let tag2 = Tag {
            id: 2,
            name: "Essential".to_string(),
            description: None,
        };
        let tags = vec![tag1.clone(), tag2.clone()];
        let mut transaction = Transaction::new_with_tags(date, amount, account, tags);

        assert_eq!(transaction.tags().len(), 2);

        // Set a single tag (should replace all existing tags)
        let new_tag = Tag {
            id: 3,
            name: "Entertainment".to_string(),
            description: None,
        };
        transaction.set_tag(Some(new_tag.clone()));
        assert_eq!(transaction.tags().len(), 1);
        assert_eq!(transaction.tags()[0], new_tag);

        // Set to None (should clear all tags)
        transaction.set_tag(None);
        assert!(transaction.tags().is_empty());
    }

    #[test]
    fn test_tag_backward_compatibility() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let amount = Decimal::new(10000, 2); // 100.00
        let account = 1;

        // Test with no tags
        let transaction = Transaction::new(date, amount, account);
        assert!(transaction.tag().is_none());

        // Test with one tag
        let tag = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };
        let transaction = Transaction::new_with_tag(date, amount, account, tag.clone());
        assert_eq!(transaction.tag().unwrap(), &tag);

        // Test with multiple tags (should return first)
        let tag2 = Tag {
            id: 2,
            name: "Essential".to_string(),
            description: None,
        };
        let tags = vec![tag.clone(), tag2];
        let transaction = Transaction::new_with_tags(date, amount, account, tags);
        assert_eq!(transaction.tag().unwrap(), &tag); // Should return first tag
    }
}

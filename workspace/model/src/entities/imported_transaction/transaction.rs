use chrono::NaiveDate;
use async_trait::async_trait;
use sea_orm::{EntityTrait, ModelTrait, QueryFilter, ColumnTrait, DatabaseConnection, RelationTrait};

use crate::transaction::{Transaction, TransactionGenerator, Tag};
use crate::entities::{tag, imported_transaction_tag};
use super::Model as ImportedTransaction;

#[async_trait]
impl TransactionGenerator for ImportedTransaction {
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // Check if the transaction date is within the given range
        self.date >= start && self.date <= end
    }

    async fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        // Only generate a transaction if the date is within the range
        if self.has_any_transaction(start, end) {
            // Get the tag for this transaction
            let tag = self.get_tag_for_transaction().await;

            // Add transaction for the account with the tag if available
            if let Some(tag) = tag {
                transactions.push(Transaction::new_with_tag(
                    self.date,
                    self.amount,
                    self.account_id,
                    tag,
                ));
            } else {
                transactions.push(Transaction::new(
                    self.date,
                    self.amount,
                    self.account_id,
                ));
            }
        }

        transactions
    }

    async fn get_tag_for_transaction(&self) -> Option<Tag> {
        // In a real implementation, we would use a database connection pool
        // For example, we could get it from a global state or pass it as a parameter
        let db = sea_orm::Database::connect("sqlite::memory:").await.ok()?;

        // Query the database for tags associated with this imported transaction
        // Using the Related trait to find tags related to this transaction
        let tags = self.find_related(tag::Entity)
            .all(&db)
            .await
            .ok()?;

        // Return the first tag if any
        tags.first().map(|t| Tag {
            id: t.id,
            name: t.name.clone(),
            description: t.description.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use sea_orm::entity::prelude::*;

    #[tokio::test]
    async fn test_has_any_transaction() {
        let transaction = ImportedTransaction {
            id: 1,
            account_id: 1,
            date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            description: "Test".to_string(),
            amount: Decimal::new(100, 0),
            import_hash: "test_hash".to_string(),
            raw_data: None,
            reconciled_transaction_type: None,
            reconciled_transaction_id: None,
        };

        // Date range includes the transaction date
        assert!(transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()
        ));

        // Date range starts on the transaction date
        assert!(transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()
        ));

        // Date range ends on the transaction date
        assert!(transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()
        ));

        // Date range is before the transaction date
        assert!(!transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2022, 12, 1).unwrap(),
            NaiveDate::from_ymd_opt(2022, 12, 31).unwrap()
        ));

        // Date range is after the transaction date
        assert!(!transaction.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 28).unwrap()
        ));
    }

    #[tokio::test]
    async fn test_generate_transactions() {
        let transaction = ImportedTransaction {
            id: 1,
            account_id: 1,
            date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            description: "Test".to_string(),
            amount: Decimal::new(100, 0),
            import_hash: "test_hash".to_string(),
            raw_data: None,
            reconciled_transaction_type: None,
            reconciled_transaction_id: None,
        };

        let transactions = transaction
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
            )
            .await;

        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].date(), NaiveDate::from_ymd_opt(2023, 1, 15).unwrap());
        assert_eq!(transactions[0].amount(), Decimal::new(100, 0));
        assert_eq!(transactions[0].account(), 1);
    }
}

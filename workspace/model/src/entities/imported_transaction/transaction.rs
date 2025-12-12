use async_trait::async_trait;
use chrono::NaiveDate;
use sea_orm::{
    DatabaseConnection, EntityTrait, ModelTrait,
};

use super::Model as ImportedTransaction;
use crate::entities::{category, tag};
use crate::transaction::{Category, Tag, Transaction, TransactionGenerator};

#[async_trait]
impl TransactionGenerator for ImportedTransaction {
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // Check if the transaction date is within the given range
        self.date >= start && self.date <= end
    }

    async fn generate_transactions(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        today: NaiveDate,
        db: &DatabaseConnection,
    ) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        // Only generate a transaction if the date is within the range
        if self.has_any_transaction(start, end) {
            // Load tags for this transaction
            let tags = self.get_tag_for_transaction(db, false).await;
            let category = self.get_category_for_transaction(db, false).await;

            let mut transaction = if tags.is_empty() {
                Transaction::new(self.date, self.amount, self.account_id)
            } else {
                Transaction::new_with_tags(self.date, self.amount, self.account_id, tags)
            };
            transaction.set_category(category);

            // For one-off transactions: if the transaction date is today or in the past, mark as paid
            if self.date <= today {
                // Set paid_on to the transaction date at midnight (start of day)
                transaction.set_paid_on(Some(self.date.and_hms_opt(0, 0, 0).unwrap()));
            }

            transactions.push(transaction);
        }

        transactions
    }

    async fn get_category_for_transaction(
        &self,
        db: &DatabaseConnection,
        _expand: bool,
    ) -> Option<Category> {
        match self.category_id {
            Some(id) => match category::Entity::find_by_id(id).one(db).await {
                Ok(Some(cat)) => Some(Category {
                    id: cat.id,
                    name: cat.name,
                    description: cat.description,
                    parent_id: cat.parent_id,
                }),
                _ => None,
            },
            None => None,
        }
    }

    async fn get_tag_for_transaction(&self, db: &DatabaseConnection, expand: bool) -> Vec<Tag> {
        // Query the database for tags associated with this imported transaction
        // Using the Related trait to find tags related to this transaction
        let tag_models = match self.find_related(tag::Entity).all(db).await {
            Ok(tags) => tags,
            Err(_) => return Vec::new(),
        };

        let mut result_tags = Vec::new();

        for tag_model in tag_models {
            let tag = Tag {
                id: tag_model.id,
                name: tag_model.name.clone(),
                description: tag_model.description.clone(),
            };

            if expand {
                // Expand this tag to include its parent hierarchy
                match tag_model.expand(db).await {
                    Ok(expanded_tags) => {
                        for expanded_tag in expanded_tags {
                            let expanded = Tag {
                                id: expanded_tag.id,
                                name: expanded_tag.name,
                                description: expanded_tag.description,
                            };
                            if !result_tags.iter().any(|t: &Tag| t.id == expanded.id) {
                                result_tags.push(expanded);
                            }
                        }
                    }
                    Err(_) => {
                        // If expansion fails, just add the original tag
                        if !result_tags.iter().any(|t: &Tag| t.id == tag.id) {
                            result_tags.push(tag);
                        }
                    }
                }
            } else {
                // Just add the tag without expansion
                result_tags.push(tag);
            }
        }

        result_tags
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
            category_id: None,
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
        // Create a mock database connection for testing
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

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
            category_id: None,
        };

        let today = NaiveDate::from_ymd_opt(2023, 1, 20).unwrap(); // Set today to Jan 20, 2023
        let transactions = transaction
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 1);
        assert_eq!(
            transactions[0].date(),
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()
        );
        assert_eq!(transactions[0].amount(), Decimal::new(100, 0));
        assert_eq!(transactions[0].account(), 1);
        assert!(transactions[0].is_paid()); // Should be paid since Jan 15 <= Jan 20 (today)
    }

    #[tokio::test]
    async fn test_get_tag_for_transaction_no_expand() {
        // Create a mock database connection for testing
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

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
            category_id: None,
        };

        // Test with expand=false (should return empty since no tags are set up in the mock DB)
        let tags = transaction.get_tag_for_transaction(&db, false).await;
        assert!(tags.is_empty());

        // Test with expand=true (should also return empty since no tags are set up in the mock DB)
        let tags_expanded = transaction.get_tag_for_transaction(&db, true).await;
        assert!(tags_expanded.is_empty());
    }
}

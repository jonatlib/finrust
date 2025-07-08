use chrono::NaiveDate;
use async_trait::async_trait;
use sea_orm::{EntityTrait, ModelTrait, QueryFilter, ColumnTrait, DatabaseConnection, RelationTrait};

use crate::transaction::{Transaction, TransactionGenerator, Tag};
use crate::entities::{tag, one_off_transaction_tag};
use super::Model as OneOffTransaction;

#[async_trait]
impl TransactionGenerator for OneOffTransaction {
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // Check if the transaction date is within the given range
        self.date >= start && self.date <= end
    }

    async fn generate_transactions(&self, start: NaiveDate, end: NaiveDate, today: NaiveDate, db: &DatabaseConnection) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        // Only generate a transaction if the date is within the range
        if self.has_any_transaction(start, end) {
            // Load tags for this transaction
            let tags = self.get_tag_for_transaction(db, false).await;

            let mut target_transaction = if tags.is_empty() {
                Transaction::new(
                    self.date,
                    self.amount,
                    self.target_account_id,
                )
            } else {
                Transaction::new_with_tags(
                    self.date,
                    self.amount,
                    self.target_account_id,
                    tags.clone(),
                )
            };

            // For one-off transactions: if the transaction date is today or in the past, mark as paid
            if self.date <= today {
                // Set paid_on to the transaction date at midnight (start of day)
                target_transaction.set_paid_on(Some(self.date.and_hms_opt(0, 0, 0).unwrap()));
            }

            transactions.push(target_transaction);

            // If there's a source account, add a transaction for it as well
            if let Some(source_account_id) = self.source_account_id {
                // For the source account, the amount is negated
                let mut source_transaction = if tags.is_empty() {
                    Transaction::new(
                        self.date,
                        -self.amount,
                        source_account_id,
                    )
                } else {
                    Transaction::new_with_tags(
                        self.date,
                        -self.amount,
                        source_account_id,
                        tags,
                    )
                };

                // Apply the same payment logic to the source transaction
                if self.date <= today {
                    // Set paid_on to the transaction date at midnight (start of day)
                    source_transaction.set_paid_on(Some(self.date.and_hms_opt(0, 0, 0).unwrap()));
                }

                transactions.push(source_transaction);
            }
        }

        transactions
    }

    async fn get_tag_for_transaction(&self, db: &DatabaseConnection, expand: bool) -> Vec<Tag> {
        // Query the database for tags associated with this one-off transaction
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

    #[tokio::test]
    async fn test_has_any_transaction() {
        let transaction = OneOffTransaction {
            id: 1,
            name: "Test".to_string(),
            description: None,
            amount: Decimal::new(100, 0),
            date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            include_in_statistics: true,
            target_account_id: 1,
            source_account_id: None,
            ledger_name: None,
            linked_import_id: None,
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

        // Single account transaction
        let transaction = OneOffTransaction {
            id: 1,
            name: "Test".to_string(),
            description: None,
            amount: Decimal::new(100, 0),
            date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            include_in_statistics: true,
            target_account_id: 1,
            source_account_id: None,
            ledger_name: None,
            linked_import_id: None,
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
        assert_eq!(transactions[0].date(), NaiveDate::from_ymd_opt(2023, 1, 15).unwrap());
        assert_eq!(transactions[0].amount(), Decimal::new(100, 0));
        assert_eq!(transactions[0].account(), 1);
        assert!(transactions[0].is_paid()); // Should be paid since Jan 15 <= Jan 20 (today)

        // Dual account transaction (transfer)
        let transfer = OneOffTransaction {
            id: 2,
            name: "Transfer".to_string(),
            description: None,
            amount: Decimal::new(200, 0),
            date: NaiveDate::from_ymd_opt(2023, 1, 20).unwrap(),
            include_in_statistics: true,
            target_account_id: 2,
            source_account_id: Some(1),
            ledger_name: None,
            linked_import_id: None,
        };

        let today = NaiveDate::from_ymd_opt(2023, 1, 25).unwrap(); // Set today to Jan 25, 2023
        let transactions = transfer
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 2);

        // Target account transaction
        assert_eq!(transactions[0].date(), NaiveDate::from_ymd_opt(2023, 1, 20).unwrap());
        assert_eq!(transactions[0].amount(), Decimal::new(200, 0));
        assert_eq!(transactions[0].account(), 2);
        assert!(transactions[0].is_paid()); // Should be paid since Jan 20 <= Jan 25 (today)

        // Source account transaction (negated amount)
        assert_eq!(transactions[1].date(), NaiveDate::from_ymd_opt(2023, 1, 20).unwrap());
        assert_eq!(transactions[1].amount(), Decimal::new(-200, 0));
        assert_eq!(transactions[1].account(), 1);
        assert!(transactions[1].is_paid()); // Should be paid since Jan 20 <= Jan 25 (today)
    }
}

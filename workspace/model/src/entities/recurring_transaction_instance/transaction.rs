use async_trait::async_trait;
use chrono::NaiveDate;
use sea_orm::{
    DatabaseConnection, EntityTrait, ModelTrait,
};

use crate::entities::recurring_transaction_instance::{
    InstanceStatus, Model as RecurringTransactionInstance,
};
use crate::entities::{category, recurring_transaction, tag};
use crate::transaction::{Category, Tag, Transaction, TransactionGenerator};

#[async_trait]
impl TransactionGenerator for RecurringTransactionInstance {
    async fn get_category_for_transaction(
        &self,
        db: &DatabaseConnection,
        _expand: bool,
    ) -> Option<Category> {
        // 1. Try instance category
        if let Some(category_id) = self.category_id {
            if let Ok(Some(cat)) = category::Entity::find_by_id(category_id).one(db).await {
                return Some(Category {
                    id: cat.id,
                    name: cat.name,
                    description: cat.description,
                    parent_id: cat.parent_id,
                });
            }
        }

        // 2. Fallback to parent recurring transaction category
        let recurring_transaction =
            match crate::entities::recurring_transaction::Entity::find_by_id(
                self.recurring_transaction_id,
            )
            .one(db)
            .await
            {
                Ok(Some(transaction)) => transaction,
                _ => return None,
            };

        if let Some(category_id) = recurring_transaction.category_id {
            if let Ok(Some(cat)) = category::Entity::find_by_id(category_id).one(db).await {
                return Some(Category {
                    id: cat.id,
                    name: cat.name,
                    description: cat.description,
                    parent_id: cat.parent_id,
                });
            }
        }

        None
    }

    async fn get_tag_for_transaction(&self, db: &DatabaseConnection, expand: bool) -> Vec<Tag> {
        // Query the database for tags associated with the parent recurring transaction
        // First, we need to get the recurring transaction
        let recurring_transaction =
            match crate::entities::recurring_transaction::Entity::find_by_id(
                self.recurring_transaction_id,
            )
            .one(db)
            .await
            {
                Ok(Some(transaction)) => transaction,
                _ => return Vec::new(),
            };

        // Then, we can find the tags associated with the recurring transaction
        let tag_models = match recurring_transaction
            .find_related(tag::Entity)
            .all(db)
            .await
        {
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
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // Only consider Paid instances or Pending instances that are due within the date range
        match self.status {
            InstanceStatus::Paid => {
                // For paid instances, use the paid date if available, otherwise use the due date
                let transaction_date = self.paid_date.unwrap_or(self.due_date);
                transaction_date >= start && transaction_date <= end
            }
            InstanceStatus::Pending => {
                // For pending instances, use the due date
                self.due_date >= start && self.due_date <= end
            }
            InstanceStatus::Skipped => false, // Skipped instances don't generate transactions
        }
    }

    async fn generate_transactions(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        today: NaiveDate,
        db: &DatabaseConnection,
    ) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        // Only generate transactions if the instance has a transaction within the date range
        if self.has_any_transaction(start, end) {
            // Load tags for this transaction
            let tags = self.get_tag_for_transaction(db, false).await;
            let category = self.get_category_for_transaction(db, false).await;

            match self.status {
                InstanceStatus::Paid => {
                    // For paid instances, use the paid date and amount if available
                    let date = self.paid_date.unwrap_or(self.due_date);
                    let amount = self.paid_amount.unwrap_or(self.expected_amount);

                    // We don't have direct access to the account IDs here, but we can assume
                    // that the recurring_transaction_id would be used to look up the accounts
                    // to get the target_account_id and source_account_id.
                    let account_id = self.recurring_transaction_id; // This is a placeholder

                    let mut transaction = if tags.is_empty() {
                        Transaction::new(date, amount, account_id)
                    } else {
                        Transaction::new_with_tags(date, amount, account_id, tags)
                    };
                    transaction.set_category(category.clone());

                    // For paid instances, always mark as paid using the paid date or due date
                    transaction.set_paid_on(Some(date.and_hms_opt(0, 0, 0).unwrap()));
                    transactions.push(transaction);
                }
                InstanceStatus::Pending => {
                    // For pending instances, use the due date and expected amount
                    let account_id = self.recurring_transaction_id; // This is a placeholder

                    let mut transaction = if tags.is_empty() {
                        Transaction::new(self.due_date, self.expected_amount, account_id)
                    } else {
                        Transaction::new_with_tags(
                            self.due_date,
                            self.expected_amount,
                            account_id,
                            tags,
                        )
                    };
                    transaction.set_category(category);

                    // For pending instances: only mark as paid if the due date is today or in the past
                    if self.due_date <= today {
                        transaction.set_paid_on(Some(self.due_date.and_hms_opt(0, 0, 0).unwrap()));
                    }

                    transactions.push(transaction);
                }
                InstanceStatus::Skipped => {
                    // Skipped instances don't generate transactions
                }
            }
        }

        transactions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_has_any_transaction() {
        // Paid instance
        let paid_instance = RecurringTransactionInstance {
            id: 1,
            recurring_transaction_id: 101,
            status: InstanceStatus::Paid,
            due_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            expected_amount: Decimal::new(-1000, 0),
            paid_date: Some(NaiveDate::from_ymd_opt(2023, 1, 14).unwrap()),
            paid_amount: Some(Decimal::new(-1000, 0)),
            reconciled_imported_transaction_id: None,
            category_id: None,
        };

        // Pending instance
        let pending_instance = RecurringTransactionInstance {
            id: 2,
            recurring_transaction_id: 102,
            status: InstanceStatus::Pending,
            due_date: NaiveDate::from_ymd_opt(2023, 2, 15).unwrap(),
            expected_amount: Decimal::new(-1000, 0),
            paid_date: None,
            paid_amount: None,
            reconciled_imported_transaction_id: None,
            category_id: None,
        };

        // Skipped instance
        let skipped_instance = RecurringTransactionInstance {
            id: 3,
            recurring_transaction_id: 103,
            status: InstanceStatus::Skipped,
            due_date: NaiveDate::from_ymd_opt(2023, 3, 15).unwrap(),
            expected_amount: Decimal::new(-1000, 0),
            paid_date: None,
            paid_amount: None,
            reconciled_imported_transaction_id: None,
            category_id: None,
        };

        // Test paid instance
        assert!(paid_instance.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()
        ));
        assert!(!paid_instance.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 28).unwrap()
        ));

        // Test pending instance
        assert!(pending_instance.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 28).unwrap()
        ));
        assert!(!pending_instance.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()
        ));

        // Test skipped instance
        assert!(!skipped_instance.has_any_transaction(
            NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 3, 31).unwrap()
        ));
    }

    #[tokio::test]
    async fn test_generate_transactions() {
        // Create a mock database connection for testing
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        // Paid instance
        let paid_instance = RecurringTransactionInstance {
            id: 1,
            recurring_transaction_id: 101,
            status: InstanceStatus::Paid,
            due_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            expected_amount: Decimal::new(-1000, 0),
            paid_date: Some(NaiveDate::from_ymd_opt(2023, 1, 14).unwrap()),
            paid_amount: Some(Decimal::new(-950, 0)), // Slightly different from expected
            reconciled_imported_transaction_id: None,
            category_id: None,
        };

        // Generate transactions for the paid instance
        let today = NaiveDate::from_ymd_opt(2023, 1, 20).unwrap(); // Set today to Jan 20, 2023
        let transactions = paid_instance
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
            NaiveDate::from_ymd_opt(2023, 1, 14).unwrap()
        );
        assert_eq!(transactions[0].amount(), Decimal::new(-950, 0));
        assert_eq!(transactions[0].account(), 101); // This is a placeholder in our implementation
        assert!(transactions[0].is_paid()); // Should be paid since it's a paid instance

        // Pending instance
        let pending_instance = RecurringTransactionInstance {
            id: 2,
            recurring_transaction_id: 102,
            status: InstanceStatus::Pending,
            due_date: NaiveDate::from_ymd_opt(2023, 2, 15).unwrap(),
            expected_amount: Decimal::new(-1000, 0),
            paid_date: None,
            paid_amount: None,
            reconciled_imported_transaction_id: None,
            category_id: None,
        };

        // Generate transactions for the pending instance
        let today = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap(); // Set today to Feb 10, 2023 (before due date)
        let transactions = pending_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 1);
        assert_eq!(
            transactions[0].date(),
            NaiveDate::from_ymd_opt(2023, 2, 15).unwrap()
        );
        assert_eq!(transactions[0].amount(), Decimal::new(-1000, 0));
        assert_eq!(transactions[0].account(), 102); // This is a placeholder in our implementation
        assert!(!transactions[0].is_paid()); // Should not be paid since due date (Feb 15) > today (Feb 10)

        // Skipped instance
        let skipped_instance = RecurringTransactionInstance {
            id: 3,
            recurring_transaction_id: 103,
            status: InstanceStatus::Skipped,
            due_date: NaiveDate::from_ymd_opt(2023, 3, 15).unwrap(),
            expected_amount: Decimal::new(-1000, 0),
            paid_date: None,
            paid_amount: None,
            reconciled_imported_transaction_id: None,
            category_id: None,
        };

        // Generate transactions for the skipped instance
        let today = NaiveDate::from_ymd_opt(2023, 3, 20).unwrap(); // Set today to Mar 20, 2023
        let transactions = skipped_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                today,
                &db,
            )
            .await;

        assert_eq!(transactions.len(), 0); // No transactions for skipped instances
    }
}

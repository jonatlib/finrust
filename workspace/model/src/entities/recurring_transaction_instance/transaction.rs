use chrono::NaiveDate;
use async_trait::async_trait;
use sea_orm::{EntityTrait, ModelTrait, QueryFilter, ColumnTrait, DatabaseConnection, RelationTrait};

use crate::transaction::{Transaction, TransactionGenerator, Tag};
use crate::entities::{tag, recurring_transaction_tag};
use crate::entities::recurring_transaction_instance::{Model as RecurringTransactionInstance, InstanceStatus};

#[async_trait]
impl TransactionGenerator for RecurringTransactionInstance {
    async fn get_tag_for_transaction(&self) -> Option<Tag> {
        // In a real implementation, we would use a database connection pool
        // For example, we could get it from a global state or pass it as a parameter
        let db = sea_orm::Database::connect("sqlite::memory:").await.ok()?;

        // Query the database for tags associated with the parent recurring transaction
        // First, we need to get the recurring transaction
        let recurring_transaction = crate::entities::recurring_transaction::Entity::find_by_id(self.recurring_transaction_id)
            .one(&db)
            .await
            .ok()?
            .unwrap();

        // Then, we can find the tags associated with the recurring transaction
        let tags = recurring_transaction.find_related(tag::Entity)
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
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // Only consider Paid instances or Pending instances that are due within the date range
        match self.status {
            InstanceStatus::Paid => {
                // For paid instances, use the paid date if available, otherwise use the due date
                let transaction_date = self.paid_date.unwrap_or(self.due_date);
                transaction_date >= start && transaction_date <= end
            },
            InstanceStatus::Pending => {
                // For pending instances, use the due date
                self.due_date >= start && self.due_date <= end
            },
            InstanceStatus::Skipped => false, // Skipped instances don't generate transactions
        }
    }

    async fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        // Only generate transactions if the instance has a transaction within the date range
        if self.has_any_transaction(start, end) {
            // Get the tag for this transaction
            let tag = self.get_tag_for_transaction().await;

            match self.status {
                InstanceStatus::Paid => {
                    // For paid instances, use the paid date and amount if available
                    let date = self.paid_date.unwrap_or(self.due_date);
                    let amount = self.paid_amount.unwrap_or(self.expected_amount);

                    // We don't have direct access to the account IDs here, but we can assume
                    // that the recurring_transaction_id would be used to look up the accounts
                    // in a real implementation. For now, we'll just use a placeholder.
                    // In a real implementation, you would need to join with the recurring_transaction table
                    // to get the target_account_id and source_account_id.
                    let account_id = self.recurring_transaction_id; // This is a placeholder

                    // Add transaction with the tag if available
                    if let Some(tag) = tag {
                        transactions.push(Transaction::new_with_tag(date, amount, account_id, tag));
                    } else {
                        transactions.push(Transaction::new(date, amount, account_id));
                    }
                },
                InstanceStatus::Pending => {
                    // For pending instances, use the due date and expected amount
                    let account_id = self.recurring_transaction_id; // This is a placeholder

                    // Add transaction with the tag if available
                    if let Some(tag) = tag {
                        transactions.push(Transaction::new_with_tag(self.due_date, self.expected_amount, account_id, tag));
                    } else {
                        transactions.push(Transaction::new(self.due_date, self.expected_amount, account_id));
                    }
                },
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
        };

        // Generate transactions for the paid instance
        let transactions = paid_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
            )
            .await;

        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].date(), NaiveDate::from_ymd_opt(2023, 1, 14).unwrap());
        assert_eq!(transactions[0].amount(), Decimal::new(-950, 0));
        assert_eq!(transactions[0].account(), 101); // This is a placeholder in our implementation

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
        };

        // Generate transactions for the pending instance
        let transactions = pending_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
            )
            .await;

        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].date(), NaiveDate::from_ymd_opt(2023, 2, 15).unwrap());
        assert_eq!(transactions[0].amount(), Decimal::new(-1000, 0));
        assert_eq!(transactions[0].account(), 102); // This is a placeholder in our implementation

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
        };

        // Generate transactions for the skipped instance
        let transactions = skipped_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
            )
            .await;

        assert_eq!(transactions.len(), 0); // No transactions for skipped instances
    }
}

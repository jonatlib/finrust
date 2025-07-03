use chrono::NaiveDate;
use std::vec::IntoIter;

use crate::transaction::{Transaction, TransactionGenerator};
use crate::entities::recurring_transaction_instance::{Model as RecurringTransactionInstance, InstanceStatus};

impl TransactionGenerator for RecurringTransactionInstance {
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

    fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> impl Iterator<Item = Transaction> {
        let mut transactions = Vec::new();

        // Only generate transactions if the instance has a transaction within the date range
        if self.has_any_transaction(start, end) {
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

                    transactions.push(Transaction::new(date, amount, account_id));
                },
                InstanceStatus::Pending => {
                    // For pending instances, use the due date and expected amount
                    let account_id = self.recurring_transaction_id; // This is a placeholder
                    transactions.push(Transaction::new(self.due_date, self.expected_amount, account_id));
                },
                InstanceStatus::Skipped => {
                    // Skipped instances don't generate transactions
                }
            }
        }

        transactions.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn test_has_any_transaction() {
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

    #[test]
    fn test_generate_transactions() {
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
        let transactions: Vec<Transaction> = paid_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
            )
            .collect();

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
        let transactions: Vec<Transaction> = pending_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
            )
            .collect();

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
        let transactions: Vec<Transaction> = skipped_instance
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
            )
            .collect();

        assert_eq!(transactions.len(), 0); // No transactions for skipped instances
    }
}

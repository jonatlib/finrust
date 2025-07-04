use chrono::NaiveDate;

use crate::transaction::{Transaction, TransactionGenerator};
use super::Model as ImportedTransaction;

impl TransactionGenerator for ImportedTransaction {
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // Check if the transaction date is within the given range
        self.date >= start && self.date <= end
    }

    fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> impl Iterator<Item = Transaction> {
        let mut transactions = Vec::new();

        // Only generate a transaction if the date is within the range
        if self.has_any_transaction(start, end) {
            // Add transaction for the account
            transactions.push(Transaction::new(
                self.date,
                self.amount,
                self.account_id,
            ));
        }

        // Convert the Vec to an Iterator
        transactions.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use sea_orm::entity::prelude::*;

    #[test]
    fn test_has_any_transaction() {
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

    #[test]
    fn test_generate_transactions() {
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

        let transactions: Vec<Transaction> = transaction
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
            )
            .collect();

        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].date(), NaiveDate::from_ymd_opt(2023, 1, 15).unwrap());
        assert_eq!(transactions[0].amount(), Decimal::new(100, 0));
        assert_eq!(transactions[0].account(), 1);
    }
}

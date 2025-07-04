use chrono::NaiveDate;

use crate::transaction::{Transaction, TransactionGenerator};
use super::Model as OneOffTransaction;

impl TransactionGenerator for OneOffTransaction {
    fn has_any_transaction(&self, start: NaiveDate, end: NaiveDate) -> bool {
        // Check if the transaction date is within the given range
        self.date >= start && self.date <= end
    }

    fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> impl Iterator<Item = Transaction> {
        let mut transactions = Vec::new();

        // Only generate a transaction if the date is within the range
        if self.has_any_transaction(start, end) {
            // Add transaction for the target account
            transactions.push(Transaction::new(
                self.date,
                self.amount,
                self.target_account_id,
            ));

            // If there's a source account, add a transaction for it as well
            if let Some(source_account_id) = self.source_account_id {
                // For the source account, the amount is negated
                transactions.push(Transaction::new(
                    self.date,
                    -self.amount,
                    source_account_id,
                ));
            }
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

    #[test]
    fn test_has_any_transaction() {
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

    #[test]
    fn test_generate_transactions() {
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

        let transactions: Vec<Transaction> = transfer
            .generate_transactions(
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
            )
            .collect();

        assert_eq!(transactions.len(), 2);

        // Target account transaction
        assert_eq!(transactions[0].date(), NaiveDate::from_ymd_opt(2023, 1, 20).unwrap());
        assert_eq!(transactions[0].amount(), Decimal::new(200, 0));
        assert_eq!(transactions[0].account(), 2);

        // Source account transaction (negated amount)
        assert_eq!(transactions[1].date(), NaiveDate::from_ymd_opt(2023, 1, 20).unwrap());
        assert_eq!(transactions[1].amount(), Decimal::new(-200, 0));
        assert_eq!(transactions[1].account(), 1);
    }
}

use chrono::NaiveDate;
use model::transaction::Transaction;
use polars::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use std::iter::Iterator;

/// Extension trait for Transaction to convert to Polars
pub trait TransactionPolars {
    /// Convert a single Transaction to a Polars row (as Series)
    fn to_series(&self) -> Vec<Series>;

    /// Convert a single Transaction to a Polars DataFrame
    fn to_df(&self) -> Result<DataFrame, PolarsError>;
}

impl TransactionPolars for Transaction {
    fn to_series(&self) -> Vec<Series> {
        vec![
            Series::new("date".into(), &[self.date().and_hms_opt(0, 0, 0).unwrap().timestamp_millis()]),
            Series::new("amount".into(), &[self.amount().to_f64().unwrap_or(0.0)]),
            Series::new("account".into(), &[self.account()]),
        ]
    }

    fn to_df(&self) -> Result<DataFrame, PolarsError> {
        let series = self.to_series();
        DataFrame::new(series.into_iter().map(|s| s.into()).collect())
    }
}

/// Extension trait for iterators over Transactions
pub trait TransactionIteratorPolars<T: Iterator<Item=Transaction>> {
    /// Convert an iterator of Transactions to a Polars DataFrame
    fn to_df(self) -> Result<DataFrame, PolarsError>;
}

impl<T: Iterator<Item=Transaction>> TransactionIteratorPolars<T> for T {
    fn to_df(self) -> Result<DataFrame, PolarsError> {
        let mut dates = Vec::new();
        let mut amounts = Vec::new();
        let mut accounts = Vec::new();

        for transaction in self {
            dates.push(transaction.date().and_hms_opt(0, 0, 0).unwrap().timestamp_millis());
            amounts.push(transaction.amount().to_f64().unwrap_or(0.0));
            accounts.push(transaction.account());
        }

        let df = DataFrame::new(vec![
            Series::new("date".into(), dates).into(),
            Series::new("amount".into(), amounts).into(),
            Series::new("account".into(), accounts).into(),
        ])?;

        Ok(df)
    }
}

/// Helper function to convert a slice of Transactions to a DataFrame
pub fn transactions_to_df(transactions: &[Transaction]) -> Result<DataFrame, PolarsError> {
    transactions.iter().cloned().to_df()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_transaction_to_df() {
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let amount = Decimal::new(10050, 2); // 100.50
        let account = 123;

        let transaction = Transaction::new(date, amount, account);
        let expected_timestamp = date.and_hms_opt(0, 0, 0).unwrap().timestamp_millis();
        let expected_amount = amount.to_f64().unwrap();

        let df = transaction.to_df().unwrap();
        assert_eq!(df.shape(), (1, 3));

        // Check column names
        let column_names = df.get_column_names();
        assert!(column_names.iter().any(|name| name.contains("date")));
        assert!(column_names.iter().any(|name| name.contains("amount")));
        assert!(column_names.iter().any(|name| name.contains("account")));

        // Check values
        let date_series = df.column("date").unwrap();
        let amount_series = df.column("amount").unwrap();
        let account_series = df.column("account").unwrap();

        // Extract values from AnyValue types
        if let AnyValue::Int64(timestamp) = date_series.get(0).unwrap() {
            assert_eq!(timestamp, expected_timestamp);
        } else {
            panic!("Expected Int64 value for date");
        }

        if let AnyValue::Float64(amount_val) = amount_series.get(0).unwrap() {
            assert_eq!(amount_val, expected_amount);
        } else {
            panic!("Expected Float64 value for amount");
        }

        if let AnyValue::Int32(account_val) = account_series.get(0).unwrap() {
            assert_eq!(account_val, account);
        } else {
            panic!("Expected Int32 value for account");
        }
    }

    #[test]
    fn test_transactions_to_df() {
        let date1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let amount1 = Decimal::new(10050, 2); // 100.50
        let account1 = 123;

        let date2 = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
        let amount2 = Decimal::new(20025, 2); // 200.25
        let account2 = 456;

        let transactions = vec![
            Transaction::new(date1, amount1, account1),
            Transaction::new(date2, amount2, account2),
        ];

        // Expected values
        let expected_timestamp1 = date1.and_hms_opt(0, 0, 0).unwrap().timestamp_millis();
        let expected_amount1 = amount1.to_f64().unwrap();
        let expected_timestamp2 = date2.and_hms_opt(0, 0, 0).unwrap().timestamp_millis();
        let expected_amount2 = amount2.to_f64().unwrap();

        let df = transactions_to_df(&transactions).unwrap();
        assert_eq!(df.shape(), (2, 3));

        // Check values
        let date_series = df.column("date").unwrap();
        let amount_series = df.column("amount").unwrap();
        let account_series = df.column("account").unwrap();

        // Check first transaction
        if let AnyValue::Int64(timestamp) = date_series.get(0).unwrap() {
            assert_eq!(timestamp, expected_timestamp1);
        } else {
            panic!("Expected Int64 value for date");
        }

        if let AnyValue::Float64(amount_val) = amount_series.get(0).unwrap() {
            assert_eq!(amount_val, expected_amount1);
        } else {
            panic!("Expected Float64 value for amount");
        }

        if let AnyValue::Int32(account_val) = account_series.get(0).unwrap() {
            assert_eq!(account_val, account1);
        } else {
            panic!("Expected Int32 value for account");
        }

        // Check second transaction
        if let AnyValue::Int64(timestamp) = date_series.get(1).unwrap() {
            assert_eq!(timestamp, expected_timestamp2);
        } else {
            panic!("Expected Int64 value for date");
        }

        if let AnyValue::Float64(amount_val) = amount_series.get(1).unwrap() {
            assert_eq!(amount_val, expected_amount2);
        } else {
            panic!("Expected Float64 value for amount");
        }

        if let AnyValue::Int32(account_val) = account_series.get(1).unwrap() {
            assert_eq!(account_val, account2);
        } else {
            panic!("Expected Int32 value for account");
        }
    }
}

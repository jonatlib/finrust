use chrono::NaiveDate;
use model::transaction::Transaction;
use polars::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use std::iter::Iterator;

/// Extension trait for Transaction to convert to Polars data structures.
///
/// This trait extends the Transaction model with methods to convert it to Polars
/// data structures (Series and DataFrame) for data analysis and manipulation.
/// It provides a bridge between the domain model and the data analysis library.
pub trait TransactionPolars {
    /// Convert a single Transaction to a Polars row (as Series).
    ///
    /// Returns a vector of Series objects, each representing a column of data
    /// with a single value from the Transaction.
    fn to_series(&self) -> Vec<Series>;

    /// Convert a single Transaction to a Polars DataFrame.
    ///
    /// Creates a DataFrame with a single row containing the Transaction data.
    /// Returns a Result that may contain a PolarsError if the DataFrame creation fails.
    fn to_df(&self) -> Result<DataFrame, PolarsError>;
}

impl TransactionPolars for Transaction {
    /// Converts the Transaction to a vector of Polars Series.
    ///
    /// Each Series represents a column in a DataFrame:
    /// - date: Converted to timestamp in milliseconds (midnight of the transaction date)
    /// - amount: Converted from Decimal to f64 (with fallback to 0.0 if conversion fails)
    /// - account: The account ID as an i32
    fn to_series(&self) -> Vec<Series> {
        vec![
            Series::new(
                "date".into(),
                &[self.date().and_hms_opt(0, 0, 0).unwrap().timestamp_millis()],
            ),
            Series::new("amount".into(), &[self.amount().to_f64().unwrap_or(0.0)]),
            Series::new("account".into(), &[self.account()]),
        ]
    }

    /// Creates a Polars DataFrame from the Transaction.
    ///
    /// Uses the to_series() method to get the Series objects and then
    /// constructs a DataFrame with a single row containing the transaction data.
    fn to_df(&self) -> Result<DataFrame, PolarsError> {
        let series = self.to_series();
        DataFrame::new(series.into_iter().map(|s| s.into()).collect())
    }
}

/// Extension trait for iterators over Transactions.
///
/// This trait extends any iterator that yields Transaction objects with the ability
/// to convert the entire iterator into a Polars DataFrame. This is useful for
/// processing collections of transactions in bulk for data analysis.
pub trait TransactionIteratorPolars<T: Iterator<Item = Transaction>> {
    /// Convert an iterator of Transactions to a Polars DataFrame.
    ///
    /// Consumes the iterator and returns a DataFrame containing all transactions,
    /// with each transaction as a row and transaction properties as columns.
    fn to_df(self) -> Result<DataFrame, PolarsError>;
}

impl<T: Iterator<Item = Transaction>> TransactionIteratorPolars<T> for T {
    /// Implements the to_df method for any iterator over Transactions.
    ///
    /// This implementation:
    /// 1. Collects all transaction data into separate vectors for each column
    /// 2. Creates Series objects for each column
    /// 3. Constructs a DataFrame from these Series
    ///
    /// The resulting DataFrame has the following columns:
    /// - date: Timestamps in milliseconds (midnight of each transaction date)
    /// - amount: Transaction amounts as f64 values
    /// - account: Account IDs as i32 values
    fn to_df(self) -> Result<DataFrame, PolarsError> {
        let mut dates = Vec::new();
        let mut amounts = Vec::new();
        let mut accounts = Vec::new();

        // Collect all transaction data into separate vectors
        for transaction in self {
            dates.push(
                transaction
                    .date()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .timestamp_millis(),
            );
            amounts.push(transaction.amount().to_f64().unwrap_or(0.0));
            accounts.push(transaction.account());
        }

        // Create a DataFrame from the collected data
        let df = DataFrame::new(vec![
            Series::new("date".into(), dates).into(),
            Series::new("amount".into(), amounts).into(),
            Series::new("account".into(), accounts).into(),
        ])?;

        Ok(df)
    }
}

/// Helper function to convert a slice of Transactions to a DataFrame.
///
/// This is a convenience function that wraps the functionality provided by
/// the TransactionIteratorPolars trait. It takes a slice of Transaction objects,
/// creates an iterator over cloned transactions, and converts them to a DataFrame.
///
/// # Arguments
///
/// * `transactions` - A slice of Transaction objects to convert
///
/// # Returns
///
/// * `Result<DataFrame, PolarsError>` - A DataFrame containing the transaction data,
///   or a PolarsError if the conversion fails
///
/// # Example
///
/// ```
/// # // This is a doctest, so we need to set up the environment
/// # extern crate compute;
/// # extern crate model;
/// # extern crate chrono;
/// # extern crate rust_decimal;
/// use chrono::NaiveDate;
/// use rust_decimal::Decimal;
/// use model::transaction::Transaction;
/// use compute::transaction::transactions_to_df;
///
/// let date1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
/// let amount1 = Decimal::new(10050, 2); // 100.50
/// let account1 = 123;
///
/// let date2 = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
/// let amount2 = Decimal::new(20025, 2); // 200.25
/// let account2 = 456;
///
/// let transactions = vec![
///     Transaction::new(date1, amount1, account1),
///     Transaction::new(date2, amount2, account2),
/// ];
/// let df = transactions_to_df(&transactions).unwrap();
/// ```
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

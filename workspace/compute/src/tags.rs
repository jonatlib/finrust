use chrono::NaiveDate;
use model::transaction::{Tag, Transaction};
use polars::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use tracing::{debug, info, instrument};

use crate::error::Result;

/// A computer that calculates expenses and incomes per account, per day, per tag.
///
/// This computer processes transactions and creates a DataFrame with multi-index
/// of date, account, single-tag and the corresponding value (sum of expenses/incomes).
/// This is not a balance calculation, just a sum per tag, account, and day.
#[derive(Debug)]
pub struct TagsComputer;

impl TagsComputer {
    /// Creates a new TagsComputer instance.
    pub fn new() -> Self {
        Self
    }

    /// Computes expenses and incomes per account, per day, per tag.
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection for retrieving data
    /// * `transactions` - Vector of transactions to process
    /// * `start_date` - Start date for the computation range
    /// * `end_date` - End date for the computation range
    ///
    /// # Returns
    ///
    /// A DataFrame with columns: date, account, tag_id, tag_name, amount
    /// where each row represents the sum of transactions for a specific
    /// date, account, and tag combination.
    #[instrument(skip(db, transactions), fields(num_transactions = transactions.len(), start_date = %start_date, end_date = %end_date))]
    pub async fn compute_tags_summary(
        &self,
        db: &DatabaseConnection,
        transactions: Vec<Transaction>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        info!(
            "Computing tags summary for {} transactions from {} to {}",
            transactions.len(),
            start_date,
            end_date
        );

        // Filter transactions within the date range
        let filtered_transactions: Vec<Transaction> = transactions
            .into_iter()
            .filter(|t| t.date() >= start_date && t.date() <= end_date)
            .collect();

        debug!(
            "Filtered to {} transactions within date range",
            filtered_transactions.len()
        );

        // Group transactions by (date, account, tag) and sum amounts
        let mut tag_data: HashMap<(NaiveDate, i32, i32, String), rust_decimal::Decimal> =
            HashMap::new();

        for transaction in filtered_transactions {
            let date = transaction.date();
            let account = transaction.account();
            let amount = transaction.amount();

            // Process each tag in the transaction
            for tag in transaction.tags() {
                let key = (date, account, tag.id, tag.name.clone());
                *tag_data.entry(key).or_insert(rust_decimal::Decimal::ZERO) += amount;
            }

            // If transaction has no tags, create an entry with a default "untagged" tag
            if transaction.tags().is_empty() {
                let key = (date, account, -1, "untagged".to_string());
                *tag_data.entry(key).or_insert(rust_decimal::Decimal::ZERO) += amount;
            }
        }

        // Convert the grouped data to DataFrame
        self.create_dataframe_from_tag_data(tag_data)
    }

    /// Creates a DataFrame from the grouped tag data.
    fn create_dataframe_from_tag_data(
        &self,
        tag_data: HashMap<(NaiveDate, i32, i32, String), rust_decimal::Decimal>,
    ) -> Result<DataFrame> {
        let mut dates = Vec::new();
        let mut accounts = Vec::new();
        let mut tag_ids = Vec::new();
        let mut tag_names = Vec::new();
        let mut amounts = Vec::new();

        // Sort by date, account, tag_id for consistent ordering
        let mut sorted_data: Vec<_> = tag_data.into_iter().collect();
        sorted_data.sort_by_key(|(key, _)| (key.0, key.1, key.2));

        for ((date, account, tag_id, tag_name), amount) in sorted_data {
            dates.push(
                date.and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp_millis(),
            );
            accounts.push(account);
            tag_ids.push(tag_id);
            tag_names.push(tag_name);
            amounts.push(amount.to_f64().unwrap_or(0.0));
        }

        let df = DataFrame::new(vec![
            Series::new("date".into(), dates).into(),
            Series::new("account".into(), accounts).into(),
            Series::new("tag_id".into(), tag_ids).into(),
            Series::new("tag_name".into(), tag_names).into(),
            Series::new("amount".into(), amounts).into(),
        ])?;

        Ok(df)
    }
}

impl Default for TagsComputer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_tags_computer_empty_transactions() {
        let computer = TagsComputer::new();
        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();

        // Mock database connection (not used in this test)
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        let result = computer
            .compute_tags_summary(&db, vec![], start_date, end_date)
            .await;

        assert!(result.is_ok());
        let df = result.unwrap();
        assert_eq!(df.height(), 0);
    }

    #[tokio::test]
    async fn test_tags_computer_with_transactions() {
        let computer = TagsComputer::new();
        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();

        // Mock database connection
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();

        // Create test transactions with tags
        let tag1 = Tag {
            id: 1,
            name: "Groceries".to_string(),
            description: Some("Food expenses".to_string()),
        };

        let tag2 = Tag {
            id: 2,
            name: "Transport".to_string(),
            description: Some("Transportation expenses".to_string()),
        };

        let transaction1 = Transaction::new_with_tag(
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            Decimal::new(-5000, 2), // -50.00
            1,                      // account 1
            tag1,
        );

        let transaction2 = Transaction::new_with_tag(
            NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            Decimal::new(-2000, 2), // -20.00
            1,                      // account 1
            tag2,
        );

        let transactions = vec![transaction1, transaction2];

        let result = computer
            .compute_tags_summary(&db, transactions, start_date, end_date)
            .await;

        assert!(result.is_ok());
        let df = result.unwrap();
        assert_eq!(df.height(), 2); // Two different tag entries

        // Check that we have the expected columns
        let columns = df.get_column_names();
        let column_names: Vec<String> = columns.iter().map(|s| s.to_string()).collect();
        assert!(column_names.contains(&"date".to_string()));
        assert!(column_names.contains(&"account".to_string()));
        assert!(column_names.contains(&"tag_id".to_string()));
        assert!(column_names.contains(&"tag_name".to_string()));
        assert!(column_names.contains(&"amount".to_string()));
    }
}

use chrono::NaiveDate;
use model::transaction::{Transaction, Category};
use polars::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use tracing::{debug, info, instrument};

use crate::error::Result;

/// A computer that calculates expenses and incomes per account, per day, per category.
///
/// This computer processes transactions and creates a DataFrame with multi-index
/// of date, account, category and the corresponding value (sum of expenses/incomes).
#[derive(Debug)]
pub struct CategoriesComputer;

impl CategoriesComputer {
    /// Creates a new CategoriesComputer instance.
    pub fn new() -> Self {
        Self
    }

    /// Computes expenses and incomes per account, per day, per category.
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
    /// A DataFrame with columns: date, account, category_id, category_name, amount
    /// where each row represents the sum of transactions for a specific
    /// date, account, and category combination.
    #[instrument(skip(db, transactions), fields(num_transactions = transactions.len(), start_date = %start_date, end_date = %end_date))]
    pub async fn compute_categories_summary(
        &self,
        db: &DatabaseConnection,
        transactions: Vec<Transaction>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        info!(
            "Computing categories summary for {} transactions from {} to {}",
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

        // Group transactions by (date, account, category) and sum amounts
        let mut category_data: HashMap<(NaiveDate, i32, i32, String), rust_decimal::Decimal> =
            HashMap::new();

        for transaction in filtered_transactions {
            let date = transaction.date();
            let account = transaction.account();
            let amount = transaction.amount();

            // Process category in the transaction
            match transaction.category() {
                Some(cat) => {
                    let key = (date, account, cat.id, cat.name.clone());
                    *category_data.entry(key).or_insert(rust_decimal::Decimal::ZERO) += amount;
                }
                None => {
                    // Untagged/Uncategorized
                    let key = (date, account, -1, "Uncategorized".to_string());
                    *category_data.entry(key).or_insert(rust_decimal::Decimal::ZERO) += amount;
                }
            }
        }

        // Convert the grouped data to DataFrame
        self.create_dataframe_from_category_data(category_data)
    }

    /// Creates a DataFrame from the grouped category data.
    fn create_dataframe_from_category_data(
        &self,
        category_data: HashMap<(NaiveDate, i32, i32, String), rust_decimal::Decimal>,
    ) -> Result<DataFrame> {
        let mut dates = Vec::new();
        let mut accounts = Vec::new();
        let mut category_ids = Vec::new();
        let mut category_names = Vec::new();
        let mut amounts = Vec::new();

        // Sort by date, account, category_id for consistent ordering
        let mut sorted_data: Vec<_> = category_data.into_iter().collect();
        sorted_data.sort_by_key(|(key, _)| (key.0, key.1, key.2));

        for ((date, account, category_id, category_name), amount) in sorted_data {
            dates.push(
                date.and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp_millis(),
            );
            accounts.push(account);
            category_ids.push(category_id);
            category_names.push(category_name);
            amounts.push(amount.to_f64().unwrap_or(0.0));
        }

        let df = DataFrame::new(vec![
            Series::new("date".into(), dates).into(),
            Series::new("account".into(), accounts).into(),
            Series::new("category_id".into(), category_ids).into(),
            Series::new("category_name".into(), category_names).into(),
            Series::new("amount".into(), amounts).into(),
        ])?;

        Ok(df)
    }
}

impl Default for CategoriesComputer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use sea_orm::Database;

    #[tokio::test]
    async fn test_compute_categories_summary() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let computer = CategoriesComputer::new();

        let cat1 = Category {
            id: 1,
            name: "Food".to_string(),
            description: None,
            parent_id: None,
        };
        let cat2 = Category {
            id: 2,
            name: "Utilities".to_string(),
            description: None,
            parent_id: None,
        };

        let mut t1 = Transaction::new(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            Decimal::new(100, 0),
            1,
        );
        t1.set_category(Some(cat1.clone()));

        let mut t2 = Transaction::new(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            Decimal::new(50, 0),
            1,
        );
        t2.set_category(Some(cat1.clone())); // Same category, same day

        let mut t3 = Transaction::new(
            NaiveDate::from_ymd_opt(2023, 1, 2).unwrap(),
            Decimal::new(200, 0),
            1,
        );
        t3.set_category(Some(cat2.clone()));

        let mut t4 = Transaction::new(
            NaiveDate::from_ymd_opt(2023, 1, 2).unwrap(),
            Decimal::new(75, 0),
            1,
        );
        // No category (Uncategorized)

        let transactions = vec![t1, t2, t3, t4];

        let df = computer
            .compute_categories_summary(
                &db,
                transactions,
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(df.height(), 3); // Food (Jan 1), Utilities (Jan 2), Uncategorized (Jan 2)

        // Filter for Food
        let mask = df
            .column("category_name")
            .unwrap()
            .str()
            .unwrap()
            .equal("Food");
        let food_df = df.filter(&mask).unwrap();
        assert_eq!(food_df.height(), 1);
        assert_eq!(
            food_df
                .column("amount")
                .unwrap()
                .f64()
                .unwrap()
                .get(0)
                .unwrap(),
            150.0
        ); // 100 + 50

        // Filter for Utilities
        let mask = df
            .column("category_name")
            .unwrap()
            .str()
            .unwrap()
            .equal("Utilities");
        let utilities_df = df.filter(&mask).unwrap();
        assert_eq!(utilities_df.height(), 1);
        assert_eq!(
            utilities_df
                .column("amount")
                .unwrap()
                .f64()
                .unwrap()
                .get(0)
                .unwrap(),
            200.0
        );

        // Filter for Uncategorized
        let mask = df
            .column("category_name")
            .unwrap()
            .str()
            .unwrap()
            .equal("Uncategorized");
        let uncategorized_df = df.filter(&mask).unwrap();
        assert_eq!(uncategorized_df.height(), 1);
        assert_eq!(
            uncategorized_df
                .column("amount")
                .unwrap()
                .f64()
                .unwrap()
                .get(0)
                .unwrap(),
            75.0
        );
    }
}

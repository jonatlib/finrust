use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use tracing::{debug, info, instrument};

use super::{AccountStateCalculator, MergeMethod};
use crate::error::Result;

// Correctly import the renamed functions
use super::forecast::recurring::{get_past_due_transactions, get_recurring_income};

/// A calculator that computes future non-paid recurring transactions and income.
///
/// This calculator is responsible for projecting recurring transactions and income
/// that have not yet been paid (no instance created) into the future.
pub struct UnpaidRecurringCalculator {
    /// The merge method to use when combining results from multiple calculators.
    merge_method: MergeMethod,
    /// The date to use as "today" for determining which recurring transactions to include.
    today: NaiveDate,
    /// The offset to use for past recurring transactions without instances.
    future_offset: Duration,
}

impl UnpaidRecurringCalculator {
    /// Creates a new unpaid recurring calculator with the specified merge method, today date, and future offset.
    pub fn new(merge_method: MergeMethod, today: NaiveDate, future_offset: Duration) -> Self {
        Self {
            merge_method,
            today,
            future_offset,
        }
    }

    /// Creates a new unpaid recurring calculator with the Sum merge method and the specified today date and future offset.
    pub fn new_with_sum_merge(today: NaiveDate, future_offset: Duration) -> Self {
        Self {
            merge_method: MergeMethod::Sum,
            today,
            future_offset,
        }
    }

    /// Creates a new unpaid recurring calculator with default values.
    pub fn default() -> Self {
        Self {
            merge_method: MergeMethod::Sum,
            today: chrono::Local::now().date_naive(),
            future_offset: Duration::days(7),
        }
    }
}

#[async_trait]
impl AccountStateCalculator for UnpaidRecurringCalculator {
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        compute_unpaid_recurring(
            db,
            accounts,
            start_date,
            end_date,
            self.today,
            self.future_offset,
        )
        .await
    }

    fn merge_method(&self) -> MergeMethod {
        self.merge_method
    }

    fn update_initial_balance(&mut self, _balance: rust_decimal::Decimal) -> bool {
        // This calculator doesn't use initial balance
        false
    }
}

#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date, today = %today, future_offset = %future_offset.num_days()
))]
async fn compute_unpaid_recurring(
    db: &DatabaseConnection,
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
    today: NaiveDate,
    future_offset: Duration,
) -> Result<DataFrame> {
    info!(
        "Computing unpaid recurring transactions for {} accounts from {} to {}",
        accounts.len(),
        start_date,
        end_date
    );

    let mut all_deltas: Vec<(i32, NaiveDate, Decimal)> = Vec::new();

    for account in accounts {
        debug!(
            "Processing account: id={}, name={}",
            account.id, account.name
        );

        // FIX 2: Call the correct, specialized function
        let recurring_transactions =
            get_past_due_transactions(db, account.id, start_date, today, future_offset).await?;

        let recurring_income =
            get_recurring_income(db, account.id, start_date, today, today, future_offset).await?;

        for (date, tx) in recurring_transactions {
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };
            if !amount.is_zero() {
                all_deltas.push((account.id, date, amount));
            }
        }

        // The tuple for income needs to be destructured correctly if its signature is also changed.
        // Assuming (NaiveDate, Model) for now.
        for (date, income) in recurring_income {
            if income.target_account_id == account.id && !income.amount.is_zero() {
                all_deltas.push((account.id, date, income.amount));
            }
        }
    }

    if all_deltas.is_empty() {
        return create_zeroed_dataframe(accounts, start_date, end_date);
    }

    let account_ids: Vec<i32> = all_deltas.iter().map(|(id, _, _)| *id).collect();
    let dates: Vec<NaiveDate> = all_deltas.iter().map(|(_, date, _)| *date).collect();
    let deltas: Vec<f64> = all_deltas
        .iter()
        .map(|(_, _, a)| a.to_string().parse::<f64>().unwrap_or(0.0))
        .collect();

    let deltas_df = DataFrame::new(vec![
        Column::new("account_id".into(), account_ids),
        Column::new("date".into(), dates),
        Column::new("delta".into(), deltas),
    ])?;

    let scaffold_df = build_scaffold_df(accounts, start_date, end_date)?;

    let result_df = scaffold_df
        .lazy()
        .join(
            deltas_df.lazy(),
            [col("account_id"), col("date")],
            [col("account_id"), col("date")],
            JoinArgs::new(JoinType::Left),
        )
        .drop(["balance"])
        .with_column(col("delta").fill_null(0.0f64))
        .group_by_stable([col("account_id"), col("date")])
        .agg([col("delta").sum().alias("delta_sum")])
        .sort(["account_id", "date"], Default::default())
        .with_column(
            col("delta_sum")
                .cum_sum(false)
                .over([col("account_id")])
                .alias("balance"),
        )
        .select([
            col("account_id"),
            col("date"),
            col("balance").cast(DataType::String),
        ])
        .collect()?;

    info!(
        "Unpaid recurring computation completed successfully with {} data points",
        result_df.height()
    );
    Ok(result_df)
}

// Helper function to create a zero-filled DataFrame if no transactions are found
fn create_zeroed_dataframe(
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<DataFrame> {
    build_scaffold_df(accounts, start_date, end_date)
}

// Helper function to build the full date range for all accounts, initialized to zero
fn build_scaffold_df(
    accounts: &[account::Model],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<DataFrame> {
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();

    for account in accounts {
        let mut current_date = start_date;
        while current_date <= end_date {
            account_ids.push(account.id);
            dates.push(current_date);
            if current_date == NaiveDate::MAX {
                break;
            }
            current_date = current_date.succ_opt().unwrap_or(current_date);
        }
    }

    let zero_balances = vec![0.0f64; dates.len()];
    DataFrame::new(vec![
        Column::new("account_id".into(), account_ids),
        Column::new("date".into(), dates),
        Column::new("balance".into(), zero_balances),
    ])
    .map_err(Into::into)
}

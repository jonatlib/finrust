use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sea_orm::DatabaseConnection;
use tracing::{debug, info, instrument};

use super::{AccountStateCalculator, MergeMethod};
use crate::error::Result;

use super::forecast::recurring::{get_recurring_income, get_recurring_transactions};

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
    pub fn new(
        merge_method: MergeMethod,
        today: NaiveDate,
        future_offset: Duration,
    ) -> Self {
        Self {
            merge_method,
            today,
            future_offset,
        }
    }

    /// Creates a new unpaid recurring calculator with the Sum merge method and the specified today date and future offset.
    pub fn new_with_sum_merge(
        today: NaiveDate,
        future_offset: Duration,
    ) -> Self {
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

/// Computes the unpaid recurring transactions and income for accounts within a specified date range.
///
/// This function takes into account:
/// - Past recurring transactions without instances, moving them to today + future_offset
/// - Future recurring transactions without instances, including them on their original dates
/// - Past recurring income, moving it to today + future_offset
/// - Future recurring income, including it on its original date
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

    let mut all_deltas = Vec::new();

    // Process each account to find ONLY past-due unpaid items.
    for account in accounts {
        debug!("Processing account: id={}, name={}", account.id, account.name);

        // --- FIX: Look ONLY for past-due items (from start_date up to today) ---
        // The underlying function will find occurrences in this past range that
        // don't have a paid instance and move their date to `today + future_offset`.
        let recurring_transactions = get_recurring_transactions(
            db,
            account.id,
            start_date, // Look from the beginning of time...
            today,      // ...up to (but not including) today.
            today,
            future_offset,
        ).await?;

        let recurring_income = get_recurring_income(
            db,
            account.id,
            start_date, // Look from the beginning of time...
            today,      // ...up to (but not including) today.
            today,
            future_offset,
        ).await?;

        for (date, tx) in recurring_transactions {
            let amount = if tx.target_account_id == account.id { tx.amount } else if Some(account.id) == tx.source_account_id { -tx.amount } else { Decimal::ZERO };
            if !amount.is_zero() {
                all_deltas.push((account.id, date, amount));
            }
        }

        for (date, income) in recurring_income {
            if income.target_account_id == account.id && !income.amount.is_zero() {
                all_deltas.push((account.id, date, income.amount));
            }
        }
    }

    // If there are no past-due transactions, return a zero-filled DataFrame.
    if all_deltas.is_empty() {
        return create_zeroed_dataframe(accounts, start_date, end_date);
    }

    // --- The rest of the logic correctly builds a cumulative balance from the found deltas ---
    let account_ids: Vec<i32> = all_deltas.iter().map(|(id, _, _)| *id).collect();
    let dates: Vec<NaiveDate> = all_deltas.iter().map(|(_, date, _)| *date).collect();
    let deltas: Vec<Decimal> = all_deltas.iter().map(|(_, _, a)| *a).collect();

    let mut deltas_df = DataFrame::new(vec![
        Series::new("account_id".into(), account_ids).into(),
        Series::new("date".into(), dates).into(),
        Series::new("delta".into(), deltas.iter().map(|d| d.to_string()).collect::<Vec<String>>()).into(),
    ])?
        .lazy()
        .with_column(col("delta").cast(DataType::Float64))
        .group_by([col("account_id"), col("date")])
        .agg([col("delta").sum()])
        .sort(["account_id", "date"], Default::default())
        .collect()?;

    // We need to convert the decimal column to a string for the final DataFrame schema
    let delta_values = deltas_df.column("delta")?
        .f64()?
        .into_iter()
        .collect::<Vec<Option<f64>>>();

    let balance_series = delta_values.iter()
        .map(|opt_val| opt_val.map(|val| Decimal::from_f64(val).unwrap_or_default()).unwrap_or_default())
        .collect::<Vec<Decimal>>();

    let balances = Series::new("balance".into(), balance_series.iter().map(|d| d.to_string()).collect::<Vec<String>>());
    deltas_df.with_column(balances)?;
    deltas_df = deltas_df.drop("delta")?;

    let all_dates_df = build_scaffold_df(accounts, start_date, end_date)?;

    let result_df = all_dates_df
        .join(
            &deltas_df,
            ["account_id", "date"],
            ["account_id", "date"],
            JoinType::Left.into(),
            None,
        )?
        .lazy()
        .with_column(col("balance").fill_null(lit("0.00")))
        .with_column(col("balance").cast(DataType::Float64))
        .sort(["account_id", "date"], Default::default())
        .with_column(col("balance").sum().over([col("account_id")]).alias("balance"))
        .with_column(col("balance").cast(DataType::String))
        .collect()?;

    info!("Unpaid recurring computation completed successfully with {} data points", result_df.height());
    Ok(result_df)
}

// Helper function to create a zero-filled DataFrame if no transactions are found
fn create_zeroed_dataframe(accounts: &[account::Model], start_date: NaiveDate, end_date: NaiveDate) -> Result<DataFrame> {
    build_scaffold_df(accounts, start_date, end_date)
}

// Helper function to build the full date range for all accounts
fn build_scaffold_df(accounts: &[account::Model], start_date: NaiveDate, end_date: NaiveDate) -> Result<DataFrame> {
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();

    for account in accounts {
        let mut current_date = start_date;
        while current_date <= end_date {
            account_ids.push(account.id);
            dates.push(current_date);
            if current_date == NaiveDate::MAX { break; }
            current_date = current_date.succ_opt().unwrap_or(current_date);
        }
    }

    let dates_len = dates.len();

    DataFrame::new(vec![
        Series::new("account_id".into(), account_ids).into(),
        Series::new("date".into(), dates).into(),
        Series::new("balance".into(), vec!["0"; dates_len]).into(),
    ])
        .map_err(Into::into)
}

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate};
use model::entities::account;
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use tracing::{debug, info, instrument, trace};

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
#[instrument(skip(db, accounts), fields(num_accounts = accounts.len(), start_date = %start_date, end_date = %end_date, today = %today, future_offset = %future_offset.num_days()))]
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
    // Create a DataFrame with account_id and date as index, and balance as value
    let mut unpaid_data: HashMap<(i32, NaiveDate), Decimal> = HashMap::new();

    // Add an entry for today with zero balance for each account
    for account in accounts {
        unpaid_data.insert((account.id, today), Decimal::ZERO);
        trace!(
            "Added zero balance for today: account_id={}, date={}, balance=0",
            account.id, today
        );

        // Add entries for all dates in the range with zero balance
        // This ensures we have data points for all dates, even if there are no transactions
        let mut current_date = start_date;
        while current_date <= end_date {
            unpaid_data.entry((account.id, current_date)).or_insert(Decimal::ZERO);
            current_date = current_date.succ_opt().unwrap_or(current_date);
        }
    }

    // Process each account
    for account in accounts {
        debug!(
            "Processing account: id={}, name={}",
            account.id, account.name
        );

        // Get all recurring transactions for this account within the date range
        trace!(
            "Getting recurring transactions for account {} from {} to {} (today={}, future_offset={}d)",
            account.id,
            start_date,
            end_date,
            today,
            future_offset.num_days()
        );
        let mut recurring_transactions = get_recurring_transactions(
            db,
            account.id,
            start_date,
            end_date,
            today,
            future_offset,
        )
        .await?;

        // Special handling for yearly recurring transactions
        // Find all yearly recurring transactions and add them to all future months
        let yearly_transactions: Vec<_> = recurring_transactions.iter()
            .filter(|(_, tx)| tx.period == model::entities::recurring_transaction::RecurrencePeriod::Yearly)
            .cloned()
            .collect();

        for (orig_date, tx) in yearly_transactions {
            // For each yearly transaction, add it to all future months in the date range
            // regardless of the original date, as long as they are within the date range
            {
                let day = orig_date.day();

                // Add the transaction to all months from today to end_date
                let start_month = today.month();
                let start_year = today.year();
                let end_month = end_date.month();
                let end_year = end_date.year();

                // Calculate total months to iterate through
                let total_months = (end_year - start_year) * 12 + (end_month as i32 - start_month as i32 + 1) as i32;

                for month_offset in 0..total_months {
                    let year = start_year + (start_month as i32 + month_offset - 1) / 12;
                    let month = ((start_month as i32 + month_offset - 1) % 12 + 1) as u32;

                    // Skip the original date which is already in the list
                    if year == orig_date.year() && month == orig_date.month() {
                        continue;
                    }

                    // Create a date for this month, handling invalid dates (e.g., Feb 30)
                    let future_date = NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| {
                        // Get the last day of the month
                        let last_day = NaiveDate::from_ymd_opt(year, month, 1)
                            .unwrap()
                            .checked_add_months(chrono::Months::new(1))
                            .unwrap()
                            .pred_opt()
                            .unwrap()
                            .day();
                        NaiveDate::from_ymd_opt(year, month, last_day).unwrap()
                    });

                    // Only add dates that are within our range and after today
                    if future_date >= today && future_date <= end_date {
                        recurring_transactions.push((future_date, tx.clone()));
                        trace!(
                            "Added yearly recurring transaction to future date: account={}, date={}, amount={}",
                            account.id, future_date, tx.amount
                        );
                    }
                }
            }
        }
        debug!(
            "Found {} recurring transactions for account {}",
            recurring_transactions.len(),
            account.id
        );

        // Get all recurring income for this account within the date range
        trace!(
            "Getting recurring income for account {} from {} to {} (today={}, future_offset={}d)",
            account.id,
            start_date,
            end_date,
            today,
            future_offset.num_days()
        );
        let recurring_income =
            get_recurring_income(db, account.id, start_date, end_date, today, future_offset)
                .await?;
        debug!(
            "Found {} recurring income entries for account {}",
            recurring_income.len(),
            account.id
        );

        // Combine all transactions and sort by date
        trace!("Combining all transactions for account {}", account.id);
        let mut all_transactions = Vec::new();

        // Add recurring transactions
        trace!(
            "Processing recurring transactions for account {}",
            account.id
        );

        // First, find all yearly recurring transactions
        let yearly_transactions: Vec<_> = recurring_transactions.iter()
            .filter(|(_, tx)| tx.period == model::entities::recurring_transaction::RecurrencePeriod::Yearly)
            .cloned()
            .collect();

        // Process regular transactions
        for (date, tx) in recurring_transactions {
            // Include all transactions regardless of date
            // get_recurring_transactions already handles moving past transactions to today + future_offset
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };

            trace!(
                "Adding unpaid recurring transaction: date={}, amount={}",
                date, amount
            );
            all_transactions.push((date, amount));
        }

        // Now handle yearly recurring transactions specially
        for (orig_date, tx) in &yearly_transactions {
            // Calculate the amount for this account
            let amount = if tx.target_account_id == account.id {
                tx.amount
            } else if Some(account.id) == tx.source_account_id {
                -tx.amount
            } else {
                Decimal::ZERO
            };

            // Skip transactions with zero amount
            if amount == Decimal::ZERO {
                continue;
            }

            trace!(
                "Processing yearly recurring transaction: date={}, amount={}, period={:?}",
                orig_date, amount, tx.period
            );

            // For yearly recurring transactions, we need to add them to all future months
            // within the date range, regardless of whether the original date is after today
            let day = orig_date.day();

            // Add the transaction to all months from today to end_date
            let start_month = today.month();
            let start_year = today.year();
            let end_month = end_date.month();
            let end_year = end_date.year();

            // Calculate total months to iterate through
            let total_months = (end_year - start_year) * 12 + (end_month as i32 - start_month as i32 + 1) as i32;

            for month_offset in 0..total_months {
                let year = start_year + (start_month as i32 + month_offset - 1) / 12;
                let month = ((start_month as i32 + month_offset - 1) % 12 + 1) as u32;

                // Create a date for this month, handling invalid dates (e.g., Feb 30)
                let future_date = NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| {
                    // Get the last day of the month
                    let last_day = NaiveDate::from_ymd_opt(year, month, 1)
                        .unwrap()
                        .checked_add_months(chrono::Months::new(1))
                        .unwrap()
                        .pred_opt()
                        .unwrap()
                        .day();
                    NaiveDate::from_ymd_opt(year, month, last_day).unwrap()
                });

                // Only add dates that are within our range
                if future_date <= end_date {
                    // Always add the yearly recurring transaction to this date
                    trace!(
                        "Adding yearly recurring transaction to future date: date={}, amount={}",
                        future_date, amount
                    );
                    all_transactions.push((future_date, amount));
                }
            }
        }

        // Add recurring income
        trace!("Processing recurring income for account {}", account.id);
        for (date, income) in recurring_income {
            // Include all income with dates on or after today
            // This ensures we account for future recurring income
            if date >= today && income.target_account_id == account.id {
                trace!(
                    "Adding unpaid recurring income: date={}, amount={}",
                    date, income.amount
                );
                all_transactions.push((date, income.amount));
            }
        }

        // Process transactions and update balances
        // Sort transactions by date
        all_transactions.sort_by(|a, b| a.0.cmp(&b.0));

        // Group transactions by date
        let mut transactions_by_date: HashMap<NaiveDate, Vec<Decimal>> = HashMap::new();
        for (date, amount) in all_transactions {
            transactions_by_date.entry(date).or_insert_with(Vec::new).push(amount);
        }

        // Process dates in chronological order
        let mut dates: Vec<NaiveDate> = transactions_by_date.keys().cloned().collect();
        dates.sort();

        // Keep track of the running balance for this account
        let mut running_balance = Decimal::ZERO;

        for date in dates {
            let amounts = transactions_by_date.get(&date).unwrap();
            let total_amount_for_date: Decimal = amounts.iter().sum();

            // Update the running balance
            running_balance += total_amount_for_date;

            let key = (account.id, date);
            let entry = unpaid_data.entry(key).or_insert(Decimal::ZERO);
            *entry = running_balance;  // Set the balance to the running total
            trace!(
                "Updated unpaid balance for account {} on {}: {} (total amount for date: {})",
                account.id, date, entry, total_amount_for_date
            );
        }

        // Propagate balances to dates without transactions
        // This ensures that each date has at least the balance from the previous date
        let mut all_dates: Vec<NaiveDate> = unpaid_data.keys()
            .filter(|(id, _)| *id == account.id)
            .map(|(_, date)| *date)
            .collect();
        all_dates.sort();

        let mut prev_balance = Decimal::ZERO;
        for date in all_dates {
            let key = (account.id, date);
            let entry = unpaid_data.get_mut(&key).unwrap();

            // If this date has a non-zero balance, update prev_balance
            if *entry != Decimal::ZERO {
                prev_balance = *entry;
            } else {
                // If this date has a zero balance, use the previous balance
                *entry = prev_balance;
                trace!(
                    "Propagated balance for account {} on {}: {}",
                    account.id, date, entry
                );
            }
        }
    }

    // Convert the HashMap to a DataFrame
    debug!("Converting unpaid recurring data to DataFrame");
    let mut account_ids = Vec::new();
    let mut dates = Vec::new();
    let mut balances = Vec::new();

    for ((account_id, date), balance) in unpaid_data {
        trace!(
            "Adding unpaid data point: account_id={}, date={}, balance={}",
            account_id, date, balance
        );
        account_ids.push(account_id);
        dates.push(date);
        balances.push(balance.to_string());
    }

    debug!(
        "Creating DataFrame with {} unpaid recurring data points",
        account_ids.len()
    );
    let df = DataFrame::new(vec![
        Series::new("account_id".into(), account_ids).into(),
        Series::new("date".into(), dates).into(),
        Series::new("balance".into(), balances).into(),
    ])?;

    info!(
        "Unpaid recurring computation completed successfully with {} data points",
        df.height()
    );
    Ok(df)
}

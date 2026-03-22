//! Financial metrics computation module.
//!
//! Provides per-account and cross-account metric calculations built on
//! top of the existing account state calculator and balance DataFrames.

use chrono::NaiveDate;
use model::entities::recurring_transaction;

pub mod account_metrics;
pub mod account_role;
pub mod cross_account_metrics;

/// Filters recurring transactions to only those active on the given date.
///
/// A transaction is active when `start_date <= date` and either `end_date`
/// is `None` (open-ended) or `end_date >= date`. Simulated transactions
/// (scenario what-if entries) are always excluded.
pub fn filter_active_recurring(
    transactions: &[recurring_transaction::Model],
    date: NaiveDate,
) -> Vec<&recurring_transaction::Model> {
    transactions
        .iter()
        .filter(|r| {
            !r.is_simulated
                && r.start_date <= date
                && r.end_date.map_or(true, |end| end >= date)
        })
        .collect()
}

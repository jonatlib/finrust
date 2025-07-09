pub mod account;
pub mod account_stats;
pub mod error;
pub mod tags;
pub mod transaction;

use chrono::{NaiveDate, Utc};
use account::{
    balance::BalanceCalculator,
    merge::MergeCalculator,
    unpaid_recurring::UnpaidRecurringCalculator,
    MergeMethod,
};

/// Returns a default pre-configured compute instance that will be used most of the time.
/// 
/// This function uses the provided date as "today" or the current date if none is provided.
/// It has the same configuration as the one used in the `test_scenario_merge_real` test.
pub fn default_compute(today: Option<NaiveDate>) -> MergeCalculator {
    // Create the today date
    let today = today.unwrap_or_else(|| Utc::now().date_naive());

    // Create the balance calculator
    let balance_calculator = BalanceCalculator::new_with_today(
        MergeMethod::FirstWins,
        today,
    );

    // Create the unpaid recurring calculator
    let unpaid_calculator = UnpaidRecurringCalculator::new_with_sum_merge(
        today,
        chrono::Duration::days(7),
    );

    // Create a merge calculator that combines both calculators
    // Use Sum merge method to sum the balances from both calculators
    MergeCalculator::new(
        vec![
            Box::new(balance_calculator),
            Box::new(unpaid_calculator),
        ],
        MergeMethod::Sum,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use account::testing::{ScenarioMergeReal, run_and_assert_scenario};
    use tokio;

    /// Test using the default compute with the real_merge scenario within range.
    /// This test verifies that the default compute works correctly with a scenario
    /// that is within the expected date range.
    #[tokio::test]
    async fn test_default_compute_within_range() {
        let scenario = ScenarioMergeReal::new();
        // Use the same date as in test_scenario_merge_real
        let today = NaiveDate::from_ymd_opt(2026, 06, 22).unwrap();
        let compute = default_compute(Some(today));

        run_and_assert_scenario(&scenario, &compute, true)
            .await
            .expect("Failed to run scenario within range");
    }

    /// Test using the default compute with the real_merge scenario outside range.
    /// This test verifies that the default compute works correctly with a scenario
    /// that is outside the expected date range.
    #[tokio::test]
    async fn test_default_compute_outside_range() {
        let scenario = ScenarioMergeReal::new();
        // Use the same date as in test_scenario_merge_real
        let today = NaiveDate::from_ymd_opt(2026, 06, 22).unwrap();
        let compute = default_compute(Some(today));

        run_and_assert_scenario(&scenario, &compute, false)
            .await
            .expect("Failed to run scenario outside range");
    }
}

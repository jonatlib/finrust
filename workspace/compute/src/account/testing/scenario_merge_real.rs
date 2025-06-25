use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::helpers::*;
use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};

pub struct ScenarioMergeReal {}

impl Default for ScenarioMergeReal {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioMergeReal {
    pub fn new() -> Self {
        Self {}
    }
}

macro_rules! date {
    ($year:expr, $month:expr, $day:expr) => {
        NaiveDate::from_ymd_opt($year, $month, $day).unwrap()
    };
}

macro_rules! expect {
    ($assert_results:ident, $account:ident, $year:expr, $month:expr, $day:expr, $amount:expr) => {
        $assert_results.push((
            $account.id,
            NaiveDate::from_ymd_opt($year, $month, $day).unwrap(),
            Decimal::new($amount * 100, 2),
        ))
    };
}

#[async_trait]
impl TestScenarioBuilder for ScenarioMergeReal {
    async fn get_scenario(&self) -> Result<TestScenario> {
        let mut assert_results: AssertResult = vec![];
        let db = setup_db().await?;

        let account1 = new_account(&db).await?;
        let account2 = new_account(&db).await?;

        /////////////////////////////////////////////////////////////////////////////////////////////////
        // Just test that manual account states are working

        new_manual_account_state(&db, &account1, date!(2025, 01, 01), 100_000).await?;
        new_manual_account_state(&db, &account2, date!(2025, 01, 01), 100_000).await?;

        expect!(assert_results, account1, 2025, 01, 01, 100_000);
        expect!(assert_results, account2, 2025, 01, 01, 100_000);

        new_manual_account_state(&db, &account1, date!(2025, 06, 01), 200_000).await?;
        new_manual_account_state(&db, &account2, date!(2025, 06, 01), 100_000).await?;

        expect!(assert_results, account1, 2025, 06, 10, 200_000);
        expect!(assert_results, account2, 2025, 06, 10, 100_000);

        /////////////////////////////////////////////////////////////////////////////////////////////////
        // Now add recurring transactions

        expect!(assert_results, account1, 2025, 10, 10, 200_000);
        expect!(assert_results, account2, 2025, 10, 10, 100_000);

        let r1 = new_recurring_transaction(&db, &account1, date!(2025, 10, 11), -1_000).await?;
        for index in (0u32..=2) {
            new_recurring_instance(&db, &r1, date!(2025, 10 + index, 11)).await?;

            expect!(assert_results, account1, 2025, 10 + index, 12, 200_000 - 1_000 * (1 + index as i64));
            expect!(assert_results, account2, 2025, 10 + index, 12, 100_000);
        }

        new_recurring_instance(&db, &r1, date!(2026, 01, 11)).await?;
        expect!(assert_results, account1, 2026, 01, 12, 200_000 - 1_000 * 4);
        expect!(assert_results, account2, 2026, 01, 12, 100_000);

        /////////////////////////////////////////////////////////////////////////////////////////////////
        // Now add another recurring but also check if it is not accounted it won't be charged

        let r2 = new_recurring_transaction(&db, &account2, date!(2026, 01, 12), -1_000).await?;
        expect!(assert_results, account1, 2026, 01, 13, 200_000 - 1_000 * 4);
        expect!(assert_results, account2, 2026, 01, 13, 100_000);

        new_recurring_instance(&db, &r2, date!(2026, 01, 14)).await?;
        expect!(assert_results, account1, 2026, 01, 15, 200_000 - 1_000 * 4);
        expect!(assert_results, account2, 2026, 01, 15, 100_000 - 1_000);

        /////////////////////////////////////////////////////////////////////////////////////////////////
        // Now test transfers between accounts

        new_one_off_account_transfer(&db, &account1, &account2, date!(2026, 01, 20), 1_000).await?;
        expect!(assert_results, account1, 2026, 01, 21, 200_000 - 1_000 * 4 - 1_000);
        expect!(assert_results, account2, 2026, 01, 21, 100_000 - 1_000 * 1 + 1_000);

        /////////////////////////////////////////////////////////////////////////////////////////////////
        // Now fastforward into future and check that
        // We assume that "today" is 2026-06-22

        for index in (0u32..6) {
            new_recurring_instance(&db, &r1, date!(2026, 01 + index, 22)).await?;
            new_recurring_instance(&db, &r2, date!(2026, 01 + index, 22)).await?;
        }

        /////////////////////////////////////////////////////////////////////////////////////////////////
        // And now we are in the future

        for index in (0u32..6) {
            expect!(assert_results, account1, 2026, 07 + index, 22, 189_000 - (1000 * index as i64));
            expect!(assert_results, account2, 2026, 07 + index, 22, 94_000 - (1000 * index as i64));
        }

        // Return the test scenario
        Ok((db, vec![account1, account2], assert_results))
    }
}

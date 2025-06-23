use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::helpers::*;
use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};

pub struct ScenarioMergeReal {}

impl ScenarioMergeReal {
    pub fn new() -> Self {
        Self {}
    }
}

macro_rules! date {
    ($year:expr, $month:expr, $day:expr) => {NaiveDate::from_ymd_opt($year, $month, $day).unwrap()};
}

macro_rules! expect {
    ($assert_results:ident, $account:ident, $year:expr, $month:expr, $day:expr, $amount:expr) => {
        $assert_results.push(
            (
                $account.id,
                NaiveDate::from_ymd_opt($year, $month, $day).unwrap(),
                Decimal::new($amount * 100, 2),
            ),
        )
    };
}

#[async_trait]
impl TestScenarioBuilder for ScenarioMergeReal {
    async fn get_scenario(&self) -> Result<TestScenario> {
        let mut assert_results: AssertResult = vec![];
        let db = setup_db().await?;

        let account1 = new_account(&db).await?;
        let account2 = new_account(&db).await?;

        new_manual_account_state(&db, &account1, date!(2025, 01, 01), 100_000).await?;
        new_manual_account_state(&db, &account2, date!(2025, 01, 01), 100_000).await?;

        expect!(assert_results, account1, 2025, 01, 01, 100_000);
        expect!(assert_results, account2, 2025, 01, 01, 100_000);

        expect!(assert_results, account1, 2025, 01, 10, 100_000);
        expect!(assert_results, account2, 2025, 01, 10, 100_000);


        // Return the test scenario
        Ok((db, vec![account1, account2], assert_results))
    }
}

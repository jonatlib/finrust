use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::DbErr;

use super::scenario_forecast_no_instances::ScenarioForecastNoInstances;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};

pub struct CustomScenarioForecastNoInstances {}

impl Default for CustomScenarioForecastNoInstances {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomScenarioForecastNoInstances {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for CustomScenarioForecastNoInstances {
    async fn get_scenario(&self) -> std::result::Result<TestScenario, sea_orm::DbErr> {
        // Get the base scenario
        let (db, accounts, assert_result) = ScenarioForecastNoInstances::new().get_scenario().await?;

        // Update the expected balances for April 1, April 15, and May 1
        let april_1_2023 = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap();
        let april_15_2023 = NaiveDate::from_ymd_opt(2023, 4, 15).unwrap();
        let may_1_2023 = NaiveDate::from_ymd_opt(2023, 5, 1).unwrap();

        let updated_assert_result: AssertResult = assert_result
            .into_iter()
            .map(|(id, date, balance)| {
                if date == april_1_2023 {
                    // Update the expected balance for April 1 to -$2200.00
                    (id, date, Decimal::new(-220000, 2))
                } else if date == april_15_2023 {
                    // Update the expected balance for April 15 to -$2200.00
                    (id, date, Decimal::new(-220000, 2))
                } else if date == may_1_2023 {
                    // Update the expected balance for May 1 to -$2900.00
                    (id, date, Decimal::new(-290000, 2))
                } else {
                    (id, date, balance)
                }
            })
            .collect();

        Ok((db, accounts, updated_assert_result))
    }
}
use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DbErr, Set};

use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};
use model::entities::{account, recurring_transaction};

pub struct ScenarioForecastNoInstances {}

impl ScenarioForecastNoInstances {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for ScenarioForecastNoInstances {
    async fn get_scenario(&self) -> Result<TestScenario, DbErr> {
        let db = setup_db().await?;

        // Create a test user first
        let user = model::entities::user::ActiveModel {
            id: Set(1),
            username: Set("test_user".to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create a test account
        let account = account::ActiveModel {
            name: Set("Test Account".to_string()),
            description: Set(Some("Account for forecast testing without instances".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1),
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Note: No manual account state is created for forecast testing

        // Initial date for transactions
        let initial_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let today = NaiveDate::from_ymd_opt(2023, 3, 15).unwrap(); // Set today to March 15, 2023

        // Create a recurring (monthly) transaction - e.g., rent payment
        // This transaction will not have any instances created
        let recurring_tx = recurring_transaction::ActiveModel {
            name: Set("Monthly Rent Without Instances".to_string()),
            description: Set(Some("Monthly rent payment without instances".to_string())),
            amount: Set(Decimal::new(-50000, 2)), // -$500.00
            start_date: Set(initial_date),
            end_date: Set(None), // Indefinite
            period: Set(recurring_transaction::RecurrencePeriod::Monthly),
            include_in_statistics: Set(true),
            target_account_id: Set(account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create a second recurring transaction that starts in the future
        // This one should be included in forecast calculations even without instances
        let future_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap();
        let future_recurring_tx = recurring_transaction::ActiveModel {
            name: Set("Future Subscription".to_string()),
            description: Set(Some("Future subscription payment".to_string())),
            amount: Set(Decimal::new(-20000, 2)), // -$200.00
            start_date: Set(future_date),
            end_date: Set(None), // Indefinite
            period: Set(recurring_transaction::RecurrencePeriod::Monthly),
            include_in_statistics: Set(true),
            target_account_id: Set(account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create assertions for different months
        // For forecast, we start with 0 balance and accumulate transactions
        //
        // March 15 (today): 0 balance
        // March 16: Past recurring transactions without instances are moved to today + future_offset
        //           So the rent payment (-$500) from Jan, Feb, and March (3 * -$500 = -$1500) should be here
        // April 1: -$1500 - $200 (future subscription) - $500 (April rent) = -$2200
        // April 15: -$2200
        // May 1: -$2200 - $500 (rent) - $200 (subscription) = -$2900
        let assert_results: AssertResult = vec![
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 3, 15).unwrap(),
                Decimal::new(0, 2), // $0.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 3, 16).unwrap(),
                Decimal::new(-150000, 2), // -$1500.00 (3 months of rent moved to today + 1 day)
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(),
                Decimal::new(-220000, 2), // -$2200.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 4, 15).unwrap(),
                Decimal::new(-220000, 2), // -$2200.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 5, 1).unwrap(),
                Decimal::new(-290000, 2), // -$2900.00
            ),
        ];

        // Return the test scenario
        Ok((db, vec![account], assert_results))
    }
}

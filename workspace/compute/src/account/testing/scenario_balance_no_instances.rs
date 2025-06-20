use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DbErr, Set};

use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};
use model::entities::{account, manual_account_state, recurring_transaction};

pub struct ScenarioBalanceNoInstances {}

impl ScenarioBalanceNoInstances {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for ScenarioBalanceNoInstances {
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
            description: Set(Some("Account for balance testing without instances".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1),
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create a manual account state (initial balance)
        let initial_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let initial_balance = Decimal::new(100000, 2); // $1,000.00

        let _manual_state = manual_account_state::ActiveModel {
            account_id: Set(account.id),
            date: Set(initial_date),
            amount: Set(initial_balance),
            ..Default::default()
        }
            .insert(&db)
            .await?;

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
        // This one should be included in balance calculations even without instances
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
        // January 31: Initial $1000, no transactions should be accounted for
        // February 28: Still $1000, no transactions should be accounted for
        // March 31: Still $1000, no transactions should be accounted for
        // April 1: $1000 - $500 (monthly rent) - $200 (future subscription) = $300
        // April 30: Still $300, no more transactions
        let assert_results: AssertResult = vec![
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
                Decimal::new(100000, 2), // $1000.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
                Decimal::new(100000, 2), // $1000.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                Decimal::new(100000, 2), // $1000.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(),
                Decimal::new(30000, 2), // $300.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 4, 30).unwrap(),
                Decimal::new(30000, 2), // $300.00
            ),
        ];

        // Return the test scenario
        Ok((db, vec![account], assert_results))
    }
}

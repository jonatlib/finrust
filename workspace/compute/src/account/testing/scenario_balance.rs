use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DbErr, Set};

use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};
use model::entities::{account, manual_account_state, one_off_transaction, recurring_transaction};

pub struct ScenarioBalance {}

impl ScenarioBalance {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for ScenarioBalance {
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
            description: Set(Some("Account for balance testing".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1), // Assuming user ID 1 exists
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
        let recurring_tx = recurring_transaction::ActiveModel {
            name: Set("Monthly Rent".to_string()),
            description: Set(Some("Monthly rent payment".to_string())),
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

        // Create a one-off transaction - e.g., a purchase
        let _one_off_tx = one_off_transaction::ActiveModel {
            name: Set("Grocery Shopping".to_string()),
            description: Set(Some("Monthly grocery shopping".to_string())),
            amount: Set(Decimal::new(-15000, 2)), // -$150.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 15).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create another one-off transaction - e.g., a bonus
        let _bonus_tx = one_off_transaction::ActiveModel {
            name: Set("Work Bonus".to_string()),
            description: Set(Some("Annual bonus".to_string())),
            amount: Set(Decimal::new(30000, 2)), // $300.00
            date: Set(NaiveDate::from_ymd_opt(2023, 3, 10).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        // Create assertions for 3 different months
        // January 31: Initial $1000 - $500 (rent) = $500
        // February 28: $500 - $500 (rent) - $150 (groceries) = -$150
        // March 31: -$150 - $500 (rent) + $300 (bonus) = -$350
        let assert_results: AssertResult = vec![
            (account.id, NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(), Decimal::new(50000, 2)),
            (account.id, NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(), Decimal::new(-15000, 2)),
            (account.id, NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(), Decimal::new(-35000, 2)),
        ];

        // Return the test scenario
        Ok((db, vec![account], assert_results))
    }
}

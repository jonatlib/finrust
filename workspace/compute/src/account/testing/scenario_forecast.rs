use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DbErr, Set};

use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};
use model::entities::{account, recurring_transaction, recurring_transaction_instance};

pub struct ScenarioForecast {}

impl ScenarioForecast {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for ScenarioForecast {
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
            description: Set(Some("Account for forecast testing".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1), // Assuming user ID 1 exists
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Note: No manual account state is created for forecast testing

        // Initial date for transactions
        let initial_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

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

        // Create recurring transaction instances for January, February, and March
        // January instance - paid
        let _jan_instance = recurring_transaction_instance::ActiveModel {
            recurring_transaction_id: Set(recurring_tx.id),
            status: Set(recurring_transaction_instance::InstanceStatus::Paid),
            due_date: Set(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()),
            expected_amount: Set(Decimal::new(-50000, 2)), // -$500.00
            paid_date: Set(Some(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap())),
            paid_amount: Set(Some(Decimal::new(-50000, 2))), // -$500.00
            reconciled_imported_transaction_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // February instance - paid
        let _feb_instance = recurring_transaction_instance::ActiveModel {
            recurring_transaction_id: Set(recurring_tx.id),
            status: Set(recurring_transaction_instance::InstanceStatus::Paid),
            due_date: Set(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap()),
            expected_amount: Set(Decimal::new(-50000, 2)), // -$500.00
            paid_date: Set(Some(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap())),
            paid_amount: Set(Some(Decimal::new(-50000, 2))), // -$500.00
            reconciled_imported_transaction_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // March instance - paid
        let _mar_instance = recurring_transaction_instance::ActiveModel {
            recurring_transaction_id: Set(recurring_tx.id),
            status: Set(recurring_transaction_instance::InstanceStatus::Paid),
            due_date: Set(NaiveDate::from_ymd_opt(2023, 3, 1).unwrap()),
            expected_amount: Set(Decimal::new(-50000, 2)), // -$500.00
            paid_date: Set(Some(NaiveDate::from_ymd_opt(2023, 3, 1).unwrap())),
            paid_amount: Set(Some(Decimal::new(-50000, 2))), // -$500.00
            reconciled_imported_transaction_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // April instance - pending (future)
        let _apr_instance = recurring_transaction_instance::ActiveModel {
            recurring_transaction_id: Set(recurring_tx.id),
            status: Set(recurring_transaction_instance::InstanceStatus::Pending),
            due_date: Set(NaiveDate::from_ymd_opt(2023, 4, 1).unwrap()),
            expected_amount: Set(Decimal::new(-50000, 2)), // -$500.00
            paid_date: Set(None),
            paid_amount: Set(None),
            reconciled_imported_transaction_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Note: No one-off transactions are created for this scenario
        // This ensures that both balance calculator and forecast calculator
        // will return the same results

        // Create assertions for 3 different months
        // For forecast, we start with 0 balance and accumulate transactions
        //
        // For both balance calculator and forecast calculator:
        // January 31: 0 - $500 (rent) = -$500
        // February 1: -$500 - $500 (rent) = -$1000 (monthly rent applied on the 1st)
        // February 28: -$1000 (no one-off transactions) = -$1000
        // March 31: -$1000 - $500 (rent) = -$1500
        // April 1: -$1500 - $500 (rent) = -$2000
        let assert_results: AssertResult = vec![
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
                Decimal::new(-50000, 2),
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 2, 01).unwrap(),
                Decimal::new(-100000, 2),
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
                Decimal::new(-100000, 2),
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                Decimal::new(-150000, 2),
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 4, 01).unwrap(),
                Decimal::new(-200000, 2),
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 4, 15).unwrap(),
                Decimal::new(-200000, 2),
            ),
        ];

        // Return the test scenario
        Ok((db, vec![account], assert_results))
    }
}

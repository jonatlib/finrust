use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DbErr, Set};

use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};
use model::entities::{account, one_off_transaction, recurring_transaction};

pub struct ScenarioReconciliationOutsideRange {}

impl ScenarioReconciliationOutsideRange {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for ScenarioReconciliationOutsideRange {
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
            description: Set(Some("Account for reconciliation testing".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1), // Assuming user ID 1 exists
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

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

        // Create a one-off transaction that is reconciled with the recurring transaction for February
        // Note: The date is different from the scheduled date (Feb 1) to test the reconciliation logic
        let one_off_tx_feb = one_off_transaction::ActiveModel {
            name: Set("February Rent".to_string()),
            description: Set(Some("February rent payment (reconciled)".to_string())),
            amount: Set(Decimal::new(-50000, 2)), // -$500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 3).unwrap()), // Paid on Feb 3 instead of Feb 1
            include_in_statistics: Set(true),
            target_account_id: Set(account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(recurring_tx.id)), // Link to recurring transaction
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create assertions for 3 different months
        // For forecast, we start with 0 balance and accumulate transactions
        //
        // January 31: 0 - $500 (rent) = -$500
        // February 3: -$500 - $500 (reconciled rent) = -$1000 (note: no duplicate on Feb 1)
        // February 28: -$1000 (no more transactions) = -$1000
        // March 1: -$1000 (no change when testing outside range with initial balance)
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
                NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
                Decimal::new(-50000, 2), // No change on Feb 1 because the rent is reconciled with Feb 3 payment
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 2, 3).unwrap(),
                Decimal::new(-100000, 2), // Rent payment on Feb 3
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
                Decimal::new(-100000, 2),
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
                Decimal::new(-100000, 2), // No change on Mar 1 when testing outside range
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                Decimal::new(-150000, 2),
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(),
                Decimal::new(-200000, 2), // April rent on the 1st
            ),
        ];

        // Return the test scenario
        Ok((db, vec![account], assert_results))
    }
}
use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DbErr, Set};

use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};
use model::entities::{account, manual_account_state, one_off_transaction, recurring_transaction};

pub struct ScenarioUnreconciledForecast {}

impl ScenarioUnreconciledForecast {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for ScenarioUnreconciledForecast {
    async fn get_scenario(&self) -> Result<TestScenario, DbErr> {
        let db = setup_db().await?;

        // Create a test user first
        let _user = model::entities::user::ActiveModel {
            id: Set(1),
            username: Set("test_user".to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create a test account
        let account = account::ActiveModel {
            name: Set("Test Account".to_string()),
            description: Set(Some("Account for unreconciled forecast testing".to_string())),
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

        // Create a recurring transaction with linked one-off transactions
        let reconciled_tx = recurring_transaction::ActiveModel {
            name: Set("Monthly Rent (Reconciled)".to_string()),
            description: Set(Some("Monthly rent payment with reconciliation".to_string())),
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

        // Create a recurring transaction without linked one-off transactions
        let _unreconciled_tx = recurring_transaction::ActiveModel {
            name: Set("Monthly Subscription (Unreconciled)".to_string()),
            description: Set(Some("Monthly subscription without reconciliation".to_string())),
            amount: Set(Decimal::new(-10000, 2)), // -$100.00
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

        // Create one-off transactions linked to the reconciled recurring transaction
        // January rent
        let _jan_rent = one_off_transaction::ActiveModel {
            name: Set("January Rent".to_string()),
            description: Set(Some("January rent payment".to_string())),
            amount: Set(Decimal::new(-50000, 2)), // -$500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(reconciled_tx.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // February rent
        let _feb_rent = one_off_transaction::ActiveModel {
            name: Set("February Rent".to_string()),
            description: Set(Some("February rent payment".to_string())),
            amount: Set(Decimal::new(-50000, 2)), // -$500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(reconciled_tx.id)),
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
            reconciled_recurring_transaction_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create assertions for the forecast model
        // For the forecast model, past unreconciled transactions are NOT moved forward to today
        // because they are not in the past relative to today (2022-12-31)
        // January 1: Initial $1000 - $500 (reconciled rent) - $100 (unreconciled subscription) = $400
        // January 31: $400 - $0 (no transactions on this day) = $400
        // February 1: $400 - $500 (reconciled rent) - $100 (unreconciled subscription) = -$200
        // February 28: -$200 - $150 (groceries) = -$350
        // March 1: -$350 - $500 (unreconciled rent) - $100 (unreconciled subscription) = -$950
        // March 31: -$950 - $0 (no transactions on this day) = -$950
        let forecast_assert_results: AssertResult = vec![
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 1, 31).unwrap(),
                Decimal::new(-60000, 2), // -$600.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(),
                Decimal::new(-120000, 2), // -$1200.00
            ),
            (
                account.id,
                NaiveDate::from_ymd_opt(2023, 3, 31).unwrap(),
                Decimal::new(-180000, 2), // -$1800.00
            ),
        ];

        // Return the test scenario with forecast assertions
        Ok((db, vec![account], forecast_assert_results))
    }
}

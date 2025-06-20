use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DbErr, Set};

use super::setup_db;
use crate::account::testing::{AssertResult, TestScenario, TestScenarioBuilder};
use model::entities::{account, manual_account_state, one_off_transaction, recurring_transaction};

pub struct ScenarioMultipleAccounts {}

impl ScenarioMultipleAccounts {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TestScenarioBuilder for ScenarioMultipleAccounts {
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

        // Create three test accounts
        let checking_account = account::ActiveModel {
            name: Set("Checking Account".to_string()),
            description: Set(Some("Primary checking account".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1),
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        let savings_account = account::ActiveModel {
            name: Set("Savings Account".to_string()),
            description: Set(Some("Long-term savings".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1),
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        let investment_account = account::ActiveModel {
            name: Set("Investment Account".to_string()),
            description: Set(Some("Stock investments".to_string())),
            currency_code: Set("USD".to_string()),
            owner_id: Set(1),
            include_in_statistics: Set(true),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Set initial balances for each account (on different days)
        let initial_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        // Checking account starts with $2,500.00 on Jan 1
        let _checking_initial = manual_account_state::ActiveModel {
            account_id: Set(checking_account.id),
            date: Set(initial_date),
            amount: Set(Decimal::new(250000, 2)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Savings account starts with $10,000.00 on Jan 3
        let _savings_initial = manual_account_state::ActiveModel {
            account_id: Set(savings_account.id),
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 3).unwrap()),
            amount: Set(Decimal::new(1000000, 2)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Investment account starts with $5,000.00 on Jan 5
        let _investment_initial = manual_account_state::ActiveModel {
            account_id: Set(investment_account.id),
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 5).unwrap()),
            amount: Set(Decimal::new(500000, 2)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create recurring transactions for each account

        // Checking account: Monthly salary deposit on the 15th
        let salary = recurring_transaction::ActiveModel {
            name: Set("Monthly Salary".to_string()),
            description: Set(Some("Monthly salary deposit".to_string())),
            amount: Set(Decimal::new(350000, 2)), // $3,500.00
            start_date: Set(NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()),
            end_date: Set(None), // Indefinite
            period: Set(recurring_transaction::RecurrencePeriod::Monthly),
            include_in_statistics: Set(true),
            target_account_id: Set(checking_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create one-off transactions for each month of the salary deposit
        // January salary
        let _jan_salary = one_off_transaction::ActiveModel {
            name: Set("January Salary".to_string()),
            description: Set(Some("January salary deposit".to_string())),
            amount: Set(Decimal::new(350000, 2)), // $3,500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 15).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(checking_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(salary.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // February salary
        let _feb_salary = one_off_transaction::ActiveModel {
            name: Set("February Salary".to_string()),
            description: Set(Some("February salary deposit".to_string())),
            amount: Set(Decimal::new(350000, 2)), // $3,500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 15).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(checking_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(salary.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Checking account: Monthly rent payment on the 1st
        let rent = recurring_transaction::ActiveModel {
            name: Set("Rent Payment".to_string()),
            description: Set(Some("Monthly rent".to_string())),
            amount: Set(Decimal::new(-120000, 2)), // -$1,200.00
            start_date: Set(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap()),
            end_date: Set(None), // Indefinite
            period: Set(recurring_transaction::RecurrencePeriod::Monthly),
            include_in_statistics: Set(true),
            target_account_id: Set(checking_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create one-off transactions for each month of the rent payment
        // February rent
        let _feb_rent = one_off_transaction::ActiveModel {
            name: Set("February Rent".to_string()),
            description: Set(Some("February rent payment".to_string())),
            amount: Set(Decimal::new(-120000, 2)), // -$1,200.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(checking_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(rent.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // March rent
        let _mar_rent = one_off_transaction::ActiveModel {
            name: Set("March Rent".to_string()),
            description: Set(Some("March rent payment".to_string())),
            amount: Set(Decimal::new(-120000, 2)), // -$1,200.00
            date: Set(NaiveDate::from_ymd_opt(2023, 3, 1).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(checking_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(rent.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Savings account: Monthly transfer from checking on the 20th
        let savings_transfer = recurring_transaction::ActiveModel {
            name: Set("Savings Transfer".to_string()),
            description: Set(Some("Monthly savings".to_string())),
            amount: Set(Decimal::new(50000, 2)), // $500.00
            start_date: Set(NaiveDate::from_ymd_opt(2023, 1, 20).unwrap()),
            end_date: Set(None), // Indefinite
            period: Set(recurring_transaction::RecurrencePeriod::Monthly),
            include_in_statistics: Set(true),
            target_account_id: Set(savings_account.id),
            source_account_id: Set(Some(checking_account.id)),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create one-off transactions for each month of the savings transfer
        // January savings transfer
        let _jan_savings = one_off_transaction::ActiveModel {
            name: Set("January Savings".to_string()),
            description: Set(Some("January savings transfer".to_string())),
            amount: Set(Decimal::new(50000, 2)), // $500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 20).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(savings_account.id),
            source_account_id: Set(Some(checking_account.id)),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(savings_transfer.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // February savings transfer
        let _feb_savings = one_off_transaction::ActiveModel {
            name: Set("February Savings".to_string()),
            description: Set(Some("February savings transfer".to_string())),
            amount: Set(Decimal::new(50000, 2)), // $500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(savings_account.id),
            source_account_id: Set(Some(checking_account.id)),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(savings_transfer.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // March savings transfer
        let _mar_savings = one_off_transaction::ActiveModel {
            name: Set("March Savings".to_string()),
            description: Set(Some("March savings transfer".to_string())),
            amount: Set(Decimal::new(50000, 2)), // $500.00
            date: Set(NaiveDate::from_ymd_opt(2023, 3, 20).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(savings_account.id),
            source_account_id: Set(Some(checking_account.id)),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(savings_transfer.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Investment account: Quarterly dividend on the 10th
        let dividend = recurring_transaction::ActiveModel {
            name: Set("Quarterly Dividend".to_string()),
            description: Set(Some("Stock dividends".to_string())),
            amount: Set(Decimal::new(25000, 2)), // $250.00
            start_date: Set(NaiveDate::from_ymd_opt(2023, 1, 10).unwrap()),
            end_date: Set(None), // Indefinite
            period: Set(recurring_transaction::RecurrencePeriod::Quarterly),
            include_in_statistics: Set(true),
            target_account_id: Set(investment_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create one-off transactions for each quarter of the dividend
        // January dividend (Q1)
        let _jan_dividend = one_off_transaction::ActiveModel {
            name: Set("January Dividend".to_string()),
            description: Set(Some("Q1 dividend payment".to_string())),
            amount: Set(Decimal::new(25000, 2)), // $250.00
            date: Set(NaiveDate::from_ymd_opt(2023, 1, 10).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(investment_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            reconciled_recurring_transaction_id: Set(Some(dividend.id)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create one-off transactions for each account on different days

        // Checking account: Car repair on Feb 10
        let _car_repair = one_off_transaction::ActiveModel {
            name: Set("Car Repair".to_string()),
            description: Set(Some("Unexpected car repair".to_string())),
            amount: Set(Decimal::new(-75000, 2)), // -$750.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 10).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(checking_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Savings account: Tax refund on Mar 15
        let _tax_refund = one_off_transaction::ActiveModel {
            name: Set("Tax Refund".to_string()),
            description: Set(Some("Annual tax refund".to_string())),
            amount: Set(Decimal::new(120000, 2)), // $1,200.00
            date: Set(NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(savings_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Investment account: Stock purchase on Feb 20
        let _stock_purchase = one_off_transaction::ActiveModel {
            name: Set("Stock Purchase".to_string()),
            description: Set(Some("Additional stock investment".to_string())),
            amount: Set(Decimal::new(-100000, 2)), // -$1,000.00
            date: Set(NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()),
            include_in_statistics: Set(true),
            target_account_id: Set(investment_account.id),
            source_account_id: Set(None),
            ledger_name: Set(None),
            linked_import_id: Set(None),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Create assertions for all accounts on various dates, including dates with no transactions
        let assert_results: AssertResult = vec![
            // Checking account assertions
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                Decimal::new(250000, 2), // Initial balance
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
                Decimal::new(600000, 2), // After salary deposit
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 20).unwrap(),
                Decimal::new(550000, 2), // After savings transfer
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 25).unwrap(),
                Decimal::new(550000, 2), // No change (date with no transactions)
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
                Decimal::new(430000, 2), // After rent payment
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 10).unwrap(),
                Decimal::new(355000, 2), // After car repair
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 15).unwrap(),
                Decimal::new(705000, 2), // After salary deposit
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
                Decimal::new(655000, 2), // After savings transfer
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
                Decimal::new(535000, 2), // After rent payment
            ),
            (
                checking_account.id,
                NaiveDate::from_ymd_opt(2023, 3, 5).unwrap(),
                Decimal::new(535000, 2), // No change (date with no transactions)
            ),
            // Savings account assertions
            (
                savings_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 3).unwrap(),
                Decimal::new(1000000, 2), // Initial balance
            ),
            (
                savings_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 20).unwrap(),
                Decimal::new(1050000, 2), // After transfer from checking
            ),
            (
                savings_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 5).unwrap(),
                Decimal::new(1050000, 2), // No change (date with no transactions)
            ),
            (
                savings_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
                Decimal::new(1100000, 2), // After transfer from checking
            ),
            (
                savings_account.id,
                NaiveDate::from_ymd_opt(2023, 3, 15).unwrap(),
                Decimal::new(1220000, 2), // After tax refund
            ),
            (
                savings_account.id,
                NaiveDate::from_ymd_opt(2023, 3, 20).unwrap(),
                Decimal::new(1270000, 2), // After transfer from checking
            ),
            // Investment account assertions
            (
                investment_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 5).unwrap(),
                Decimal::new(500000, 2), // Initial balance
            ),
            (
                investment_account.id,
                NaiveDate::from_ymd_opt(2023, 1, 10).unwrap(),
                Decimal::new(525000, 2), // After quarterly dividend
            ),
            (
                investment_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 15).unwrap(),
                Decimal::new(525000, 2), // No change (date with no transactions)
            ),
            (
                investment_account.id,
                NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
                Decimal::new(425000, 2), // After stock purchase
            ),
            (
                investment_account.id,
                NaiveDate::from_ymd_opt(2023, 3, 25).unwrap(),
                Decimal::new(425000, 2), // No change (date with no transactions)
            ),
            (
                investment_account.id,
                NaiveDate::from_ymd_opt(2023, 4, 10).unwrap(),
                Decimal::new(425000, 2), // No quarterly dividend applied in balance model
            ),
        ];

        // Return the test scenario with all three accounts
        Ok((
            db,
            vec![checking_account, savings_account, investment_account],
            assert_results,
        ))
    }
}

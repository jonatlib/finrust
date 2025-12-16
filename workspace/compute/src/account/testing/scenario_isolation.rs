//! Test suite for scenario isolation behavior.
//!
//! This module tests that simulated transactions (scenarios) are properly isolated
//! from real transactions and that they can be selectively included in calculations.

use chrono::NaiveDate;
use migration::{Migrator, MigratorTrait};
use model::entities::{account, one_off_transaction, scenario, user};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};

use crate::BalanceCalculator;
use crate::account::MergeMethod;
use crate::error::Result;
use crate::AccountStateCalculator;

/// Helper to set up a test database with migrations
async fn setup_db() -> Result<DatabaseConnection> {
    let db = Database::connect("sqlite::memory:").await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}

/// Helper to create a test user
async fn create_test_user(db: &DatabaseConnection, username: &str) -> Result<user::Model> {
    let user = user::ActiveModel {
        username: Set(username.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(user)
}

/// Helper to create a test account
async fn create_test_account(
    db: &DatabaseConnection,
    name: &str,
    owner_id: i32,
) -> Result<account::Model> {
    let account = account::ActiveModel {
        name: Set(name.to_string()),
        description: Set(Some(format!("{} account", name))),
        currency_code: Set("USD".to_string()),
        owner_id: Set(owner_id),
        include_in_statistics: Set(true),
        ledger_name: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(account)
}

/// Helper to create a manual account state
async fn create_manual_state(
    db: &DatabaseConnection,
    account_id: i32,
    date: NaiveDate,
    amount: Decimal,
) -> Result<()> {
    model::entities::manual_account_state::ActiveModel {
        account_id: Set(account_id),
        date: Set(date),
        amount: Set(amount),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Helper to create a scenario
async fn create_scenario(
    db: &DatabaseConnection,
    name: &str,
    description: Option<&str>,
) -> Result<scenario::Model> {
    let scenario = scenario::ActiveModel {
        name: Set(name.to_string()),
        description: Set(description.map(|s| s.to_string())),
        created_at: Set(chrono::Local::now().naive_local()),
        is_active: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(scenario)
}

/// Helper to create a one-off transaction
async fn create_transaction(
    db: &DatabaseConnection,
    name: &str,
    account_id: i32,
    amount: Decimal,
    date: NaiveDate,
    scenario_id: Option<i32>,
    is_simulated: bool,
) -> Result<one_off_transaction::Model> {
    let tx = one_off_transaction::ActiveModel {
        name: Set(name.to_string()),
        description: Set(Some(format!("{} transaction", name))),
        amount: Set(amount),
        date: Set(date),
        include_in_statistics: Set(true),
        target_account_id: Set(account_id),
        source_account_id: Set(None),
        category_id: Set(None),
        ledger_name: Set(None),
        linked_import_id: Set(None),
        scenario_id: Set(scenario_id),
        is_simulated: Set(is_simulated),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(tx)
}

/// Test Case 1: "Ghost Isolation"
///
/// Setup: Create 1 Real transaction (-$100), 1 Scenario A transaction (-$500), 1 Scenario B transaction (-$200).
/// Assert: StandardCalculator sees only -$100.
/// Assert: ScenarioCalculator(A) sees -$600 (-100 real + -500 ghost).
/// Assert: ScenarioCalculator(B) sees -$300 (-100 real + -200 ghost).
#[tokio::test]
async fn test_ghost_isolation() -> Result<()> {
    let db = setup_db().await?;

    // Create user and account
    let user = create_test_user(&db, "test_user").await?;
    let account = create_test_account(&db, "Checking", user.id).await?;

    // Create manual state with $1000 starting balance on Jan 1
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    create_manual_state(&db, account.id, start_date, Decimal::new(100000, 2)).await?;

    // Create scenarios
    let scenario_a = create_scenario(&db, "Buy Tesla", Some("Purchase a Tesla vehicle")).await?;
    let scenario_b = create_scenario(&db, "Buy Toyota", Some("Purchase a Toyota vehicle")).await?;

    // Create transactions on Jan 15
    let tx_date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

    // Real transaction: -$100
    create_transaction(
        &db,
        "Groceries",
        account.id,
        Decimal::new(-10000, 2),
        tx_date,
        None,
        false,
    )
    .await?;

    // Scenario A transaction: -$500
    create_transaction(
        &db,
        "Tesla Down Payment",
        account.id,
        Decimal::new(-50000, 2),
        tx_date,
        Some(scenario_a.id),
        true,
    )
    .await?;

    // Scenario B transaction: -$200
    create_transaction(
        &db,
        "Toyota Down Payment",
        account.id,
        Decimal::new(-20000, 2),
        tx_date,
        Some(scenario_b.id),
        true,
    )
    .await?;

    // Compute balance with Standard Calculator (no scenario)
    let end_date = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
    let today = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();

    let standard_calc = BalanceCalculator::new_with_today(MergeMethod::FirstWins, today);
    let standard_result = standard_calc
        .compute_account_state(&db, &[account.clone()], start_date, end_date)
        .await?;

    // Standard calculator should see only the real transaction: $1000 - $100 = $900
    let standard_balance = standard_result
        .column("balance")?
        .str()?
        .get(standard_result.height() - 1)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert_eq!(standard_balance, 900.0, "Standard calculator should show $900");

    // Compute balance with Scenario A Calculator
    let scenario_a_calc = BalanceCalculator::new_with_today_and_scenario(
        MergeMethod::FirstWins,
        today,
        scenario_a.id,
    );
    let scenario_a_result = scenario_a_calc
        .compute_account_state(&db, &[account.clone()], start_date, end_date)
        .await?;

    // Scenario A calculator should see real + scenario A: $1000 - $100 - $500 = $400
    let scenario_a_balance = scenario_a_result
        .column("balance")?
        .str()?
        .get(scenario_a_result.height() - 1)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert_eq!(
        scenario_a_balance, 400.0,
        "Scenario A calculator should show $400"
    );

    // Compute balance with Scenario B Calculator
    let scenario_b_calc = BalanceCalculator::new_with_today_and_scenario(
        MergeMethod::FirstWins,
        today,
        scenario_b.id,
    );
    let scenario_b_result = scenario_b_calc
        .compute_account_state(&db, &[account.clone()], start_date, end_date)
        .await?;

    // Scenario B calculator should see real + scenario B: $1000 - $100 - $200 = $700
    let scenario_b_balance = scenario_b_result
        .column("balance")?
        .str()?
        .get(scenario_b_result.height() - 1)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert_eq!(
        scenario_b_balance, 700.0,
        "Scenario B calculator should show $700"
    );

    Ok(())
}

/// Test Case 2: "The Application"
///
/// Action: Simulate "Applying" Scenario A (update DB to set is_simulated = false).
/// Assert: StandardCalculator now sees -$600.
#[tokio::test]
async fn test_scenario_application() -> Result<()> {
    let db = setup_db().await?;

    // Create user and account
    let user = create_test_user(&db, "test_user").await?;
    let account = create_test_account(&db, "Checking", user.id).await?;

    // Create manual state with $1000 starting balance on Jan 1
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    create_manual_state(&db, account.id, start_date, Decimal::new(100000, 2)).await?;

    // Create scenario A
    let scenario_a = create_scenario(&db, "Buy Tesla", Some("Purchase a Tesla vehicle")).await?;

    // Create transactions on Jan 15
    let tx_date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

    // Real transaction: -$100
    create_transaction(
        &db,
        "Groceries",
        account.id,
        Decimal::new(-10000, 2),
        tx_date,
        None,
        false,
    )
    .await?;

    // Scenario A transaction: -$500 (simulated)
    let scenario_tx = create_transaction(
        &db,
        "Tesla Down Payment",
        account.id,
        Decimal::new(-50000, 2),
        tx_date,
        Some(scenario_a.id),
        true,
    )
    .await?;

    // Verify initial state: Standard calculator should see only real transaction
    let end_date = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
    let today = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();

    let standard_calc = BalanceCalculator::new_with_today(MergeMethod::FirstWins, today);
    let initial_result = standard_calc
        .compute_account_state(&db, &[account.clone()], start_date, end_date)
        .await?;

    let initial_balance = initial_result
        .column("balance")?
        .str()?
        .get(initial_result.height() - 1)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert_eq!(
        initial_balance, 900.0,
        "Before application, standard calculator should show $900"
    );

    // Apply Scenario A: Set is_simulated to false
    use sea_orm::EntityTrait;
    let mut active_tx: one_off_transaction::ActiveModel = scenario_tx.into();
    active_tx.is_simulated = Set(false);
    active_tx.update(&db).await?;

    // Verify after application: Standard calculator should now see both transactions
    let final_result = standard_calc
        .compute_account_state(&db, &[account.clone()], start_date, end_date)
        .await?;

    let final_balance = final_result
        .column("balance")?
        .str()?
        .get(final_result.height() - 1)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert_eq!(
        final_balance, 400.0,
        "After application, standard calculator should show $400"
    );

    Ok(())
}

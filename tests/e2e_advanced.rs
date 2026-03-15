mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use chrono::NaiveDate;
use common::setup_test_app;
use finrust::handlers::accounts::{AccountKind, CreateAccountRequest};
use finrust::handlers::manual_account_states::CreateManualAccountStateRequest;
use finrust::handlers::recurring_income::CreateRecurringIncomeRequest;
use finrust::handlers::scenarios::CreateScenarioRequest;
use finrust::handlers::transactions::{
    CreateImportedTransactionRequest, CreateRecurringInstanceRequest,
    CreateRecurringTransactionRequest, CreateTransactionRequest,
};
use finrust::schemas::{ApiResponse, TimeseriesQuery};
use rust_decimal::Decimal;

#[tokio::test]
async fn test_e2e_advanced_scenario() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    eprintln!("=== Phase 1: Setup accounts ===");
    // ── Phase 1: Setup accounts (owner_id=1) ──

    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: "Main".to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: None,
            ledger_name: None,
            account_kind: Some(AccountKind::RealAccount),
            target_amount: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let main_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: "Emergency Fund".to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: None,
            ledger_name: None,
            account_kind: Some(AccountKind::Savings),
            target_amount: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let emergency_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: "Stocks".to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: None,
            ledger_name: None,
            account_kind: Some(AccountKind::Investment),
            target_amount: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let stocks_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: "Car Loan".to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: None,
            ledger_name: None,
            account_kind: Some(AccountKind::Debt),
            target_amount: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let _car_loan_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: "Vacation Fund".to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: None,
            ledger_name: None,
            account_kind: Some(AccountKind::Goal),
            target_amount: Some(Decimal::new(50000, 0)),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let vacation_id = body.data["id"].as_i64().unwrap() as i32;

    eprintln!("=== Phase 2: Create tags ===");
    // ── Phase 2: Create tags ──

    let resp = server
        .post("/api/v1/tags")
        .json(&serde_json::json!({
            "name": "personal",
            "description": null,
            "parent_id": null,
            "ledger_name": null
        }))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let personal_tag_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/tags")
        .json(&serde_json::json!({
            "name": "work",
            "description": null,
            "parent_id": null,
            "ledger_name": null
        }))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let _work_tag_id = body.data["id"].as_i64().unwrap() as i32;

    eprintln!("=== Phase 3: Recurring transactions ===");
    // ── Phase 3: Multiple recurring transactions with different periods ──

    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&CreateRecurringTransactionRequest {
            name: "Weekly Groceries".to_string(),
            description: None,
            amount: Decimal::new(-1000, 0),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(),
            end_date: None,
            period: "Weekly".to_string(),
            include_in_statistics: None,
            target_account_id: main_id,
            source_account_id: None,
            ledger_name: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let _groceries_rec_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&CreateRecurringTransactionRequest {
            name: "Monthly Rent".to_string(),
            description: None,
            amount: Decimal::new(-15000, 0),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: None,
            period: "Monthly".to_string(),
            include_in_statistics: None,
            target_account_id: main_id,
            source_account_id: None,
            ledger_name: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let rent_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&CreateRecurringTransactionRequest {
            name: "Quarterly Insurance".to_string(),
            description: None,
            amount: Decimal::new(-5000, 0),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: None,
            period: "Quarterly".to_string(),
            include_in_statistics: None,
            target_account_id: main_id,
            source_account_id: None,
            ledger_name: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&CreateRecurringTransactionRequest {
            name: "Monthly Car Payment".to_string(),
            description: None,
            amount: Decimal::new(-3000, 0),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: None,
            period: "Monthly".to_string(),
            include_in_statistics: None,
            target_account_id: main_id,
            source_account_id: None,
            ledger_name: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let car_payment_id = body.data["id"].as_i64().unwrap() as i32;

    eprintln!("=== Phase 4: Recurring income ===");
    // ── Phase 4: Recurring income ──

    let resp = server
        .post("/api/v1/recurring-incomes")
        .json(&CreateRecurringIncomeRequest {
            name: "Salary".to_string(),
            description: None,
            amount: Decimal::new(60000, 0),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: None,
            period: "Monthly".to_string(),
            include_in_statistics: None,
            target_account_id: main_id,
            source_name: None,
            ledger_name: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let salary_id = body.data["id"].as_i64().unwrap() as i32;

    eprintln!("=== Phase 5: Bulk create instances ===");
    // ── Phase 5: Bulk create instances ──

    // Create rent instances for Jan and Feb manually
    let resp = server
        .post(&format!(
            "/api/v1/recurring-transactions/{rent_id}/instances"
        ))
        .json(&CreateRecurringInstanceRequest {
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            amount: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    let resp = server
        .post(&format!(
            "/api/v1/recurring-transactions/{rent_id}/instances"
        ))
        .json(&CreateRecurringInstanceRequest {
            date: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            amount: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Bulk create: rent March + car payment Jan & Feb
    let resp = server
        .post("/api/v1/recurring-transactions/bulk-create-instances")
        .json(&serde_json::json!({
            "instances": [
                {
                    "recurring_transaction_id": rent_id,
                    "due_date": "2025-03-01",
                    "instance_id": null
                },
                {
                    "recurring_transaction_id": car_payment_id,
                    "due_date": "2025-01-01",
                    "instance_id": null
                },
                {
                    "recurring_transaction_id": car_payment_id,
                    "due_date": "2025-02-01",
                    "instance_id": null
                }
            ],
            "mark_as_paid": true
        }))
        .await;
    resp.assert_status_ok();
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let created = body.data["created_count"].as_i64().unwrap_or(0);
    let updated = body.data["updated_count"].as_i64().unwrap_or(0);
    assert!(created + updated > 0);

    eprintln!("=== Phase 6: Imported transactions ===");
    // ── Phase 6: Imported transactions ──

    let resp = server
        .post("/api/v1/imported-transactions")
        .json(&CreateImportedTransactionRequest {
            account_id: main_id,
            date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            description: "Supermarket purchase".to_string(),
            amount: Decimal::new(-500, 0),
            import_hash: "hash_001".to_string(),
            raw_data: None,
            category_id: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let import1_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/imported-transactions")
        .json(&CreateImportedTransactionRequest {
            account_id: main_id,
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            description: "Gas station".to_string(),
            amount: Decimal::new(-800, 0),
            import_hash: "hash_002".to_string(),
            raw_data: None,
            category_id: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    let resp = server
        .post("/api/v1/imported-transactions")
        .json(&CreateImportedTransactionRequest {
            account_id: main_id,
            date: NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
            description: "Online shop".to_string(),
            amount: Decimal::new(-1200, 0),
            import_hash: "hash_003".to_string(),
            raw_data: None,
            category_id: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Verify 3 imported transactions for the account
    let resp = server
        .get(&format!(
            "/api/v1/imported-transactions?account_id={main_id}"
        ))
        .await;
    resp.assert_status_ok();
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(body.data.len(), 3);

    // Verify unreconciled filter
    let resp = server
        .get("/api/v1/imported-transactions?reconciled=false")
        .await;
    resp.assert_status_ok();
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert!(body.data.len() >= 3);

    eprintln!("=== Phase 7: Reconcile imported transaction ===");
    // ── Phase 7: Reconcile imported transaction ──

    // Create a matching one-off transaction
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Supermarket purchase".to_string(),
            description: Some("Matched import".to_string()),
            amount: Decimal::new(-500, 0),
            date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            include_in_statistics: None,
            target_account_id: main_id,
            source_account_id: None,
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let matching_txn_id = body.data["id"].as_i64().unwrap() as i32;

    // Reconcile imported transaction with the one-off
    let resp = server
        .post(&format!(
            "/api/v1/imported-transactions/{import1_id}/reconcile"
        ))
        .json(&serde_json::json!({
            "transaction_type": "OneOff",
            "transaction_id": matching_txn_id
        }))
        .await;
    resp.assert_status_ok();
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(body.data["reconciled_transaction_type"], "OneOff");

    // Clear reconciliation
    let resp = server
        .delete(&format!(
            "/api/v1/imported-transactions/{import1_id}/reconcile"
        ))
        .await;
    resp.assert_status_ok();
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert!(body.data["reconciled_transaction_type"].is_null());

    eprintln!("=== Phase 8: Scenarios ===");
    // ── Phase 8: Scenarios ──

    eprintln!("  Phase 8a: Creating scenario...");
    let resp = server
        .post("/api/v1/scenarios")
        .json(&CreateScenarioRequest {
            name: "Buy new car".to_string(),
            description: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let scenario_id = body.data["id"].as_i64().unwrap() as i32;
    eprintln!("  Phase 8b: Scenario created, id={}", scenario_id);

    // Add simulated transaction to the scenario
    eprintln!("  Phase 8c: Adding simulated transaction...");
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Buy new car".to_string(),
            description: Some("Large purchase".to_string()),
            amount: Decimal::new(-300000, 0),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            include_in_statistics: None,
            target_account_id: main_id,
            source_account_id: None,
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: Some(scenario_id),
            is_simulated: Some(true),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    eprintln!("  Phase 8d: Simulated transaction created");

    // Verify timeseries WITH scenario
    eprintln!("  Phase 8e: Querying timeseries with scenario...");
    let ts_query = TimeseriesQuery {
        start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        include_ignored: false,
        scenario_id: Some(scenario_id),
    };
    let resp = server
        .get("/api/v1/accounts/timeseries")
        .add_query_params(&ts_query)
        .await;
    resp.assert_status_ok();
    eprintln!("  Phase 8f: Parsing timeseries response...");
    let body: ApiResponse<::common::AccountStateTimeseries> = resp.json();
    assert!(body.success);
    eprintln!("  Phase 8g: Timeseries with scenario verified");

    eprintln!("=== Phase 9: Complex money movement ===");
    // ── Phase 9: Complex money movement ──

    let tx_date = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();

    // Transfer 10000 from main to emergency fund
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Emergency fund deposit".to_string(),
            description: None,
            amount: Decimal::new(10000, 0),
            date: tx_date,
            include_in_statistics: None,
            target_account_id: emergency_id,
            source_account_id: Some(main_id),
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Transfer 5000 from main to stocks
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Stock investment".to_string(),
            description: None,
            amount: Decimal::new(5000, 0),
            date: tx_date,
            include_in_statistics: None,
            target_account_id: stocks_id,
            source_account_id: Some(main_id),
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Transfer 2000 from main to vacation fund
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Vacation savings".to_string(),
            description: None,
            amount: Decimal::new(2000, 0),
            date: tx_date,
            include_in_statistics: None,
            target_account_id: vacation_id,
            source_account_id: Some(main_id),
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    eprintln!("=== Phase 10: Manual account states ===");
    // ── Phase 10: Manual account states ──

    let resp = server
        .post(&format!("/api/v1/accounts/{main_id}/manual-states"))
        .json(&CreateManualAccountStateRequest {
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            amount: Decimal::new(100000, 0),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    let resp = server
        .post(&format!("/api/v1/accounts/{main_id}/manual-states"))
        .json(&CreateManualAccountStateRequest {
            date: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            amount: Decimal::new(85000, 0),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    // Verify 2 manual states
    let resp = server
        .get(&format!("/api/v1/accounts/{main_id}/manual-states"))
        .await;
    resp.assert_status_ok();
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(body.data.len(), 2);

    eprintln!("=== Phase 11: Verify statistics, timeseries, metrics ===");
    // ── Phase 11: Verify all statistics, timeseries, metrics ──

    let resp = server.get("/api/v1/accounts/statistics").await;
    resp.assert_status_ok();

    let ts_query = TimeseriesQuery {
        start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        include_ignored: false,
        scenario_id: None,
    };
    let resp = server
        .get("/api/v1/accounts/timeseries")
        .add_query_params(&ts_query)
        .await;
    resp.assert_status_ok();

    let resp = server.get("/api/v1/metrics/dashboard").await;
    resp.assert_status_ok();

    let resp = server
        .get(&format!("/api/v1/accounts/{main_id}/metrics"))
        .await;
    resp.assert_status_ok();

    let resp = server
        .get(&format!(
            "/api/v1/accounts/{main_id}/monthly-min-balance"
        ))
        .await;
    resp.assert_status_ok();

    eprintln!("=== Phase 12: CRUD completeness ===");
    // ── Phase 12: CRUD completeness ──

    // Update scenario
    let resp = server
        .put(&format!("/api/v1/scenarios/{scenario_id}"))
        .json(&serde_json::json!({
            "name": "Buy used car",
            "description": "Changed plan",
            "is_active": false
        }))
        .await;
    resp.assert_status_ok();
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(body.data["name"], "Buy used car");

    // Delete scenario
    let resp = server
        .delete(&format!("/api/v1/scenarios/{scenario_id}"))
        .await;
    let status = resp.status_code().as_u16();
    assert!(status == 200 || status == 204);

    // Verify scenario is gone
    let resp = server
        .get(&format!("/api/v1/scenarios/{scenario_id}"))
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);

    // Verify recurring income exists
    let resp = server.get("/api/v1/recurring-incomes").await;
    resp.assert_status_ok();
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert!(body.data.iter().any(|ri| ri["name"] == "Salary"));

    // Update recurring income
    let resp = server
        .put(&format!("/api/v1/recurring-incomes/{salary_id}"))
        .json(&serde_json::json!({
            "amount": "65000"
        }))
        .await;
    resp.assert_status_ok();

    // Delete recurring income
    let resp = server
        .delete(&format!("/api/v1/recurring-incomes/{salary_id}"))
        .await;
    let status = resp.status_code().as_u16();
    assert!(status == 200 || status == 204);

    // Delete a tag
    let resp = server
        .delete(&format!("/api/v1/tags/{personal_tag_id}"))
        .await;
    let status = resp.status_code().as_u16();
    assert!(status == 200 || status == 204);
}

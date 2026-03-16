mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use chrono::NaiveDate;
use common::setup_test_app;
use finrust::handlers::accounts::{AccountKind, CreateAccountRequest};
use finrust::handlers::categories::CreateCategoryRequest;
use finrust::handlers::transactions::CreateTransactionRequest;
use finrust::handlers::users::CreateUserRequest;
use finrust::schemas::{ApiResponse, TimeseriesQuery};
use rust_decimal::Decimal;
use ::common::AccountStateTimeseries;

#[tokio::test]
async fn test_e2e_basic_scenario() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    // ── Step 1: Health check ──
    let resp = server.get("/health").await;
    resp.assert_status(StatusCode::OK);

    // ── Step 2: Create user "Alice" ──
    let resp = server
        .post("/api/v1/users")
        .json(&CreateUserRequest {
            username: "alice".to_string(),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let alice_user_id = body.data["id"].as_i64().unwrap() as i32;

    // ── Step 3: Create 2 accounts ──
    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: "Checking".to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: alice_user_id,
            include_in_statistics: None,
            ledger_name: None,
            account_kind: Some(AccountKind::RealAccount),
            target_amount: None,
            color: None,
            is_liquid: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let checking_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: "Savings".to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: alice_user_id,
            include_in_statistics: None,
            ledger_name: None,
            account_kind: Some(AccountKind::Savings),
            target_amount: None,
            color: None,
            is_liquid: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let savings_id = body.data["id"].as_i64().unwrap() as i32;

    // ── Step 4: Verify accounts list ──
    let resp = server.get("/api/v1/accounts").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(body.data.len(), 2);

    // ── Step 5: Create category "Groceries" ──
    let resp = server
        .post("/api/v1/categories")
        .json(&CreateCategoryRequest {
            name: "Groceries".to_string(),
            description: Some("Food shopping".to_string()),
            parent_id: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let groceries_id = body.data["id"].as_i64().unwrap() as i32;

    // ── Step 6: Create transactions (all on 2026-01-15) ──
    let tx_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();

    // Salary: +50000 into checking (income)
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Salary".to_string(),
            description: Some("Monthly salary".to_string()),
            amount: Decimal::new(50000, 0),
            date: tx_date,
            include_in_statistics: None,
            target_account_id: checking_id,
            source_account_id: None,
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Groceries: -2000 from checking (expense)
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Groceries".to_string(),
            description: Some("Weekly groceries".to_string()),
            amount: Decimal::new(-2000, 0),
            date: tx_date,
            include_in_statistics: None,
            target_account_id: checking_id,
            source_account_id: None,
            ledger_name: None,
            linked_import_id: None,
            category_id: Some(groceries_id),
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Utilities: -3000 from checking (expense)
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Utilities".to_string(),
            description: Some("Electric and water".to_string()),
            amount: Decimal::new(-3000, 0),
            date: tx_date,
            include_in_statistics: None,
            target_account_id: checking_id,
            source_account_id: None,
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Transfer to savings: 10000 from checking -> savings
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: "Transfer to savings".to_string(),
            description: None,
            amount: Decimal::new(10000, 0),
            date: tx_date,
            include_in_statistics: None,
            target_account_id: savings_id,
            source_account_id: Some(checking_id),
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: None,
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // ── Step 7: Verify transactions list ──
    let resp = server.get("/api/v1/transactions").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(body.data.len(), 4);

    let resp = server
        .get(&format!("/api/v1/accounts/{checking_id}/transactions"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(body.data.len(), 4);

    // ── Step 8: Verify statistics ──
    let resp = server
        .get(&format!("/api/v1/accounts/{checking_id}/statistics"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    let resp = server
        .get(&format!("/api/v1/accounts/{savings_id}/statistics"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    let resp = server.get("/api/v1/accounts/statistics").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert!(body.data.len() >= 2);

    // ── Step 9: Verify timeseries ──
    let ts_query = TimeseriesQuery {
        start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        include_ignored: false,
        scenario_id: None,
    };

    let resp = server
        .get(&format!("/api/v1/accounts/{checking_id}/timeseries"))
        .add_query_params(&ts_query)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<AccountStateTimeseries> = resp.json();
    assert!(body.success);
    assert!(!body.data.data_points.is_empty());

    let last_point = body
        .data
        .data_points
        .iter()
        .filter(|p| p.account_id == checking_id)
        .max_by_key(|p| p.date)
        .expect("should have at least one data point for checking");
    assert_eq!(last_point.balance, Decimal::new(35000, 0));

    // ── Step 10: Verify dashboard metrics ──
    let resp = server.get("/api/v1/metrics/dashboard").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(body.data["total_net_worth"], "45000");

    // ── Step 11: Verify account metrics ──
    let resp = server
        .get(&format!("/api/v1/accounts/{checking_id}/metrics"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(body.data["current_balance"], "35000");

    // ── Step 12: Verify monthly min balance ──
    let resp = server
        .get(&format!(
            "/api/v1/accounts/{checking_id}/monthly-min-balance?months=12"
        ))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert!(body.data["data_points"].as_array().is_some());

    // ── Step 13: Verify category stats ──
    let resp = server
        .get("/api/v1/categories/stats?start_date=2026-01-01&end_date=2026-12-31")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);

    let groceries_stat = body
        .data
        .iter()
        .find(|c| c["category_name"] == "Groceries")
        .expect("should have Groceries in category stats");
    assert_eq!(groceries_stat["own_total"], "-2000");
}

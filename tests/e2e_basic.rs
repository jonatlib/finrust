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

    // ── Step 8: Verify statistics with frozen values ──
    use std::str::FromStr;
    let dec = |s: &str| Decimal::from_str(s).unwrap();
    let json_dec = |v: &serde_json::Value| -> Decimal {
        match v {
            serde_json::Value::String(s) => Decimal::from_str(s).unwrap(),
            serde_json::Value::Number(n) => Decimal::from_str(&n.to_string()).unwrap(),
            other => panic!("Expected decimal, got {other}"),
        }
    };

    // Checking statistics (year 2026, default)
    let resp = server
        .get(&format!("/api/v1/accounts/{checking_id}/statistics"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let cs = &body.data["statistics"][0];
    assert_eq!(json_dec(&cs["min_state"]), dec("0"), "Checking min=0 (before salary)");
    assert_eq!(json_dec(&cs["max_state"]), dec("35000"), "Checking max=35000 (after all txns)");
    assert_eq!(json_dec(&cs["end_of_period_state"]), dec("35000"), "Checking end_of_period");
    assert_eq!(json_dec(&cs["current_state"]), dec("35000"), "Checking current");
    assert_eq!(json_dec(&cs["end_of_current_month_state"]), dec("35000"), "Checking end_of_month");
    assert_eq!(json_dec(&cs["upcoming_expenses"]), dec("0"), "Checking upcoming=0 (no recurring)");

    // Savings statistics
    let resp = server
        .get(&format!("/api/v1/accounts/{savings_id}/statistics"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let ss = &body.data["statistics"][0];
    assert_eq!(json_dec(&ss["min_state"]), dec("0"), "Savings min=0 (before transfer)");
    assert_eq!(json_dec(&ss["max_state"]), dec("10000"), "Savings max=10000 (after transfer)");
    assert_eq!(json_dec(&ss["end_of_period_state"]), dec("10000"), "Savings end_of_period");
    assert_eq!(json_dec(&ss["current_state"]), dec("10000"), "Savings current");

    // Batch statistics
    let resp = server.get("/api/v1/accounts/statistics").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(body.data.len(), 2, "Should have exactly 2 accounts in batch stats");

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

    // ── Step 10: Verify dashboard metrics with frozen values ──
    let resp = server.get("/api/v1/metrics/dashboard").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let dm = &body.data;
    assert_eq!(json_dec(&dm["total_net_worth"]), dec("45000"), "Total net worth");
    assert_eq!(json_dec(&dm["liquid_net_worth"]), dec("45000"), "All accounts liquid");
    assert_eq!(json_dec(&dm["non_liquid_net_worth"]), dec("0"), "No non-liquid accounts");
    assert_eq!(json_dec(&dm["essential_burn_rate"]), dec("0"), "No recurring");
    assert_eq!(json_dec(&dm["full_burn_rate"]), dec("0"), "No recurring");

    let acct_metrics = dm["account_metrics"].as_array().unwrap();
    assert_eq!(acct_metrics.len(), 2, "2 accounts in dashboard");
    for am in acct_metrics {
        let aid = am["account_id"].as_i64().unwrap() as i32;
        if aid == checking_id {
            assert_eq!(json_dec(&am["current_balance"]), dec("35000"), "Dashboard: Checking");
        } else if aid == savings_id {
            assert_eq!(json_dec(&am["current_balance"]), dec("10000"), "Dashboard: Savings");
        }
    }

    // ── Step 11: Verify account metrics with frozen values ──
    let resp = server
        .get(&format!("/api/v1/accounts/{checking_id}/metrics"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(json_dec(&body.data["current_balance"]), dec("35000"), "Checking metrics");
    assert_eq!(body.data["account_kind"], "RealAccount");
    assert_eq!(body.data["kind_metrics"]["type"], "Operating");

    let resp = server
        .get(&format!("/api/v1/accounts/{savings_id}/metrics"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(json_dec(&body.data["current_balance"]), dec("10000"), "Savings metrics");
    assert_eq!(body.data["account_kind"], "Savings");
    assert_eq!(body.data["kind_metrics"]["type"], "Reserve");

    // ── Step 12: Verify monthly min balance with frozen values ──
    let resp = server
        .get(&format!(
            "/api/v1/accounts/{checking_id}/monthly-min-balance?months=12"
        ))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let pts = body.data["data_points"].as_array().unwrap();
    assert!(!pts.is_empty());
    let jan_point = pts.iter().find(|p| p["year"] == 2026 && p["month"] == 1);
    if let Some(jp) = jan_point {
        assert_eq!(json_dec(&jp["min_balance"]), dec("0"), "Min balance in Jan = 0 (before salary)");
    }

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

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use chrono::NaiveDate;
use common::setup_test_app;
use finrust::handlers::accounts::{AccountKind, CreateAccountRequest};
use finrust::handlers::categories::CreateCategoryRequest;
use finrust::handlers::manual_account_states::CreateManualAccountStateRequest;
use finrust::handlers::recurring_income::CreateRecurringIncomeRequest;
use finrust::handlers::transactions::{
    CreateRecurringInstanceRequest, CreateRecurringTransactionRequest, CreateTransactionRequest,
    RecurringInstanceResponse, RecurringTransactionResponse,
};
use finrust::handlers::transactions::recurring_instances::UpdateRecurringInstanceRequest;
use finrust::schemas::ApiResponse;
use rust_decimal::Decimal;
use serde_json::json;

#[tokio::test]
async fn test_e2e_intermediate_scenario() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    // ── Phase 1: Create accounts ──────────────────────────────────────

    let accounts = vec![
        ("Checking", AccountKind::RealAccount),
        ("Savings", AccountKind::Savings),
        ("Investment", AccountKind::Investment),
        ("Credit Card", AccountKind::Debt),
    ];

    let mut account_ids: Vec<i32> = Vec::new();
    for (name, kind) in &accounts {
        let req = CreateAccountRequest {
            name: name.to_string(),
            description: Some(format!("{} account", name)),
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: Some(true),
            ledger_name: None,
            account_kind: Some(kind.clone()),
            target_amount: None,
            color: None,
            is_liquid: None,
        };
        let resp = server.post("/api/v1/accounts").json(&req).await;
        resp.assert_status(StatusCode::CREATED);
        let body: ApiResponse<serde_json::Value> = resp.json();
        assert!(body.success);
        account_ids.push(body.data["id"].as_i64().unwrap() as i32);
    }

    let checking_id = account_ids[0];
    let savings_id = account_ids[1];
    let investment_id = account_ids[2];
    let _credit_card_id = account_ids[3];

    // ── Phase 2: Create category hierarchy ────────────────────────────

    let create_cat = |name: &str, parent_id: Option<i32>| CreateCategoryRequest {
        name: name.to_string(),
        description: Some(format!("{} category", name)),
        parent_id,
    };

    let resp = server
        .post("/api/v1/categories")
        .json(&create_cat("Living", None))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    let living_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/categories")
        .json(&create_cat("Rent", Some(living_id)))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    let rent_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/categories")
        .json(&create_cat("Utilities", Some(living_id)))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    let utilities_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/categories")
        .json(&create_cat("Food", None))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    let food_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/categories")
        .json(&create_cat("Groceries", Some(food_id)))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    let groceries_id = body.data["id"].as_i64().unwrap() as i32;

    let resp = server
        .post("/api/v1/categories")
        .json(&create_cat("Restaurants", Some(food_id)))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    let restaurants_id = body.data["id"].as_i64().unwrap() as i32;

    // ── Phase 3: Create recurring income ──────────────────────────────

    let salary_req = CreateRecurringIncomeRequest {
        name: "Monthly Salary".to_string(),
        description: Some("Regular monthly salary".to_string()),
        amount: Decimal::new(50000, 0),
        start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end_date: None,
        period: "Monthly".to_string(),
        include_in_statistics: Some(true),
        target_account_id: checking_id,
        source_name: Some("Employer".to_string()),
        ledger_name: None,
        scenario_id: None,
        is_simulated: Some(false),
    };
    let resp = server
        .post("/api/v1/recurring-incomes")
        .json(&salary_req)
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    // ── Phase 4: Create recurring transactions ────────────────────────

    let rent_req = CreateRecurringTransactionRequest {
        name: "Monthly Rent".to_string(),
        description: Some("Apartment rent".to_string()),
        amount: Decimal::new(-15000, 0),
        start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end_date: None,
        period: "Monthly".to_string(),
        include_in_statistics: Some(true),
        target_account_id: checking_id,
        source_account_id: None,
        ledger_name: None,
        category_id: Some(rent_id),
        scenario_id: None,
        is_simulated: Some(false),
    };
    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&rent_req)
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<RecurringTransactionResponse> = resp.json();
    assert!(body.success);
    let rent_recurring_id = body.data.id;

    let utilities_req = CreateRecurringTransactionRequest {
        name: "Monthly Utilities".to_string(),
        description: Some("Monthly utility bills".to_string()),
        amount: Decimal::new(-3000, 0),
        start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end_date: None,
        period: "Monthly".to_string(),
        include_in_statistics: Some(true),
        target_account_id: checking_id,
        source_account_id: None,
        ledger_name: None,
        category_id: Some(utilities_id),
        scenario_id: None,
        is_simulated: Some(false),
    };
    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&utilities_req)
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<RecurringTransactionResponse> = resp.json();
    assert!(body.success);
    let utilities_recurring_id = body.data.id;

    let savings_req = CreateRecurringTransactionRequest {
        name: "Monthly Savings".to_string(),
        description: Some("Transfer to savings".to_string()),
        amount: Decimal::new(5000, 0),
        start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end_date: None,
        period: "Monthly".to_string(),
        include_in_statistics: Some(true),
        target_account_id: savings_id,
        source_account_id: Some(checking_id),
        ledger_name: None,
        category_id: None,
        scenario_id: None,
        is_simulated: Some(false),
    };
    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&savings_req)
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<RecurringTransactionResponse> = resp.json();
    assert!(body.success);
    let savings_recurring_id = body.data.id;

    // ── Phase 5: Create and pay recurring instances for Jan–Mar 2025 ──

    let recurring_ids = [rent_recurring_id, utilities_recurring_id, savings_recurring_id];
    let months = [1u32, 2, 3];

    for &rec_id in &recurring_ids {
        for &month in &months {
            let date = NaiveDate::from_ymd_opt(2025, month, 1).unwrap();

            let instance_req = CreateRecurringInstanceRequest {
                date,
                amount: None,
            };
            let resp = server
                .post(&format!(
                    "/api/v1/recurring-transactions/{}/instances",
                    rec_id
                ))
                .json(&instance_req)
                .await;
            resp.assert_status(StatusCode::CREATED);
            let body: ApiResponse<RecurringInstanceResponse> = resp.json();
            assert!(body.success);
            let instance_id = body.data.id;
            let expected_amount = body.data.expected_amount;

            let update_req = UpdateRecurringInstanceRequest {
                status: Some("Paid".to_string()),
                due_date: None,
                expected_amount: None,
                paid_date: Some(date),
                paid_amount: Some(expected_amount),
            };
            let resp = server
                .put(&format!("/api/v1/recurring-instances/{}", instance_id))
                .json(&update_req)
                .await;
            resp.assert_status(StatusCode::OK);
            let body: ApiResponse<serde_json::Value> = resp.json();
            assert!(body.success);
        }
    }

    // ── Phase 6: Check missing instances ──────────────────────────────

    let resp = server
        .get("/api/v1/recurring-transactions/missing-instances")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert!(
        !body.data.is_empty(),
        "Should have missing instances for months after March"
    );

    // ── Phase 7: Create one-off transactions ──────────────────────────

    let oneoff_txns = vec![
        ("Groceries Jan", -1500, 2025, 1, 10, checking_id, None, Some(groceries_id)),
        ("Restaurant Jan", -800, 2025, 1, 20, checking_id, None, Some(restaurants_id)),
        ("Groceries Feb", -1800, 2025, 2, 5, checking_id, None, Some(groceries_id)),
        ("Investment purchase", 20000, 2025, 2, 15, investment_id, Some(checking_id), None),
        ("Groceries Mar", -2000, 2025, 3, 10, checking_id, None, Some(groceries_id)),
    ];

    let mut transaction_ids: Vec<i32> = Vec::new();
    for (name, amount, year, month, day, target, source, cat) in &oneoff_txns {
        let req = CreateTransactionRequest {
            name: name.to_string(),
            description: None,
            amount: Decimal::new(*amount as i64, 0),
            date: NaiveDate::from_ymd_opt(*year, *month, *day).unwrap(),
            include_in_statistics: Some(true),
            target_account_id: *target,
            source_account_id: *source,
            ledger_name: None,
            linked_import_id: None,
            category_id: *cat,
            scenario_id: None,
            is_simulated: Some(false),
        };
        let resp = server.post("/api/v1/transactions").json(&req).await;
        resp.assert_status(StatusCode::CREATED);
        let body: ApiResponse<serde_json::Value> = resp.json();
        assert!(body.success);
        transaction_ids.push(body.data["id"].as_i64().unwrap() as i32);
    }

    // ── Phase 8: Add manual account state (opening balance) ───────────

    let manual_state_req = CreateManualAccountStateRequest {
        date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        amount: Decimal::new(0, 0),
    };
    let resp = server
        .post(&format!(
            "/api/v1/accounts/{}/manual-states",
            checking_id
        ))
        .json(&manual_state_req)
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    // ── Phase 9: Verify statistics ────────────────────────────────────

    let resp = server
        .get(&format!(
            "/api/v1/accounts/{}/statistics?year=2025",
            checking_id
        ))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);

    let resp = server
        .get("/api/v1/accounts/statistics?year=2025")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert!(
        body.data.len() >= 4,
        "Should have stats for all 4 accounts, got {}",
        body.data.len()
    );

    // ── Phase 10: Verify timeseries ───────────────────────────────────

    let resp = server
        .get("/api/v1/accounts/timeseries?start_date=2025-01-01&end_date=2025-04-01")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<::common::AccountStateTimeseries> = resp.json();
    assert!(body.success);
    assert!(
        !body.data.data_points.is_empty(),
        "Should have timeseries data points for accounts"
    );

    // ── Phase 11: Verify dashboard metrics ────────────────────────────

    let resp = server.get("/api/v1/metrics/dashboard").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert!(
        body.data.get("total_net_worth").is_some(),
        "Dashboard should contain total_net_worth"
    );

    // ── Phase 12: Verify account metrics per type ─────────────────────

    for &acct_id in &[checking_id, savings_id, investment_id] {
        let resp = server
            .get(&format!("/api/v1/accounts/{}/metrics", acct_id))
            .await;
        resp.assert_status(StatusCode::OK);
        let body: ApiResponse<serde_json::Value> = resp.json();
        assert!(body.success);
    }

    // ── Phase 13: Verify category stats ───────────────────────────────

    let resp = server
        .get("/api/v1/categories/stats?start_date=2025-01-01&end_date=2025-03-31")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);

    let food_stat = body
        .data
        .iter()
        .find(|c| c["category_name"] == "Food")
        .expect("Food category should appear in stats");
    let food_total: f64 = food_stat["total_amount"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    // Food total = Groceries(-1500 -1800 -2000) + Restaurants(-800) = -6100
    assert_eq!(
        food_total, -6100.0,
        "Food total should be -6100 (sum of Groceries + Restaurants)"
    );

    let living_stat = body
        .data
        .iter()
        .find(|c| c["category_name"] == "Living")
        .expect("Living category should appear in stats");
    let living_total: f64 = living_stat["total_amount"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    // Living total = Rent(-15000*3) + Utilities(-3000*3) = -54000
    assert_eq!(
        living_total, -54000.0,
        "Living total should be -54000 (sum of Rent + Utilities over 3 months)"
    );

    // ── Phase 14: Verify category children ────────────────────────────

    let resp = server
        .get(&format!("/api/v1/categories/{}/children", living_id))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(
        body.data.len(),
        2,
        "Living should have exactly 2 children (Rent, Utilities)"
    );
    let child_names: Vec<&str> = body
        .data
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(child_names.contains(&"Rent"));
    assert!(child_names.contains(&"Utilities"));

    // ── Phase 15: Update and delete operations ────────────────────────

    let first_txn_id = transaction_ids[0];
    let update_req = json!({
        "name": "Groceries Jan (updated)"
    });
    let resp = server
        .put(&format!("/api/v1/transactions/{}", first_txn_id))
        .json(&update_req)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(body.data["name"], "Groceries Jan (updated)");

    let second_txn_id = transaction_ids[1];
    let resp = server
        .delete(&format!("/api/v1/transactions/{}", second_txn_id))
        .await;
    resp.assert_status(StatusCode::OK);

    let resp = server
        .get(&format!("/api/v1/transactions/{}", second_txn_id))
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

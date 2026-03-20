mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use chrono::{Datelike, NaiveDate};
use common::setup_test_app;
use finrust::handlers::accounts::{AccountKind, CreateAccountRequest};
use finrust::handlers::manual_account_states::CreateManualAccountStateRequest;
use finrust::handlers::transactions::CreateTransactionRequest;
use finrust::schemas::{ApiResponse, TimeseriesQuery};
use rust_decimal::Decimal;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_else(|_| panic!("Invalid decimal: {s}"))
}

fn json_dec(v: &serde_json::Value) -> Decimal {
    match v {
        serde_json::Value::String(s) => dec(s),
        serde_json::Value::Number(n) => dec(&n.to_string()),
        serde_json::Value::Null => panic!("Expected decimal, got null"),
        other => panic!("Expected decimal, got {other}"),
    }
}

async fn create_account(
    server: &TestServer,
    name: &str,
    kind: AccountKind,
    liquid: bool,
    include_in_stats: bool,
    target: Option<Decimal>,
) -> i32 {
    let resp = server
        .post("/api/v1/accounts")
        .json(&CreateAccountRequest {
            name: name.to_string(),
            description: None,
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: Some(include_in_stats),
            ledger_name: None,
            account_kind: Some(kind),
            target_amount: target,
            color: None,
            is_liquid: Some(liquid),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    body.data["id"].as_i64().unwrap() as i32
}

async fn create_txn(
    server: &TestServer,
    name: &str,
    amount: i64,
    date: NaiveDate,
    target_id: i32,
    source_id: Option<i32>,
) {
    let resp = server
        .post("/api/v1/transactions")
        .json(&CreateTransactionRequest {
            name: name.to_string(),
            description: None,
            amount: Decimal::new(amount, 0),
            date,
            include_in_statistics: Some(true),
            target_account_id: target_id,
            source_account_id: source_id,
            ledger_name: None,
            linked_import_id: None,
            category_id: None,
            scenario_id: None,
            is_simulated: Some(false),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);
}

async fn assert_balance(server: &TestServer, account_id: i32, date_str: &str, expected: Decimal) {
    let target = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
    let year_start = NaiveDate::from_ymd_opt(target.year(), 1, 1).unwrap();
    let end = target.succ_opt().unwrap();
    let ts_query = TimeseriesQuery {
        start_date: year_start,
        end_date: end,
        include_ignored: true,
        scenario_id: None,
    };
    let resp = server
        .get(&format!("/api/v1/accounts/{account_id}/timeseries"))
        .add_query_params(&ts_query)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<::common::AccountStateTimeseries> = resp.json();
    assert!(body.success);
    let point = body
        .data
        .data_points
        .iter()
        .find(|p| p.date.to_string() == date_str && p.account_id == account_id)
        .unwrap_or_else(|| panic!("No data point for account {account_id} on {date_str}"));
    assert_eq!(
        point.balance, expected,
        "Account {account_id} balance on {date_str}: expected {expected}, got {}",
        point.balance
    );
}

async fn get_account_stats(
    server: &TestServer,
    account_id: i32,
    year: i32,
) -> serde_json::Value {
    let resp = server
        .get(&format!(
            "/api/v1/accounts/{account_id}/statistics?year={year}"
        ))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    body.data["statistics"][0].clone()
}

fn assert_stat(stats: &serde_json::Value, field: &str, expected: Decimal, ctx: &str) {
    let actual = json_dec(&stats[field]);
    assert_eq!(actual, expected, "{ctx}: {field} expected {expected}, got {actual}");
}


/// Comprehensive snapshot test: 3 accounts, one-off transactions only, all in Jan 2026.
///
/// Since there are no recurring transactions, balances are stable after the last
/// transaction, making every value deterministic regardless of exact run date
/// (as long as it runs in 2026 after January).
///
/// Accounts:
///   1. Checking (RealAccount, liquid)   — manual state 100000 on 2026-01-01
///   2. Savings  (Savings, liquid, target=50000)
///   3. Investment (Investment, non-liquid)
///
/// Transactions (all January 2026):
///   +50000 Jan 05 → Checking  (Salary)
///   -5000  Jan 10 → Checking  (Groceries)
///   +10000 Jan 12 → Savings   from Checking (Transfer)
///   +20000 Jan 15 → Investment from Checking (Transfer)
///   +5000  Jan 18 → Investment (Capital gain)
///   +8000  Jan 20 → Savings   from Checking (Transfer)
///
/// Final balances (from Jan 20 onward, stable):
///   Checking:   100000 + 50000 - 5000 - 10000 - 20000 - 8000 = 107000
///   Savings:    0 + 10000 + 8000 = 18000
///   Investment: 0 + 20000 + 5000 = 25000
///   Net worth:  150000
#[tokio::test]
async fn test_snapshot_frozen_balances() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    let d = |m: u32, d: u32| NaiveDate::from_ymd_opt(2026, m, d).unwrap();

    // ── Phase 1: Create accounts ──
    let checking_id =
        create_account(&server, "Checking", AccountKind::RealAccount, true, true, None).await;
    let savings_id = create_account(
        &server,
        "Savings",
        AccountKind::Savings,
        true,
        true,
        Some(Decimal::new(50000, 0)),
    )
        .await;
    let investment_id =
        create_account(&server, "Investment", AccountKind::Investment, false, true, None).await;

    // ── Phase 2: Manual state for Checking ──
    let resp = server
        .post(&format!("/api/v1/accounts/{checking_id}/manual-states"))
        .json(&CreateManualAccountStateRequest {
            date: d(1, 1),
            amount: Decimal::new(100000, 0),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // ── Phase 3: Create transactions (all January 2026) ──
    create_txn(&server, "Salary", 50000, d(1, 5), checking_id, None).await;
    create_txn(&server, "Groceries", -5000, d(1, 10), checking_id, None).await;
    create_txn(&server, "To Savings", 10000, d(1, 12), savings_id, Some(checking_id)).await;
    create_txn(&server, "To Investment", 20000, d(1, 15), investment_id, Some(checking_id)).await;
    create_txn(&server, "Capital gain", 5000, d(1, 18), investment_id, None).await;
    create_txn(&server, "To Savings 2", 8000, d(1, 20), savings_id, Some(checking_id)).await;

    // ── Phase 4: Freeze timeseries balances at specific dates ──

    assert_balance(&server, checking_id, "2026-01-01", dec("100000")).await;
    assert_balance(&server, checking_id, "2026-01-05", dec("150000")).await;
    assert_balance(&server, checking_id, "2026-01-10", dec("145000")).await;
    assert_balance(&server, checking_id, "2026-01-12", dec("135000")).await;
    assert_balance(&server, checking_id, "2026-01-15", dec("115000")).await;
    assert_balance(&server, checking_id, "2026-01-20", dec("107000")).await;

    assert_balance(&server, savings_id, "2026-01-05", dec("0")).await;
    assert_balance(&server, savings_id, "2026-01-12", dec("10000")).await;
    assert_balance(&server, savings_id, "2026-01-20", dec("18000")).await;

    assert_balance(&server, investment_id, "2026-01-10", dec("0")).await;
    assert_balance(&server, investment_id, "2026-01-15", dec("20000")).await;
    assert_balance(&server, investment_id, "2026-01-18", dec("25000")).await;

    // ── Phase 5: Freeze per-account statistics (year 2026) ──

    let cs = get_account_stats(&server, checking_id, 2026).await;
    assert_stat(&cs, "min_state", dec("100000"), "Checking");
    assert_stat(&cs, "max_state", dec("150000"), "Checking");
    assert_stat(&cs, "end_of_period_state", dec("107000"), "Checking");
    assert_stat(&cs, "current_state", dec("107000"), "Checking");
    assert_stat(&cs, "end_of_current_month_state", dec("107000"), "Checking");
    assert_stat(&cs, "upcoming_expenses", dec("0"), "Checking");
    assert_stat(&cs, "average_expense", dec("43000"), "Checking");
    assert_stat(&cs, "average_income", dec("50000"), "Checking");

    let ss = get_account_stats(&server, savings_id, 2026).await;
    assert_stat(&ss, "min_state", dec("0"), "Savings");
    assert_stat(&ss, "max_state", dec("18000"), "Savings");
    assert_stat(&ss, "end_of_period_state", dec("18000"), "Savings");
    assert_stat(&ss, "current_state", dec("18000"), "Savings");
    assert_stat(&ss, "end_of_current_month_state", dec("18000"), "Savings");
    assert_stat(&ss, "upcoming_expenses", dec("0"), "Savings");
    assert_stat(&ss, "average_expense", dec("0"), "Savings");
    assert_stat(&ss, "average_income", dec("18000"), "Savings");

    let is_ = get_account_stats(&server, investment_id, 2026).await;
    assert_stat(&is_, "min_state", dec("0"), "Investment");
    assert_stat(&is_, "max_state", dec("25000"), "Investment");
    assert_stat(&is_, "end_of_period_state", dec("25000"), "Investment");
    assert_stat(&is_, "current_state", dec("25000"), "Investment");
    assert_stat(&is_, "end_of_current_month_state", dec("25000"), "Investment");
    assert_stat(&is_, "upcoming_expenses", dec("0"), "Investment");
    assert_stat(&is_, "average_expense", dec("0"), "Investment");
    assert_stat(&is_, "average_income", dec("25000"), "Investment");

    // ── Phase 6: Freeze batch statistics ──

    let resp = server.get("/api/v1/accounts/statistics?year=2026").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert!(body.success);
    assert_eq!(body.data.len(), 3, "Batch statistics: 3 accounts");

    for item in &body.data {
        let stat = &item["statistics"][0];
        let aid = stat["account_id"].as_i64().unwrap() as i32;
        let eop = json_dec(&stat["end_of_period_state"]);
        if aid == checking_id {
            assert_eq!(eop, dec("107000"), "Batch: Checking end_of_period");
        } else if aid == savings_id {
            assert_eq!(eop, dec("18000"), "Batch: Savings end_of_period");
        } else if aid == investment_id {
            assert_eq!(eop, dec("25000"), "Batch: Investment end_of_period");
        }
    }

    // ── Phase 7: Freeze dashboard metrics ──

    let resp = server.get("/api/v1/metrics/dashboard").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let dm = &body.data;

    assert_eq!(json_dec(&dm["total_net_worth"]), dec("150000"), "Dashboard total_net_worth");
    assert_eq!(json_dec(&dm["liquid_net_worth"]), dec("125000"), "Dashboard liquid (Checking+Savings)");
    assert_eq!(json_dec(&dm["non_liquid_net_worth"]), dec("25000"), "Dashboard non-liquid (Investment)");
    assert_eq!(json_dec(&dm["essential_burn_rate"]), dec("0"), "No recurring → zero burn rate");
    assert_eq!(json_dec(&dm["full_burn_rate"]), dec("0"), "No recurring → zero burn rate");

    assert_eq!(json_dec(&dm["free_cashflow"]), dec("0"), "No recurring → zero cashflow");
    assert!(dm["savings_rate"].is_null(), "No income → savings_rate is null");
    assert_eq!(json_dec(&dm["goal_engine"]), dec("0"), "No recurring → zero goal_engine");

    let acct_metrics = dm["account_metrics"].as_array().unwrap();
    assert_eq!(acct_metrics.len(), 3, "Dashboard: 3 account metrics");

    for am in acct_metrics {
        let aid = am["account_id"].as_i64().unwrap() as i32;
        let bal = json_dec(&am["current_balance"]);
        if aid == checking_id {
            assert_eq!(bal, dec("107000"), "Dashboard Checking current_balance");
            assert_eq!(am["account_kind"], "RealAccount");
        } else if aid == savings_id {
            assert_eq!(bal, dec("18000"), "Dashboard Savings current_balance");
            assert_eq!(am["account_kind"], "Savings");
        } else if aid == investment_id {
            assert_eq!(bal, dec("25000"), "Dashboard Investment current_balance");
            assert_eq!(am["account_kind"], "Investment");
        }
    }

    // ── Phase 8: Freeze per-account metrics ──

    for (aid, expected_bal, kind) in [
        (checking_id, dec("107000"), "RealAccount"),
        (savings_id, dec("18000"), "Savings"),
        (investment_id, dec("25000"), "Investment"),
    ] {
        let resp = server
            .get(&format!("/api/v1/accounts/{aid}/metrics"))
            .await;
        resp.assert_status(StatusCode::OK);
        let body: ApiResponse<serde_json::Value> = resp.json();
        assert!(body.success);
        assert_eq!(
            json_dec(&body.data["current_balance"]),
            expected_bal,
            "Account {aid} metrics current_balance"
        );
        assert_eq!(body.data["account_kind"], kind);
    }

    // kind_metrics types
    let resp = server.get(&format!("/api/v1/accounts/{checking_id}/metrics")).await;
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert_eq!(body.data["kind_metrics"]["type"], "Operating");

    let resp = server.get(&format!("/api/v1/accounts/{savings_id}/metrics")).await;
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert_eq!(body.data["kind_metrics"]["type"], "Reserve");

    let resp = server.get(&format!("/api/v1/accounts/{investment_id}/metrics")).await;
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert_eq!(body.data["kind_metrics"]["type"], "Investment");

    // ── Phase 9: Freeze monthly min balance ──

    let resp = server
        .get(&format!(
            "/api/v1/accounts/{checking_id}/monthly-min-balance?months=6"
        ))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let points = body.data["data_points"].as_array().unwrap();
    assert!(!points.is_empty(), "Should have monthly min balance data");

    // Find Jan 2026 — min balance should be 100000 (balance on Jan 1 before salary on Jan 5)
    let jan_point = points
        .iter()
        .find(|p| p["year"] == 2026 && p["month"] == 1)
        .expect("Should have Jan 2026 data point");
    assert_eq!(
        json_dec(&jan_point["min_balance"]),
        dec("100000"),
        "Jan 2026 min = initial manual state (100000)"
    );

    // Feb 2026 and beyond — min balance should be 107000 (stable after last transaction)
    let feb_point = points
        .iter()
        .find(|p| p["year"] == 2026 && p["month"] == 2);
    if let Some(fp) = feb_point {
        assert_eq!(
            json_dec(&fp["min_balance"]),
            dec("107000"),
            "Feb 2026 min = final balance (107000)"
        );
    }

    // ── Phase 10: Cross-checks ──

    // Timeseries net worth at Jan 20 should sum to 150000 (all 3 accounts)
    let ts_query = TimeseriesQuery {
        start_date: d(1, 1),
        end_date: d(1, 22),
        include_ignored: true,
        scenario_id: None,
    };
    let resp = server
        .get("/api/v1/accounts/timeseries")
        .add_query_params(&ts_query)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<::common::AccountStateTimeseries> = resp.json();
    assert!(body.success);
    let net_worth_ts: Decimal = body
        .data
        .data_points
        .iter()
        .filter(|p| p.date == d(1, 20))
        .map(|p| p.balance)
        .sum();
    assert_eq!(
        net_worth_ts,
        dec("150000"),
        "Timeseries net worth at 2026-01-20 should equal dashboard total_net_worth"
    );
}

/// Verify include_in_statistics filtering works for all endpoints.
#[tokio::test]
async fn test_snapshot_include_ignored_filtering() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    let visible_id =
        create_account(&server, "Visible", AccountKind::RealAccount, true, true, None).await;
    let hidden_id =
        create_account(&server, "Hidden", AccountKind::Investment, false, false, None).await;

    let tx_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    create_txn(&server, "Salary", 80000, tx_date, visible_id, None).await;
    create_txn(&server, "Deposit", 30000, tx_date, hidden_id, None).await;

    // Dashboard includes ALL accounts (even hidden)
    let resp = server.get("/api/v1/metrics/dashboard").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    assert_eq!(
        json_dec(&body.data["total_net_worth"]),
        dec("110000"),
        "Dashboard net worth includes both visible (80000) and hidden (30000)"
    );
    let metrics = body.data["account_metrics"].as_array().unwrap();
    assert_eq!(metrics.len(), 2, "Dashboard metrics for BOTH accounts");

    // Statistics without include_ignored: only visible
    let resp = server.get("/api/v1/accounts/statistics?year=2026").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert_eq!(body.data.len(), 1, "Without include_ignored: 1 account");
    assert_eq!(body.data[0]["statistics"][0]["account_id"], visible_id);

    // Statistics with include_ignored: both
    let resp = server
        .get("/api/v1/accounts/statistics?year=2026&include_ignored=true")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<Vec<serde_json::Value>> = resp.json();
    assert_eq!(body.data.len(), 2, "With include_ignored: 2 accounts");

    // Timeseries without include_ignored
    let ts_query = TimeseriesQuery {
        start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        include_ignored: false,
        scenario_id: None,
    };
    let resp = server
        .get("/api/v1/accounts/timeseries")
        .add_query_params(&ts_query)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<::common::AccountStateTimeseries> = resp.json();
    let ts_ids: std::collections::HashSet<i32> =
        body.data.data_points.iter().map(|p| p.account_id).collect();
    assert!(ts_ids.contains(&visible_id));
    assert!(!ts_ids.contains(&hidden_id), "Hidden excluded from timeseries");

    // Timeseries with include_ignored
    let ts_all = TimeseriesQuery {
        start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        include_ignored: true,
        scenario_id: None,
    };
    let resp = server
        .get("/api/v1/accounts/timeseries")
        .add_query_params(&ts_all)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<::common::AccountStateTimeseries> = resp.json();
    let ts_ids: std::collections::HashSet<i32> =
        body.data.data_points.iter().map(|p| p.account_id).collect();
    assert!(ts_ids.contains(&visible_id));
    assert!(ts_ids.contains(&hidden_id), "Hidden included with flag");

    // Individual metrics work for hidden account
    let resp = server.get(&format!("/api/v1/accounts/{hidden_id}/metrics")).await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert_eq!(json_dec(&body.data["current_balance"]), dec("30000"));
    assert_eq!(body.data["account_kind"], "Investment");

    // Individual statistics for hidden: 404 without include_ignored
    let resp = server
        .get(&format!("/api/v1/accounts/{hidden_id}/statistics?year=2026"))
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);

    // Individual statistics for hidden: OK with include_ignored=true
    let resp = server
        .get(&format!("/api/v1/accounts/{hidden_id}/statistics?year=2026&include_ignored=true"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let s = &body.data["statistics"][0];
    assert_eq!(json_dec(&s["end_of_period_state"]), dec("30000"));
}

/// Freeze dashboard burn rates computed from active recurring transactions.
#[tokio::test]
async fn test_snapshot_dashboard_burn_rates() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    let checking_id =
        create_account(&server, "Main", AccountKind::RealAccount, true, true, None).await;
    let _savings_id =
        create_account(&server, "EF", AccountKind::EmergencyFund, true, true, None).await;

    // Monthly rent: -15000 (expense, no source)
    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&finrust::handlers::transactions::CreateRecurringTransactionRequest {
            name: "Rent".to_string(),
            description: None,
            amount: Decimal::new(-15000, 0),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            period: "Monthly".to_string(),
            include_in_statistics: Some(true),
            target_account_id: checking_id,
            source_account_id: None,
            ledger_name: None,
            category_id: None,
            scenario_id: None,
            is_simulated: Some(false),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Monthly utilities: -3000 (expense, no source)
    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&finrust::handlers::transactions::CreateRecurringTransactionRequest {
            name: "Utilities".to_string(),
            description: None,
            amount: Decimal::new(-3000, 0),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            period: "Monthly".to_string(),
            include_in_statistics: Some(true),
            target_account_id: checking_id,
            source_account_id: None,
            ledger_name: None,
            category_id: None,
            scenario_id: None,
            is_simulated: Some(false),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Monthly savings transfer: +5000 (internal, HAS source)
    let resp = server
        .post("/api/v1/recurring-transactions")
        .json(&finrust::handlers::transactions::CreateRecurringTransactionRequest {
            name: "Monthly Savings".to_string(),
            description: None,
            amount: Decimal::new(5000, 0),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            period: "Monthly".to_string(),
            include_in_statistics: Some(true),
            target_account_id: _savings_id,
            source_account_id: Some(checking_id),
            ledger_name: None,
            category_id: None,
            scenario_id: None,
            is_simulated: Some(false),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    // Recurring income: +50000/month salary
    let resp = server
        .post("/api/v1/recurring-incomes")
        .json(&finrust::handlers::recurring_income::CreateRecurringIncomeRequest {
            name: "Salary".to_string(),
            description: None,
            amount: Decimal::new(50000, 0),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            period: "Monthly".to_string(),
            include_in_statistics: Some(true),
            target_account_id: checking_id,
            source_name: None,
            ledger_name: None,
            scenario_id: None,
            is_simulated: Some(false),
        })
        .await;
    resp.assert_status(StatusCode::CREATED);

    let resp = server.get("/api/v1/metrics/dashboard").await;
    resp.assert_status(StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = resp.json();
    assert!(body.success);
    let dm = &body.data;

    // Burn rates: only expenses WITHOUT source_account_id
    // Rent (-15000) + Utilities (-3000) = 18000
    // Savings transfer has source_account_id → excluded
    assert_eq!(
        json_dec(&dm["essential_burn_rate"]),
        dec("18000"),
        "Essential burn = rent + utilities (on RealAccount)"
    );
    assert_eq!(
        json_dec(&dm["full_burn_rate"]),
        dec("18000"),
        "Full burn = rent + utilities"
    );

    // free_cashflow = operating net flow + committed transfers out.
    // Checking: rent(-15000) + utilities(-3000) + savings_mirror(-5000) = -23000
    // Committed transfers out: savings transfer (+5000 on EF from Checking) = 5000
    // free_cashflow = -23000 + 5000 = -18000 (expenses only, transfers excluded)
    assert_eq!(json_dec(&dm["free_cashflow"]), dec("-18000"), "free_cashflow = income - expenses");

    // Cashflow breakdown is present and consistent
    let bd = &dm["cashflow_breakdown"];
    assert_eq!(json_dec(&bd["operating_net_flow"]), dec("-23000"), "operating net flow");
    assert_eq!(json_dec(&bd["committed_transfers_out"]), dec("5000"), "committed transfers out");
    assert_eq!(json_dec(&bd["free_cashflow"]), dec("-18000"), "breakdown free_cashflow matches");
    assert!(!bd["contributions"].as_array().unwrap().is_empty(), "has contributions");

    assert!(dm["savings_rate"].is_null(), "No income → savings_rate null");
    assert_eq!(json_dec(&dm["goal_engine"]), dec("5000"), "goal_engine = EF transfer amount");
    assert!(dm["commitment_ratio"].is_null(), "No income → commitment_ratio null");
    assert_eq!(
        json_dec(&dm["liquidity_ratio_months"]),
        dec("-1"),
        "Negative net worth / burn rate"
    );
}

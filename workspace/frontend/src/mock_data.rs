use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub id: String,
    pub date: String,
    pub description: String,
    pub amount: f64,
    pub account_id: i32,
    pub category_id: String,
    #[serde(rename = "type")]
    pub txn_type: String,
    pub status: String,
    pub is_simulated: bool,
    pub scenario_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurringRule {
    pub id: String,
    pub name: String,
    pub amount: f64,
    pub frequency: String,
    pub next_date: String,
    pub end_date: Option<String>,
    pub active: bool,
    pub category_id: String,
    pub account_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Budget {
    pub category_id: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountBalance {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub currency: String,
    pub current_balance: f64,
    pub institution: String,
    pub include_in_overview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetWorthPoint {
    pub date: String,
    pub accounts: std::collections::HashMap<i32, f64>,
}

pub fn get_mock_accounts() -> Vec<AccountBalance> {
    vec![
        AccountBalance {
            id: 1,
            name: "Main Checking".to_string(),
            account_type: "checking".to_string(),
            currency: "USD".to_string(),
            current_balance: 2450.50,
            institution: "Chase".to_string(),
            include_in_overview: true,
        },
        AccountBalance {
            id: 2,
            name: "High Yield Savings".to_string(),
            account_type: "savings".to_string(),
            currency: "USD".to_string(),
            current_balance: 15000.00,
            institution: "Ally".to_string(),
            include_in_overview: true,
        },
        AccountBalance {
            id: 3,
            name: "Amex Gold".to_string(),
            account_type: "credit_card".to_string(),
            currency: "USD".to_string(),
            current_balance: -450.20,
            institution: "Amex".to_string(),
            include_in_overview: true,
        },
        AccountBalance {
            id: 4,
            name: "Secret Stash".to_string(),
            account_type: "cash".to_string(),
            currency: "USD".to_string(),
            current_balance: 500.00,
            institution: "Home".to_string(),
            include_in_overview: false,
        },
    ]
}

pub fn get_mock_categories() -> Vec<Category> {
    vec![
        Category { id: "c1".to_string(), name: "Housing".to_string(), color: "#ef4444".to_string() },
        Category { id: "c2".to_string(), name: "Food".to_string(), color: "#f59e0b".to_string() },
        Category { id: "c3".to_string(), name: "Income".to_string(), color: "#22c55e".to_string() },
        Category { id: "c4".to_string(), name: "Utilities".to_string(), color: "#3b82f6".to_string() },
        Category { id: "c5".to_string(), name: "Entertainment".to_string(), color: "#a855f7".to_string() },
        Category { id: "c6".to_string(), name: "Transport".to_string(), color: "#6366f1".to_string() },
    ]
}

pub fn get_mock_transactions() -> Vec<Transaction> {
    let mut txns = Vec::new();
    let today = Local::now().naive_local().date();

    for i in 0..60 {
        let date = today - Duration::days(i);
        let date_str = date.format("%Y-%m-%d").to_string();

        // Paycheck every 14 days
        if i % 14 == 0 {
            txns.push(Transaction {
                id: format!("t{}", txns.len() + 1),
                date: date_str.clone(),
                description: "Tech Corp Salary".to_string(),
                amount: 3500.00,
                account_id: 1,
                category_id: "c3".to_string(),
                txn_type: "income".to_string(),
                status: "cleared".to_string(),
                is_simulated: false,
                scenario_id: None,
            });
        }

        // Groceries every 3 days
        if i % 3 == 0 {
            txns.push(Transaction {
                id: format!("t{}", txns.len() + 1),
                date: date_str.clone(),
                description: "Grocery Store".to_string(),
                amount: -(50.0 + (i as f64 * 3.7) % 50.0),
                account_id: 1,
                category_id: "c2".to_string(),
                txn_type: "expense".to_string(),
                status: "cleared".to_string(),
                is_simulated: false,
                scenario_id: None,
            });
        }

        // Rent on 1st of month
        if date.day() == 1 {
            txns.push(Transaction {
                id: format!("t{}", txns.len() + 1),
                date: date_str.clone(),
                description: "Luxury Apartments".to_string(),
                amount: -1800.00,
                account_id: 1,
                category_id: "c1".to_string(),
                txn_type: "expense".to_string(),
                status: "cleared".to_string(),
                is_simulated: false,
                scenario_id: None,
            });
        }

        // Transport every 5 days
        if i % 5 == 0 {
            txns.push(Transaction {
                id: format!("t{}", txns.len() + 1),
                date: date_str.clone(),
                description: "Uber/Lyft".to_string(),
                amount: -(15.0 + (i as f64 * 1.3) % 20.0),
                account_id: 3,
                category_id: "c6".to_string(),
                txn_type: "expense".to_string(),
                status: "pending".to_string(),
                is_simulated: false,
                scenario_id: None,
            });
        }
    }

    // Add a couple of simulated/scenario transactions for demo
    txns.push(Transaction {
        id: format!("t{}", txns.len() + 1),
        date: (today + Duration::days(5)).format("%Y-%m-%d").to_string(),
        description: "Simulated Future Expense".to_string(),
        amount: -250.00,
        account_id: 1,
        category_id: "c2".to_string(),
        txn_type: "expense".to_string(),
        status: "pending".to_string(),
        is_simulated: true,
        scenario_id: Some(1),
    });

    txns.push(Transaction {
        id: format!("t{}", txns.len() + 1),
        date: (today + Duration::days(10)).format("%Y-%m-%d").to_string(),
        description: "Scenario Test Transaction".to_string(),
        amount: -100.00,
        account_id: 2,
        category_id: "c5".to_string(),
        txn_type: "expense".to_string(),
        status: "pending".to_string(),
        is_simulated: true,
        scenario_id: Some(1),
    });

    txns.sort_by(|a, b| b.date.cmp(&a.date));
    txns
}

pub fn get_mock_recurring() -> Vec<RecurringRule> {
    let today = Local::now().naive_local().date();

    vec![
        RecurringRule {
            id: "r1".to_string(),
            name: "Rent Payment".to_string(),
            amount: -1800.00,
            frequency: "Monthly".to_string(),
            next_date: (today + Duration::days(5)).format("%Y-%m-%d").to_string(),
            end_date: None,
            active: true,
            category_id: "c1".to_string(),
            account_id: 1,
        },
        RecurringRule {
            id: "r2".to_string(),
            name: "Netflix".to_string(),
            amount: -15.99,
            frequency: "Monthly".to_string(),
            next_date: (today + Duration::days(12)).format("%Y-%m-%d").to_string(),
            end_date: None,
            active: true,
            category_id: "c5".to_string(),
            account_id: 3,
        },
        RecurringRule {
            id: "r3".to_string(),
            name: "Paycheck".to_string(),
            amount: 3500.00,
            frequency: "Bi-Weekly".to_string(),
            next_date: (today + Duration::days(2)).format("%Y-%m-%d").to_string(),
            end_date: None,
            active: true,
            category_id: "c3".to_string(),
            account_id: 1,
        },
    ]
}

pub fn get_mock_budgets() -> Vec<Budget> {
    vec![
        Budget { category_id: "c1".to_string(), amount: 2000.0 },
        Budget { category_id: "c2".to_string(), amount: 600.0 },
        Budget { category_id: "c5".to_string(), amount: 200.0 },
        Budget { category_id: "c4".to_string(), amount: 300.0 },
    ]
}

pub fn generate_net_worth_history() -> Vec<NetWorthPoint> {
    let accounts = get_mock_accounts();
    let today = Local::now().naive_local().date();
    let mut history = Vec::new();

    let mut balances: std::collections::HashMap<i32, f64> = accounts
        .iter()
        .map(|a| (a.id, a.current_balance))
        .collect();

    for i in (0..=90).rev() {
        let date = today - Duration::days(i);
        let date_str = date.format("%Y-%m-%d").to_string();

        // Add some variation to balances going backward
        for (_, balance) in balances.iter_mut() {
            *balance -= i as f64 * 5.0 - 225.0;
        }

        history.push(NetWorthPoint {
            date: date_str,
            accounts: balances.clone(),
        });
    }

    history
}

pub fn generate_forecast() -> Vec<NetWorthPoint> {
    let accounts = get_mock_accounts();
    let recurring = get_mock_recurring();
    let today = Local::now().naive_local().date();
    let mut forecast = Vec::new();

    let mut balances: std::collections::HashMap<i32, f64> = accounts
        .iter()
        .map(|a| (a.id, a.current_balance))
        .collect();

    let mut next_dates: std::collections::HashMap<String, NaiveDate> = recurring
        .iter()
        .map(|r| (r.id.clone(), NaiveDate::parse_from_str(&r.next_date, "%Y-%m-%d").unwrap()))
        .collect();

    for i in 1..=90 {
        let date = today + Duration::days(i);
        let date_str = date.format("%Y-%m-%d").to_string();

        // Apply recurring rules
        for rule in &recurring {
            if let Some(next_date) = next_dates.get_mut(&rule.id) {
                if &date == next_date {
                    if let Some(balance) = balances.get_mut(&rule.account_id) {
                        *balance += rule.amount;
                    }

                    // Advance next date based on frequency
                    *next_date = match rule.frequency.as_str() {
                        "Weekly" => *next_date + Duration::weeks(1),
                        "Bi-Weekly" => *next_date + Duration::weeks(2),
                        "Monthly" => {
                            let mut new_date = *next_date;
                            new_date = if new_date.month() == 12 {
                                NaiveDate::from_ymd_opt(new_date.year() + 1, 1, new_date.day()).unwrap()
                            } else {
                                NaiveDate::from_ymd_opt(new_date.year(), new_date.month() + 1, new_date.day()).unwrap()
                            };
                            new_date
                        }
                        "Yearly" => NaiveDate::from_ymd_opt(next_date.year() + 1, next_date.month(), next_date.day()).unwrap(),
                        _ => *next_date,
                    };
                }
            }
        }

        forecast.push(NetWorthPoint {
            date: date_str,
            accounts: balances.clone(),
        });
    }

    forecast
}

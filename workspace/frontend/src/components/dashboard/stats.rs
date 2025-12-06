use yew::prelude::*;
use crate::mock_data::{get_mock_accounts, get_mock_transactions};

#[function_component(Stats)]
pub fn stats() -> Html {
    let accounts = get_mock_accounts();
    let transactions = get_mock_transactions();

    let included_accounts: Vec<_> = accounts.iter().filter(|a| a.include_in_overview).collect();
    let net_worth: f64 = included_accounts.iter().map(|a| a.current_balance).sum();

    let income: f64 = transactions.iter()
        .filter(|t| t.amount > 0.0)
        .map(|t| t.amount)
        .sum();

    let expenses: f64 = transactions.iter()
        .filter(|t| t.amount < 0.0)
        .map(|t| t.amount.abs())
        .sum();

    let format_currency = |amount: f64| -> String {
        format!("${:.2}", amount)
    };

    let net_worth_class = if net_worth >= 0.0 { "text-primary" } else { "text-error" };

    html! {
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Net Worth"}</div>
                    <div class={classes!("stat-value", net_worth_class)}>{format_currency(net_worth)}</div>
                    <div class="stat-desc">{"Included accounts only"}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Income (60d)"}</div>
                    <div class="stat-value text-success">{format_currency(income)}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Expenses (60d)"}</div>
                    <div class="stat-value text-error">{format_currency(expenses)}</div>
                </div>
            </div>
        </div>
    }
}

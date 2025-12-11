use yew::prelude::*;
use chrono::{Local, Duration};
use rust_decimal::prelude::*;
use crate::api_client::account::get_accounts;
use crate::api_client::statistics::get_all_accounts_statistics;
use crate::api_client::timeseries::get_all_accounts_timeseries;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

#[function_component(Stats)]
pub fn stats() -> Html {
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let (timeseries_state, _) = use_fetch_with_refetch(|| async {
        let end_date = Local::now().date_naive();
        let start_date = end_date - Duration::days(60);
        get_all_accounts_timeseries(start_date, end_date).await
    });
    let (statistics_state, _) = use_fetch_with_refetch(get_all_accounts_statistics);

    let format_currency = |amount: Decimal| -> String {
        format!("${:.2}", amount)
    };

    // Calculate net worth from latest timeseries data
    let net_worth = match (&*accounts_state, &*timeseries_state) {
        (FetchState::Success(accounts), FetchState::Success(timeseries)) => {
            let included_account_ids: Vec<i32> = accounts
                .iter()
                .filter(|a| a.include_in_statistics)
                .map(|a| a.id)
                .collect();

            // Group data points by account_id and get the latest balance for each
            let mut latest_balances: std::collections::HashMap<i32, (chrono::NaiveDate, Decimal)> = std::collections::HashMap::new();
            for point in &timeseries.data_points {
                if included_account_ids.contains(&point.account_id) {
                    latest_balances.entry(point.account_id)
                        .and_modify(|(date, balance)| {
                            if point.date > *date {
                                *date = point.date;
                                *balance = point.balance;
                            }
                        })
                        .or_insert((point.date, point.balance));
                }
            }

            latest_balances.values().map(|(_, balance)| *balance).sum::<Decimal>()
        },
        _ => Decimal::ZERO,
    };

    // Calculate income and expenses from statistics
    let (income, expenses) = match &*statistics_state {
        FetchState::Success(collections) => {
            let total_income = collections
                .iter()
                .flat_map(|c| &c.statistics)
                .filter_map(|s| s.average_income)
                .sum::<Decimal>();

            let total_expenses = collections
                .iter()
                .flat_map(|c| &c.statistics)
                .filter_map(|s| s.average_expense)
                .sum::<Decimal>();

            (total_income, total_expenses)
        },
        _ => (Decimal::ZERO, Decimal::ZERO),
    };

    let net_worth_class = if net_worth >= Decimal::ZERO { "text-primary" } else { "text-error" };

    let is_loading = matches!(*accounts_state, FetchState::Loading)
        || matches!(*timeseries_state, FetchState::Loading)
        || matches!(*statistics_state, FetchState::Loading);

    if is_loading {
        return html! {
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div class="stats shadow bg-base-100">
                    <div class="stat">
                        <div class="stat-title">{"Net Worth"}</div>
                        <div class="stat-value"><span class="loading loading-spinner loading-sm"></span></div>
                    </div>
                </div>
                <div class="stats shadow bg-base-100">
                    <div class="stat">
                        <div class="stat-title">{"Income (60d)"}</div>
                        <div class="stat-value"><span class="loading loading-spinner loading-sm"></span></div>
                    </div>
                </div>
                <div class="stats shadow bg-base-100">
                    <div class="stat">
                        <div class="stat-title">{"Expenses (60d)"}</div>
                        <div class="stat-value"><span class="loading loading-spinner loading-sm"></span></div>
                    </div>
                </div>
            </div>
        };
    }

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

use yew::prelude::*;
use chrono::{Local, Duration};
use rust_decimal::prelude::*;
use crate::api_client::account::get_accounts_with_ignored;
use crate::api_client::statistics::get_all_accounts_statistics;
use crate::api_client::timeseries::get_all_accounts_timeseries_with_ignored;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::formatting::use_currency;
use crate::hooks::FetchState;

#[function_component(Stats)]
pub fn stats() -> Html {
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));
    let (timeseries_state, _) = use_fetch_with_refetch(|| async {
        let end_date = Local::now().date_naive();
        let start_date = end_date - Duration::days(60);
        get_all_accounts_timeseries_with_ignored(start_date, end_date, true).await
    });
    let (statistics_state, _) = use_fetch_with_refetch(get_all_accounts_statistics);
    let currency = use_currency();

    let format_currency = {
        let currency = currency.clone();
        move |amount: Decimal| -> String {
            format!("{:.1} {}", amount, currency)
        }
    };

    // Calculate net worth from latest timeseries data, split by liquidity
    let (net_worth, liquid_net_worth, non_liquid_net_worth) = match (&*accounts_state, &*timeseries_state) {
        (FetchState::Success(accounts), FetchState::Success(timeseries)) => {
            let all_account_ids: Vec<i32> = accounts.iter().map(|a| a.id).collect();
            let liquid_ids: std::collections::HashSet<i32> = accounts.iter()
                .filter(|a| a.is_liquid)
                .map(|a| a.id)
                .collect();

            let mut latest_balances: std::collections::HashMap<i32, (chrono::NaiveDate, Decimal)> = std::collections::HashMap::new();
            for point in &timeseries.data_points {
                if all_account_ids.contains(&point.account_id) {
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

            let total: Decimal = latest_balances.values().map(|(_, b)| *b).sum();
            let liquid: Decimal = latest_balances.iter()
                .filter(|(id, _)| liquid_ids.contains(id))
                .map(|(_, (_, b))| *b)
                .sum();
            let non_liquid = total - liquid;
            (total, liquid, non_liquid)
        },
        _ => (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
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
    let liquid_class = if liquid_net_worth >= Decimal::ZERO { "text-success" } else { "text-error" };

    let is_loading = matches!(*accounts_state, FetchState::Loading)
        || matches!(*timeseries_state, FetchState::Loading)
        || matches!(*statistics_state, FetchState::Loading);

    if is_loading {
        return html! {
            <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
                {for ["Net Worth", "Liquid", "Non-Liquid", "Avg. Monthly Income", "Avg. Monthly Expenses"].iter().map(|title| {
                    html! {
                        <div class="stats shadow bg-base-100">
                            <div class="stat">
                                <div class="stat-title">{title}</div>
                                <div class="stat-value"><span class="loading loading-spinner loading-sm"></span></div>
                            </div>
                        </div>
                    }
                })}
            </div>
        };
    }

    html! {
        <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Net Worth"}</div>
                    <div class={classes!("stat-value", net_worth_class)}>{format_currency(net_worth)}</div>
                    <div class="stat-desc">{"All accounts"}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Liquid"}</div>
                    <div class={classes!("stat-value", liquid_class)}>{format_currency(liquid_net_worth)}</div>
                    <div class="stat-desc">{"Cash & liquid assets"}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Non-Liquid"}</div>
                    <div class="stat-value text-secondary">{format_currency(non_liquid_net_worth)}</div>
                    <div class="stat-desc">{"House, equity, etc."}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Avg. Monthly Income"}</div>
                    <div class="stat-value text-success">{format_currency(income)}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Avg. Monthly Expenses"}</div>
                    <div class="stat-value text-error">{format_currency(expenses)}</div>
                </div>
            </div>
        </div>
    }
}

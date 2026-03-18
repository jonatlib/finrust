use crate::api_client::account::{get_accounts_with_ignored, AccountResponse};
use crate::api_client::metrics::get_dashboard_metrics;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::formatting::{fmt_amount, fmt_amount_opt};
use crate::hooks::FetchState;
use common::metrics::{AccountKindMetricsDto, DashboardMetricsDto};
use rust_decimal::Decimal;
use std::collections::HashMap;
use yew::prelude::*;

/// Formats a Decimal as a percentage (assumes the value is already 0-1 scale).
fn fmt_percent(value: Option<Decimal>) -> String {
    match value {
        Some(d) => {
            let pct = d * Decimal::from(100);
            format!("{:.1}%", pct)
        }
        None => "N/A".to_string(),
    }
}

/// Formats an optional Decimal as months (e.g. "4.9 months").
fn fmt_months(value: Option<Decimal>) -> String {
    match value {
        Some(d) => format!("{:.1} months", d),
        None => "N/A".to_string(),
    }
}

#[function_component(DashboardMetrics)]
pub fn dashboard_metrics() -> Html {
    let (fetch_state, _refetch) = use_fetch_with_refetch(get_dashboard_metrics);
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));

    match (&*fetch_state, &*accounts_state) {
        (FetchState::Loading, _) | (_, FetchState::Loading) => html! {
            <div class="card bg-base-100 shadow">
                <div class="card-body">
                    <h2 class="card-title">{"Financial Metrics"}</h2>
                    <div class="flex justify-center items-center py-8">
                        <span class="loading loading-spinner loading-lg"></span>
                    </div>
                </div>
            </div>
        },
        (FetchState::Error(error), _) | (_, FetchState::Error(error)) => html! {
            <div class="card bg-base-100 shadow">
                <div class="card-body">
                    <h2 class="card-title">{"Financial Metrics"}</h2>
                    <div class="alert alert-error">
                        <span>{format!("Failed to load metrics: {}", error)}</span>
                    </div>
                </div>
            </div>
        },
        (FetchState::Success(dashboard), FetchState::Success(accounts)) => render_dashboard(dashboard, accounts),
        _ => html! { <></> },
    }
}

fn render_dashboard(d: &DashboardMetricsDto, accounts: &[AccountResponse]) -> Html {
    let net_worth_class = if d.total_net_worth >= Decimal::ZERO {
        "text-success"
    } else {
        "text-error"
    };

    let liquid_class = if d.liquid_net_worth >= Decimal::ZERO {
        "text-success"
    } else {
        "text-error"
    };

    let free_cf_class = if d.free_cashflow >= Decimal::ZERO {
        "text-success"
    } else {
        "text-error"
    };

    // Find main-account sweep potential (first RealAccount with operating metrics)
    let sweep_potential = d
        .account_metrics
        .iter()
        .filter(|m| m.account_kind == "RealAccount")
        .find_map(|m| {
            if let Some(AccountKindMetricsDto::Operating(op)) = &m.kind_metrics {
                op.sweep_potential
            } else {
                None
            }
        });

    // Find EF coverage (reserve accounts with essential coverage)
    let ef_coverage = d
        .account_metrics
        .iter()
        .filter(|m| m.account_kind == "Savings" || m.account_kind == "Goal" || m.account_kind == "EmergencyFund")
        .find_map(|m| {
            if let Some(AccountKindMetricsDto::Reserve(res)) = &m.kind_metrics {
                res.months_of_essential_coverage
            } else {
                None
            }
        });

    // Find debt info for mortgage refix readiness
    let debt_accounts: Vec<_> = d
        .account_metrics
        .iter()
        .filter(|m| m.account_kind == "Debt")
        .collect();
    let total_outstanding: Decimal = debt_accounts
        .iter()
        .filter_map(|m| {
            if let Some(AccountKindMetricsDto::Debt(debt)) = &m.kind_metrics {
                debt.outstanding_principal
            } else {
                None
            }
        })
        .sum();
    let has_debt = !debt_accounts.is_empty();

    let balance_stats = compute_balance_stats(d, accounts);

    html! {
        <div class="card bg-base-100 shadow">
            <div class="card-body">
                <h2 class="card-title">
                    <i class="fas fa-chart-pie text-primary"></i>
                    {"Financial Health Dashboard"}
                </h2>
                <p class="text-sm text-gray-500 mb-4">{"Key financial metrics across all accounts"}</p>

                // Balance-based asset allocation
                <h3 class="text-sm font-semibold mt-2 mb-1 opacity-70">{"Asset Allocation (by balance)"}</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-6 gap-3">
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Liquid Assets"}</div>
                        <div class="stat-value text-lg text-success">
                            {fmt_amount(balance_stats.liquid_assets)}
                        </div>
                        <div class="stat-desc text-xs">{format!("{} accounts", balance_stats.liquid_count)}</div>
                    </div>
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Non-Liquid Assets"}</div>
                        <div class="stat-value text-lg text-secondary">
                            {fmt_amount(balance_stats.non_liquid_assets)}
                        </div>
                        <div class="stat-desc text-xs">{format!("{} accounts", balance_stats.non_liquid_count)}</div>
                    </div>
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Investments"}</div>
                        <div class="stat-value text-lg text-primary">
                            {fmt_amount(balance_stats.investment_balance)}
                        </div>
                        <div class="stat-desc text-xs">
                            {"Gain: "}{fmt_amount_opt(balance_stats.investment_gain)}
                        </div>
                    </div>
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Equity"}</div>
                        <div class="stat-value text-lg text-accent">
                            {fmt_amount(balance_stats.equity_balance)}
                        </div>
                        <div class="stat-desc text-xs">{format!("{} accounts", balance_stats.equity_count)}</div>
                    </div>
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Total Debt"}</div>
                        <div class={classes!("stat-value", "text-lg", if balance_stats.debt_balance != Decimal::ZERO { "text-error" } else { "text-success" })}>
                            {fmt_amount(balance_stats.debt_balance)}
                        </div>
                        <div class="stat-desc text-xs">{format!("{} accounts", balance_stats.debt_count)}</div>
                    </div>
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"House / Property"}</div>
                        <div class="stat-value text-lg">
                            {fmt_amount(balance_stats.house_balance)}
                        </div>
                        <div class="stat-desc text-xs">{format!("{} accounts", balance_stats.house_count)}</div>
                    </div>
                </div>

                // Cashflow-based metrics
                <h3 class="text-sm font-semibold mt-4 mb-1 opacity-70">{"Cashflow Metrics"}</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-6 gap-3">
                    // 1. Household Net Worth
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Net Worth"}</div>
                        <div class={classes!("stat-value", "text-lg", net_worth_class)}>
                            {fmt_amount(d.total_net_worth)}
                        </div>
                        <div class="stat-desc text-xs">{"Assets minus debts"}</div>
                    </div>

                    // 2. Liquid Net Worth
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Liquid Net Worth"}</div>
                        <div class={classes!("stat-value", "text-lg", liquid_class)}>
                            {fmt_amount(d.liquid_net_worth)}
                        </div>
                        <div class="stat-desc text-xs">{"Cash & liquid assets"}</div>
                    </div>

                    // 3. Non-Liquid Net Worth
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Non-Liquid"}</div>
                        <div class="stat-value text-lg text-secondary">
                            {fmt_amount(d.non_liquid_net_worth)}
                        </div>
                        <div class="stat-desc text-xs">{"House, equity, etc."}</div>
                    </div>

                    // 4. Essential Burn Rate
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Essential Burn"}</div>
                        <div class="stat-value text-lg text-warning">
                            {fmt_amount(d.essential_burn_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"Monthly essentials"}</div>
                    </div>

                    // 5. Free Cashflow
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Free Cashflow"}</div>
                        <div class={classes!("stat-value", "text-lg", free_cf_class)}>
                            {fmt_amount(d.free_cashflow)}
                        </div>
                        <div class="stat-desc text-xs">{"Income minus expenses"}</div>
                    </div>

                    // 6. EF Coverage
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"EF Coverage"}</div>
                        <div class="stat-value text-lg text-info">
                            {fmt_months(ef_coverage)}
                        </div>
                        <div class="stat-desc text-xs">{"Emergency fund months"}</div>
                    </div>
                </div>

                // Second row: Ratios and engine
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-3 mt-3">
                    // 6. Goal Engine
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Goal Engine"}</div>
                        <div class="stat-value text-lg text-primary">
                            {fmt_amount(d.goal_engine)}
                        </div>
                        <div class="stat-desc text-xs">{"Monthly wealth building"}</div>
                    </div>

                    // 7. Commitment Ratio
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Commitment Ratio"}</div>
                        <div class={classes!("stat-value", "text-lg", commitment_color(d.commitment_ratio))}>
                            {fmt_percent(d.commitment_ratio)}
                        </div>
                        <div class="stat-desc text-xs">{"Fixed obligations / income"}</div>
                    </div>

                    // 8. Savings Rate (as proxy for discretionary leakage)
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Savings Rate"}</div>
                        <div class={classes!("stat-value", "text-lg", savings_color(d.savings_rate))}>
                            {fmt_percent(d.savings_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"(Income - spending) / income"}</div>
                    </div>

                    // 9. Sweep Potential
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Sweep Potential"}</div>
                        <div class="stat-value text-lg text-accent">
                            {fmt_amount_opt(sweep_potential)}
                        </div>
                        <div class="stat-desc text-xs">{"Main account surplus"}</div>
                    </div>

                    // 10. Debt / Mortgage Readiness
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Debt Outstanding"}</div>
                        <div class={classes!("stat-value", "text-lg", if has_debt { "text-error" } else { "text-success" })}>
                            {if has_debt { fmt_amount(total_outstanding) } else { "None".to_string() }}
                        </div>
                        <div class="stat-desc text-xs">
                            {if has_debt {
                                format!("{} debt account(s)", debt_accounts.len())
                            } else {
                                "Debt free!".to_string()
                            }}
                        </div>
                    </div>
                </div>

                // Liquidity & debt burden row
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mt-3">
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Liquidity Ratio"}</div>
                        <div class="stat-value text-lg text-info">
                            {fmt_months(d.liquidity_ratio_months)}
                        </div>
                        <div class="stat-desc text-xs">{"Liquid assets / essential burn"}</div>
                    </div>

                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Debt Burden"}</div>
                        <div class="stat-value text-lg">
                            {fmt_percent(d.total_debt_burden)}
                        </div>
                        <div class="stat-desc text-xs">{"Debt payments / income"}</div>
                    </div>

                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Full Burn Rate"}</div>
                        <div class="stat-value text-lg text-error">
                            {fmt_amount(d.full_burn_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"All monthly expenses"}</div>
                    </div>
                </div>
            </div>
        </div>
    }
}

struct BalanceStats {
    liquid_assets: Decimal,
    liquid_count: usize,
    non_liquid_assets: Decimal,
    non_liquid_count: usize,
    investment_balance: Decimal,
    investment_gain: Option<Decimal>,
    equity_balance: Decimal,
    equity_count: usize,
    debt_balance: Decimal,
    debt_count: usize,
    house_balance: Decimal,
    house_count: usize,
}

fn compute_balance_stats(d: &DashboardMetricsDto, accounts: &[AccountResponse]) -> BalanceStats {
    let liquid_map: HashMap<i32, bool> = accounts.iter().map(|a| (a.id, a.is_liquid)).collect();

    let mut stats = BalanceStats {
        liquid_assets: Decimal::ZERO,
        liquid_count: 0,
        non_liquid_assets: Decimal::ZERO,
        non_liquid_count: 0,
        investment_balance: Decimal::ZERO,
        investment_gain: None,
        equity_balance: Decimal::ZERO,
        equity_count: 0,
        debt_balance: Decimal::ZERO,
        debt_count: 0,
        house_balance: Decimal::ZERO,
        house_count: 0,
    };

    let mut total_gain = Decimal::ZERO;
    let mut has_gain = false;

    for m in &d.account_metrics {
        let is_liquid = liquid_map.get(&m.account_id).copied().unwrap_or(true);

        if is_liquid {
            stats.liquid_assets += m.current_balance;
            stats.liquid_count += 1;
        } else {
            stats.non_liquid_assets += m.current_balance;
            stats.non_liquid_count += 1;
        }

        match m.account_kind.as_str() {
            "Investment" => {
                stats.investment_balance += m.current_balance;
                if let Some(AccountKindMetricsDto::Investment(inv)) = &m.kind_metrics {
                    if let Some(gl) = inv.gain_loss_absolute {
                        total_gain += gl;
                        has_gain = true;
                    }
                }
            }
            "Equity" => {
                stats.equity_balance += m.current_balance;
                stats.equity_count += 1;
            }
            "Debt" => {
                stats.debt_balance += m.current_balance;
                stats.debt_count += 1;
            }
            "House" => {
                stats.house_balance += m.current_balance;
                stats.house_count += 1;
            }
            _ => {}
        }
    }

    stats.investment_gain = if has_gain { Some(total_gain) } else { None };
    stats
}

/// Returns a color class based on the commitment ratio value.
fn commitment_color(ratio: Option<Decimal>) -> &'static str {
    match ratio {
        Some(r) if r > Decimal::new(70, 2) => "text-error",
        Some(r) if r > Decimal::new(50, 2) => "text-warning",
        Some(_) => "text-success",
        None => "",
    }
}

/// Returns a color class based on the savings rate value.
fn savings_color(rate: Option<Decimal>) -> &'static str {
    match rate {
        Some(r) if r < Decimal::ZERO => "text-error",
        Some(r) if r < Decimal::new(10, 2) => "text-warning",
        Some(_) => "text-success",
        None => "",
    }
}

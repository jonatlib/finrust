use crate::api_client::account::{get_accounts_with_ignored, AccountResponse};
use crate::api_client::metrics::get_dashboard_metrics;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::formatting::{fmt_amount, fmt_amount_opt};
use crate::hooks::FetchState;
use common::metrics::{AccountKindMetricsDto, DashboardMetricsDto};
use rust_decimal::Decimal;
use std::collections::HashMap;
use web_sys::MouseEvent;
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum BreakdownCategory {
    Operating,
    Safety,
    Consumption,
    Wealth,
    Debt,
    Savings,
    Shock1M,
    Shock3M,
    Shock6M,
}

#[function_component(DashboardMetrics)]
pub fn dashboard_metrics() -> Html {
    let (fetch_state, _refetch) = use_fetch_with_refetch(get_dashboard_metrics);
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));

    let selected_category = use_state(|| BreakdownCategory::Operating);

    let select_operating = {
        let selected_category = selected_category.clone();
        Callback::from(move |_| selected_category.set(BreakdownCategory::Operating))
    };

    let select_safety = {
        let selected_category = selected_category.clone();
        Callback::from(move |_| selected_category.set(BreakdownCategory::Safety))
    };

    let select_consumption = {
        let selected_category = selected_category.clone();
        Callback::from(move |_| selected_category.set(BreakdownCategory::Consumption))
    };

    let select_wealth = {
        let selected_category = selected_category.clone();
        Callback::from(move |_| selected_category.set(BreakdownCategory::Wealth))
    };

    let select_debt = {
        let selected_category = selected_category.clone();
        Callback::from(move |_| selected_category.set(BreakdownCategory::Debt))
    };

    let select_savings = {
        let selected_category = selected_category.clone();
        Callback::from(move |_| selected_category.set(BreakdownCategory::Savings))
    };

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
        (FetchState::Success(dashboard), FetchState::Success(accounts)) => render_dashboard(
            dashboard,
            accounts,
            &selected_category,
            &select_operating,
            &select_safety,
            &select_consumption,
            &select_wealth,
            &select_debt,
            &select_savings,
        ),
        _ => html! { <></> },
    }
}

fn render_dashboard(
    d: &DashboardMetricsDto,
    accounts: &[AccountResponse],
    selected_category: &UseStateHandle<BreakdownCategory>,
    select_operating: &Callback<MouseEvent>,
    select_safety: &Callback<MouseEvent>,
    select_consumption: &Callback<MouseEvent>,
    select_wealth: &Callback<MouseEvent>,
    select_debt: &Callback<MouseEvent>,
    select_savings: &Callback<MouseEvent>,
) -> Html {
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

                // Advanced Cashflow Analysis (NEW METRICS)
                <h3 class="text-sm font-semibold mt-4 mb-1 opacity-70">
                    {"Advanced Cashflow Analysis"}
                    <span class="badge badge-sm badge-primary ml-2">{"NEW"}</span>
                </h3>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                    // Operating Free Cashflow (THE REAL NUMBER)
                    <button
                        class={classes!(
                            "stat", "bg-base-200", "rounded-lg", "p-3",
                            "border-2", "cursor-pointer", "hover:shadow-md", "transition-all",
                            if **selected_category == BreakdownCategory::Operating {
                                "border-primary"
                            } else {
                                "border-transparent"
                            }
                        )}
                        onclick={select_operating.clone()}
                    >
                        <div class="stat-title text-xs font-bold">{"Operating Free Cashflow"}</div>
                        <div class={classes!(
                            "stat-value", "text-lg",
                            if d.operating_free_cashflow.unwrap_or(Decimal::ZERO) >= Decimal::ZERO {
                                "text-success"
                            } else {
                                "text-error"
                            }
                        )}>
                            {fmt_amount_opt(d.operating_free_cashflow)}
                        </div>
                        <div class="stat-desc text-xs">
                            {"Sum of operating account flows"}
                        </div>
                    </button>

                    // Safety Reserve Rate
                    <button
                        class={classes!(
                            "stat", "bg-base-200", "rounded-lg", "p-3",
                            "border-2", "cursor-pointer", "hover:shadow-md", "transition-all",
                            if **selected_category == BreakdownCategory::Safety {
                                "border-info"
                            } else {
                                "border-transparent"
                            }
                        )}
                        onclick={select_safety.clone()}
                    >
                        <div class="stat-title text-xs">{"Safety Reserve Rate"}</div>
                        <div class="stat-value text-lg text-info">
                            {fmt_amount_opt(d.safety_reserve_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"Emergency + income smoothing"}</div>
                    </button>

                    // Consumption Goal Rate
                    <button
                        class={classes!(
                            "stat", "bg-base-200", "rounded-lg", "p-3",
                            "border-2", "cursor-pointer", "hover:shadow-md", "transition-all",
                            if **selected_category == BreakdownCategory::Consumption {
                                "border-warning"
                            } else {
                                "border-transparent"
                            }
                        )}
                        onclick={select_consumption.clone()}
                    >
                        <div class="stat-title text-xs">{"Consumption Goal Rate"}</div>
                        <div class="stat-value text-lg text-warning">
                            {fmt_amount_opt(d.consumption_goal_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"Sinking funds + allowances (WILL BE SPENT)"}</div>
                    </button>

                    // Wealth Building Rate (TRUE wealth)
                    <button
                        class={classes!(
                            "stat", "bg-base-200", "rounded-lg", "p-3",
                            "border-2", "cursor-pointer", "hover:shadow-md", "transition-all",
                            if **selected_category == BreakdownCategory::Wealth {
                                "border-success"
                            } else {
                                "border-transparent"
                            }
                        )}
                        onclick={select_wealth.clone()}
                    >
                        <div class="stat-title text-xs">{"Wealth Building Rate"}</div>
                        <div class="stat-value text-lg text-success">
                            {fmt_amount_opt(d.wealth_building_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"True long-term investments only"}</div>
                    </button>

                    // Debt Payment Rate
                    <button
                        class={classes!(
                            "stat", "bg-base-200", "rounded-lg", "p-3",
                            "border-2", "cursor-pointer", "hover:shadow-md", "transition-all",
                            if **selected_category == BreakdownCategory::Debt {
                                "border-error"
                            } else {
                                "border-transparent"
                            }
                        )}
                        onclick={select_debt.clone()}
                    >
                        <div class="stat-title text-xs">{"Debt Payments"}</div>
                        <div class="stat-value text-lg text-error">
                            {fmt_amount_opt(d.debt_payment_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"Monthly debt payments (mandatory)"}</div>
                    </button>

                    // Savings Rate
                    <button
                        class={classes!(
                            "stat", "bg-base-200", "rounded-lg", "p-3",
                            "border-2", "cursor-pointer", "hover:shadow-md", "transition-all",
                            if **selected_category == BreakdownCategory::Savings {
                                "border-accent"
                            } else {
                                "border-transparent"
                            }
                        )}
                        onclick={select_savings.clone()}
                    >
                        <div class="stat-title text-xs">{"Savings Rate"}</div>
                        <div class="stat-value text-lg text-accent">
                            {fmt_amount_opt(d.savings_rate_category)}
                        </div>
                        <div class="stat-desc text-xs">{"Savings/Goal account contributions"}</div>
                    </button>
                </div>

                // Category Breakdown - Dynamic based on selection
                {match **selected_category {
                    BreakdownCategory::Operating => {
                        if let Some(ref breakdown) = d.operating_free_cashflow_breakdown {
                            html! {
                                <>
                                    <h3 class="text-sm font-semibold mt-4 mb-1 opacity-70">
                                        {"Operating Free Cashflow Breakdown"}
                                    </h3>
                                    <div class="bg-base-200 rounded-lg p-4">
                                        <table class="table table-sm">
                                            <thead>
                                                <tr>
                                                    <th class="text-xs">{"Account"}</th>
                                                    <th class="text-xs text-right">{"This Month"}</th>
                                                    <th class="text-xs text-right">{"3-mo Avg"}</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {for breakdown.contributions.iter().map(|contrib| {
                                                    let is_negative = contrib.net_flow.is_sign_negative();
                                                    html! {
                                                        <tr>
                                                            <td class="text-xs opacity-70">{format!("{} ({})", contrib.account_name, contrib.account_kind)}</td>
                                                            <td class={classes!(
                                                                "text-right", "font-mono", "text-sm",
                                                                if is_negative { "text-error" } else { "text-success" }
                                                            )}>
                                                                {fmt_amount(contrib.net_flow)}
                                                            </td>
                                                            <td class="text-right font-mono text-sm opacity-70">
                                                                {fmt_amount_opt(contrib.three_month_avg_net_flow)}
                                                            </td>
                                                        </tr>
                                                    }
                                                })}
                                                <tr class="border-t-2">
                                                    <td class="text-xs font-bold">{"= Total"}</td>
                                                    <td class={classes!(
                                                        "text-right", "font-mono", "font-bold", "text-sm",
                                                        if breakdown.total.is_sign_negative() { "text-error" } else { "text-success" }
                                                    )}>
                                                        {fmt_amount(breakdown.total)}
                                                    </td>
                                                    <td></td>
                                                </tr>
                                            </tbody>
                                        </table>
                                    </div>
                                </>
                            }
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Safety => {
                        if let Some(ref breakdown) = d.safety_reserve_rate_breakdown {
                            render_category_breakdown("Safety Reserve Breakdown", breakdown, "info")
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Consumption => {
                        if let Some(ref breakdown) = d.consumption_goal_rate_breakdown {
                            render_category_breakdown("Consumption Goal Breakdown", breakdown, "warning")
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Wealth => {
                        if let Some(ref breakdown) = d.wealth_building_rate_breakdown {
                            render_category_breakdown("Wealth Building Breakdown", breakdown, "success")
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Debt => {
                        if let Some(ref breakdown) = d.debt_payment_rate_breakdown {
                            render_category_breakdown("Debt Payment Breakdown", breakdown, "error")
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Savings => {
                        if let Some(ref breakdown) = d.savings_rate_breakdown {
                            render_category_breakdown("Savings Rate Breakdown", breakdown, "accent")
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Shock1M => {
                        if let Some(ref details) = d.shock_readiness_1m_details {
                            render_shock_breakdown("1-Month Shock Reserve Breakdown", details)
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Shock3M => {
                        if let Some(ref details) = d.shock_readiness_3m_details {
                            render_shock_breakdown("3-Month Shock Reserve Breakdown", details)
                        } else {
                            html! {}
                        }
                    },
                    BreakdownCategory::Shock6M => {
                        if let Some(ref details) = d.shock_readiness_6m_details {
                            render_shock_breakdown("6-Month Shock Reserve Breakdown", details)
                        } else {
                            html! {}
                        }
                    },
                }}

                // Shock Readiness (NEW METRICS)
                <h3 class="text-sm font-semibold mt-4 mb-1 opacity-70">
                    {"Shock Readiness"}
                    <span class="badge badge-sm badge-warning ml-2">{"CRITICAL"}</span>
                </h3>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    {render_shock_readiness_card("1-Month", d.shock_readiness_1m, &d.shock_readiness_1m_details, &selected_category, &Callback::from({
                        let selected_category = selected_category.clone();
                        move |_| selected_category.set(BreakdownCategory::Shock1M)
                    }), **selected_category == BreakdownCategory::Shock1M)}
                    {render_shock_readiness_card("3-Month", d.shock_readiness_3m, &d.shock_readiness_3m_details, &selected_category, &Callback::from({
                        let selected_category = selected_category.clone();
                        move |_| selected_category.set(BreakdownCategory::Shock3M)
                    }), **selected_category == BreakdownCategory::Shock3M)}
                    {render_shock_readiness_card("6-Month", d.shock_readiness_6m, &d.shock_readiness_6m_details, &selected_category, &Callback::from({
                        let selected_category = selected_category.clone();
                        move |_| selected_category.set(BreakdownCategory::Shock6M)
                    }), **selected_category == BreakdownCategory::Shock6M)}
                </div>

                // Info alert explaining the new metrics
                <div class="alert alert-info mt-4">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                    <div>
                        <h3 class="font-bold">{"New Advanced Metrics Explained"}</h3>
                        <div class="text-xs">
                            <p>{"• Operating Free Cashflow: The REAL number = operating net flow + true wealth transfers (excludes sinking funds, tax)"}</p>
                            <p>{"• Safety Reserve Rate: Emergency funds + income smoothing buffers"}</p>
                            <p>{"• Consumption Goal Rate: Sinking funds + allowances (will be spent)"}</p>
                            <p>{"• Wealth Building Rate: True long-term investments only"}</p>
                            <p>{"• Tax reserves are treated as mandatory spending (like paying taxes directly), not counted in any goal category"}</p>
                            <p>{"• Shock Readiness: Uses ONLY true emergency reserves + operating buffers (excludes house, investments, earmarked funds)"}</p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Renders a shock readiness card with progress bar and details
fn render_shock_readiness_card(
    timeframe: &str,
    ready: Option<bool>,
    details: &Option<common::metrics::ShockReadinessDetailsDto>,
    _selected_category: &UseStateHandle<BreakdownCategory>,
    onclick: &Callback<MouseEvent>,
    is_selected: bool,
) -> Html {
    use crate::formatting::fmt_amount;

    let is_ready = ready.unwrap_or(false);
    let status_class = if is_ready { "text-success" } else { "text-error" };

    html! {
        <button
            class={classes!(
                "stat", "bg-base-200", "rounded-lg", "p-3",
                "border-2", "cursor-pointer", "hover:shadow-md", "transition-all", "text-left",
                if is_selected { "border-warning" } else { "border-transparent" }
            )}
            onclick={onclick.clone()}
        >
            <div class="stat-title text-xs">{format!("{} Income Disruption", timeframe)}</div>
            <div class={classes!("stat-value", "text-2xl", status_class)}>
                {if is_ready { "✓ READY" } else { "✗ NOT READY" }}
            </div>

            {if let Some(ref d) = details {
                let progress_pct = (d.progress_ratio * rust_decimal::Decimal::new(100, 0))
                    .to_string()
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    .min(100.0);

                html! {
                    <>
                        <div class="mt-2">
                            <div class="text-xs opacity-70 mb-1">
                                {format!("{} of {} ({:.0}%)",
                                    fmt_amount(d.current_reserves),
                                    fmt_amount(d.target_reserves),
                                    progress_pct
                                )}
                            </div>
                            <progress
                                class={classes!(
                                    "progress", "w-full",
                                    if is_ready { "progress-success" } else { "progress-error" }
                                )}
                                value={progress_pct.to_string()}
                                max="100"
                            />
                        </div>
                        {if let Some(ref proj_date) = d.projected_date {
                            html! {
                                <div class="text-xs opacity-70 mt-1">
                                    {format!("Target: {}", proj_date)}
                                </div>
                            }
                        } else {
                            html! {}
                        }}
                    </>
                }
            } else {
                html! {
                    <div class="stat-desc text-xs">
                        {format!("Can survive {} with true reserves", timeframe.to_lowercase())}
                    </div>
                }
            }}
        </button>
    }
}

/// Renders shock reserve account breakdown
fn render_shock_breakdown(title: &str, details: &common::metrics::ShockReadinessDetailsDto) -> Html {
    use crate::formatting::fmt_amount;

    html! {
        <>
            <h3 class="text-sm font-semibold mt-4 mb-1 opacity-70">
                {title}
            </h3>
            <div class="bg-base-200 rounded-lg p-4">
                <table class="table table-sm">
                    <thead>
                        <tr>
                            <th class="text-xs">{"Account"}</th>
                            <th class="text-xs text-right">{"Balance"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for details.account_contributions.iter().map(|account| {
                            html! {
                                <tr>
                                    <td class="text-xs opacity-70">{format!("{} ({})", account.account_name, account.account_kind)}</td>
                                    <td class="text-right font-mono text-sm text-success">
                                        {fmt_amount(account.balance)}
                                    </td>
                                </tr>
                            }
                        })}
                        <tr class="border-t-2">
                            <td class="text-xs font-bold">{"= Total Shock Reserves"}</td>
                            <td class="text-right font-mono font-bold text-sm text-info">
                                {fmt_amount(details.current_reserves)}
                            </td>
                        </tr>
                        <tr>
                            <td class="text-xs opacity-70">{format!("Target ({} months × monthly burn)", details.months)}</td>
                            <td class="text-right font-mono text-sm opacity-70">
                                {fmt_amount(details.target_reserves)}
                            </td>
                        </tr>
                    </tbody>
                </table>
                <div class="text-xs opacity-70 mt-2">
                    {"Emergency fund accounts + Operating account buffers"}
                </div>
            </div>
        </>
    }
}

/// Renders a generic category breakdown table
fn render_category_breakdown(title: &str, breakdown: &common::metrics::CategoryBreakdownDto, color: &str) -> Html {
    use crate::formatting::fmt_amount;

    let total_color = match color {
        "info" => "text-info",
        "warning" => "text-warning",
        "success" => "text-success",
        "error" => "text-error",
        "accent" => "text-accent",
        _ => "text-primary",
    };

    html! {
        <>
            <h3 class="text-sm font-semibold mt-4 mb-1 opacity-70">
                {title}
            </h3>
            <div class="bg-base-200 rounded-lg p-4">
                <table class="table table-sm">
                    <thead>
                        <tr>
                            <th class="text-xs">{"Account"}</th>
                            <th class="text-xs text-right">{"This Month"}</th>
                            <th class="text-xs text-right">{"3-mo Avg"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for breakdown.contributions.iter().map(|contrib| {
                            html! {
                                <tr>
                                    <td class="text-xs opacity-70">{format!("{} ({})", contrib.account_name, contrib.account_kind)}</td>
                                    <td class="text-right font-mono text-sm text-success">
                                        {fmt_amount(contrib.net_flow)}
                                    </td>
                                    <td class="text-right font-mono text-sm opacity-70">
                                        {fmt_amount_opt(contrib.three_month_avg_net_flow)}
                                    </td>
                                </tr>
                            }
                        })}
                        <tr class="border-t-2">
                            <td class="text-xs font-bold">{"= Total"}</td>
                            <td class={classes!("text-right", "font-mono", "font-bold", "text-sm", total_color)}>
                                {fmt_amount(breakdown.total)}
                            </td>
                            <td></td>
                        </tr>
                    </tbody>
                </table>
                <div class="text-xs opacity-70 mt-2">
                    {&breakdown.description}
                </div>
            </div>
        </>
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

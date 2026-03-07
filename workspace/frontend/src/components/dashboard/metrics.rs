use crate::api_client::metrics::get_dashboard_metrics;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use common::metrics::{AccountKindMetricsDto, DashboardMetricsDto};
use rust_decimal::Decimal;
use yew::prelude::*;

/// Formats a Decimal value as a currency string with 2 decimal places.
fn fmt_currency(amount: Decimal) -> String {
    format!("{:.2}", amount)
}

/// Formats an optional Decimal as currency or "N/A".
fn fmt_opt(value: Option<Decimal>) -> String {
    match value {
        Some(d) => fmt_currency(d),
        None => "N/A".to_string(),
    }
}

/// Formats a Decimal as a percentage (assumes the value is already 0–1 scale).
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

/// Renders the top-10 financial metrics on the dashboard.
#[function_component(DashboardMetrics)]
pub fn dashboard_metrics() -> Html {
    let (fetch_state, _refetch) = use_fetch_with_refetch(get_dashboard_metrics);

    match &*fetch_state {
        FetchState::Loading => html! {
            <div class="card bg-base-100 shadow">
                <div class="card-body">
                    <h2 class="card-title">{"Financial Metrics"}</h2>
                    <div class="flex justify-center items-center py-8">
                        <span class="loading loading-spinner loading-lg"></span>
                    </div>
                </div>
            </div>
        },
        FetchState::Error(error) => html! {
            <div class="card bg-base-100 shadow">
                <div class="card-body">
                    <h2 class="card-title">{"Financial Metrics"}</h2>
                    <div class="alert alert-error">
                        <span>{format!("Failed to load metrics: {}", error)}</span>
                    </div>
                </div>
            </div>
        },
        FetchState::Success(dashboard) => render_dashboard(dashboard),
        FetchState::NotStarted => html! { <></> },
    }
}

fn render_dashboard(d: &DashboardMetricsDto) -> Html {
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

    // Find EF coverage (Savings/Goal accounts with reserve metrics that have essential coverage)
    let ef_coverage = d
        .account_metrics
        .iter()
        .filter(|m| m.account_kind == "Savings" || m.account_kind == "Goal")
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

    html! {
        <div class="card bg-base-100 shadow">
            <div class="card-body">
                <h2 class="card-title">
                    <i class="fas fa-chart-pie text-primary"></i>
                    {"Financial Health Dashboard"}
                </h2>
                <p class="text-sm text-gray-500 mb-4">{"Key financial metrics across all accounts"}</p>

                // Top row: Net worth metrics
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-3">
                    // 1. Household Net Worth
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Net Worth"}</div>
                        <div class={classes!("stat-value", "text-lg", net_worth_class)}>
                            {fmt_currency(d.total_net_worth)}
                        </div>
                        <div class="stat-desc text-xs">{"Assets minus debts"}</div>
                    </div>

                    // 2. Liquid Net Worth
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Liquid Net Worth"}</div>
                        <div class={classes!("stat-value", "text-lg", liquid_class)}>
                            {fmt_currency(d.liquid_net_worth)}
                        </div>
                        <div class="stat-desc text-xs">{"Liquid accounts only"}</div>
                    </div>

                    // 3. Essential Burn Rate
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Essential Burn"}</div>
                        <div class="stat-value text-lg text-warning">
                            {fmt_currency(d.essential_burn_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"Monthly essentials"}</div>
                    </div>

                    // 4. Free Cashflow
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Free Cashflow"}</div>
                        <div class={classes!("stat-value", "text-lg", free_cf_class)}>
                            {fmt_currency(d.free_cashflow)}
                        </div>
                        <div class="stat-desc text-xs">{"Income minus expenses"}</div>
                    </div>

                    // 5. EF Coverage
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
                            {fmt_currency(d.goal_engine)}
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
                            {fmt_opt(sweep_potential)}
                        </div>
                        <div class="stat-desc text-xs">{"Main account surplus"}</div>
                    </div>

                    // 10. Debt / Mortgage Readiness
                    <div class="stat bg-base-200 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Debt Outstanding"}</div>
                        <div class={classes!("stat-value", "text-lg", if has_debt { "text-error" } else { "text-success" })}>
                            {if has_debt { fmt_currency(total_outstanding) } else { "None".to_string() }}
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
                            {fmt_currency(d.full_burn_rate)}
                        </div>
                        <div class="stat-desc text-xs">{"All monthly expenses"}</div>
                    </div>
                </div>
            </div>
        </div>
    }
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

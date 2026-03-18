use crate::api_client::metrics::get_account_metrics;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::formatting::{fmt_amount, fmt_amount_opt};
use crate::hooks::FetchState;
use common::metrics::{AccountKindMetricsDto, AccountMetricsDto};
use rust_decimal::Decimal;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account_id: i32,
}

/// Renders per-account financial metrics including kind-specific details.
#[function_component(AccountMetrics)]
pub fn account_metrics_component(props: &Props) -> Html {
    let account_id = props.account_id;
    let (fetch_state, _refetch) =
        use_fetch_with_refetch(move || get_account_metrics(account_id));

    html! {
        <div class="card bg-base-100 shadow mt-6">
            <div class="card-body">
                <h3 class="card-title text-lg">
                    <i class="fas fa-chart-bar text-primary"></i>
                    {"Financial Metrics"}
                </h3>

                {match &*fetch_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-warning mt-2">
                            <span>{format!("Metrics unavailable: {}", error)}</span>
                        </div>
                    },
                    FetchState::Success(metrics) => render_account_metrics(metrics),
                    FetchState::NotStarted => html! { <></> },
                }}
            </div>
        </div>
    }
}

fn fmt_percent(value: Option<Decimal>) -> String {
    match value {
        Some(d) => {
            let pct = d * Decimal::from(100);
            format!("{:.1}%", pct)
        }
        None => "N/A".to_string(),
    }
}

fn render_account_metrics(m: &AccountMetricsDto) -> Html {
    let balance_class = if m.current_balance >= Decimal::ZERO {
        "text-success"
    } else {
        "text-error"
    };

    let flow_class = |v: Option<Decimal>| -> &'static str {
        match v {
            Some(d) if d >= Decimal::ZERO => "text-success",
            Some(_) => "text-error",
            None => "",
        }
    };

    html! {
        <>
            // Universal metrics
            <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-3 mt-4">
                <div class="stat bg-base-200 rounded-lg p-3">
                    <div class="stat-title text-xs">{"Current Balance"}</div>
                    <div class={classes!("stat-value", "text-lg", balance_class)}>
                        {fmt_amount(m.current_balance)}
                    </div>
                </div>

                {if let Some(target) = m.target_balance {
                    html! {
                        <div class="stat bg-base-200 rounded-lg p-3">
                            <div class="stat-title text-xs">{"Target Balance"}</div>
                            <div class="stat-value text-lg">{fmt_amount(target)}</div>
                        </div>
                    }
                } else {
                    html! {}
                }}

                {if m.funding_ratio.is_some() {
                    html! {
                        <div class="stat bg-base-200 rounded-lg p-3">
                            <div class="stat-title text-xs">{"Funding Ratio"}</div>
                            <div class={classes!("stat-value", "text-lg", funding_color(m.funding_ratio))}>
                                {fmt_percent(m.funding_ratio)}
                            </div>
                            <div class="stat-desc text-xs">{"Current / target"}</div>
                        </div>
                    }
                } else {
                    html! {}
                }}

                <div class="stat bg-base-200 rounded-lg p-3">
                    <div class="stat-title text-xs">{"Monthly Net Flow"}</div>
                    <div class={classes!("stat-value", "text-lg", flow_class(m.monthly_net_flow))}>
                        {fmt_amount_opt(m.monthly_net_flow)}
                    </div>
                    <div class="stat-desc text-xs">{"This month"}</div>
                </div>

                <div class="stat bg-base-200 rounded-lg p-3">
                    <div class="stat-title text-xs">{"3M Avg Net Flow"}</div>
                    <div class={classes!("stat-value", "text-lg", flow_class(m.three_month_avg_net_flow))}>
                        {fmt_amount_opt(m.three_month_avg_net_flow)}
                    </div>
                    <div class="stat-desc text-xs">{"Rolling 3-month average"}</div>
                </div>

                <div class="stat bg-base-200 rounded-lg p-3">
                    <div class="stat-title text-xs">{"Flow Volatility"}</div>
                    <div class="stat-value text-lg">
                        {fmt_amount_opt(m.flow_volatility)}
                    </div>
                    <div class="stat-desc text-xs">{"Std deviation of net flow"}</div>
                </div>
            </div>

            // Kind-specific metrics
            {if let Some(kind) = &m.kind_metrics {
                render_kind_metrics(kind)
            } else {
                html! {}
            }}
        </>
    }
}

fn render_kind_metrics(kind: &AccountKindMetricsDto) -> Html {
    match kind {
        AccountKindMetricsDto::Operating(op) => html! {
            <div class="mt-4">
                <h4 class="font-semibold text-sm mb-2 text-gray-600">{"Operating Account Metrics"}</h4>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div class="stat bg-primary/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Operating Buffer"}</div>
                        <div class={classes!("stat-value", "text-lg", buffer_color(op.operating_buffer))}>
                            {fmt_amount_opt(op.operating_buffer)}
                        </div>
                        <div class="stat-desc text-xs">{"Balance above target"}</div>
                    </div>

                    <div class="stat bg-primary/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Sweep Potential"}</div>
                        <div class="stat-value text-lg text-accent">
                            {fmt_amount_opt(op.sweep_potential)}
                        </div>
                        <div class="stat-desc text-xs">{"Safe to sweep out"}</div>
                    </div>

                    <div class="stat bg-primary/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Mandatory Coverage"}</div>
                        <div class="stat-value text-lg">
                            {match op.mandatory_coverage_months {
                                Some(d) => format!("{:.1} months", d),
                                None => "N/A".to_string(),
                            }}
                        </div>
                        <div class="stat-desc text-xs">{"Months of obligations covered"}</div>
                    </div>
                </div>
            </div>
        },
        AccountKindMetricsDto::Reserve(res) => html! {
            <div class="mt-4">
                <h4 class="font-semibold text-sm mb-2 text-gray-600">{"Reserve Account Metrics"}</h4>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                    {if let Some(date) = res.goal_reached_date {
                        html! {
                            <div class="stat bg-info/10 rounded-lg p-3">
                                <div class="stat-title text-xs">{"Goal Reached Date"}</div>
                                <div class="stat-value text-lg text-info">
                                    {date.format("%Y-%m-%d").to_string()}
                                </div>
                                <div class="stat-desc text-xs">{"Projected completion"}</div>
                            </div>
                        }
                    } else {
                        html! {
                            <div class="stat bg-info/10 rounded-lg p-3">
                                <div class="stat-title text-xs">{"Goal Reached Date"}</div>
                                <div class="stat-value text-lg text-gray-400">{"Not projected"}</div>
                            </div>
                        }
                    }}

                    <div class="stat bg-info/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Essential Coverage"}</div>
                        <div class="stat-value text-lg text-info">
                            {match res.months_of_essential_coverage {
                                Some(d) => format!("{:.1} months", d),
                                None => "N/A".to_string(),
                            }}
                        </div>
                        <div class="stat-desc text-xs">{"Months of essentials this covers"}</div>
                    </div>
                </div>
            </div>
        },
        AccountKindMetricsDto::Investment(inv) => html! {
            <div class="mt-4">
                <h4 class="font-semibold text-sm mb-2 text-gray-600">{"Investment Metrics"}</h4>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div class="stat bg-success/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Net Contributions"}</div>
                        <div class="stat-value text-lg">
                            {fmt_amount_opt(inv.net_contributions)}
                        </div>
                        <div class="stat-desc text-xs">{"Total invested"}</div>
                    </div>

                    <div class="stat bg-success/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Gain / Loss"}</div>
                        <div class={classes!("stat-value", "text-lg", gain_color(inv.gain_loss_absolute))}>
                            {fmt_amount_opt(inv.gain_loss_absolute)}
                        </div>
                        <div class="stat-desc text-xs">{"Absolute"}</div>
                    </div>

                    <div class="stat bg-success/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Return %"}</div>
                        <div class={classes!("stat-value", "text-lg", gain_color(inv.gain_loss_percent))}>
                            {fmt_percent_direct(inv.gain_loss_percent)}
                        </div>
                        <div class="stat-desc text-xs">{"Percentage return"}</div>
                    </div>
                </div>
            </div>
        },
        AccountKindMetricsDto::Debt(debt) => html! {
            <div class="mt-4">
                <h4 class="font-semibold text-sm mb-2 text-gray-600">{"Debt Metrics"}</h4>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div class="stat bg-error/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Outstanding Principal"}</div>
                        <div class="stat-value text-lg text-error">
                            {fmt_amount_opt(debt.outstanding_principal)}
                        </div>
                    </div>

                    <div class="stat bg-error/10 rounded-lg p-3">
                        <div class="stat-title text-xs">{"Monthly Payment"}</div>
                        <div class="stat-value text-lg">
                            {fmt_amount_opt(debt.required_monthly_payment)}
                        </div>
                        <div class="stat-desc text-xs">{"Required payment"}</div>
                    </div>

                    {if let Some(date) = debt.debt_free_date {
                        html! {
                            <div class="stat bg-error/10 rounded-lg p-3">
                                <div class="stat-title text-xs">{"Debt-Free Date"}</div>
                                <div class="stat-value text-lg text-success">
                                    {date.format("%Y-%m-%d").to_string()}
                                </div>
                                <div class="stat-desc text-xs">{"Projected payoff"}</div>
                            </div>
                        }
                    } else {
                        html! {
                            <div class="stat bg-error/10 rounded-lg p-3">
                                <div class="stat-title text-xs">{"Debt-Free Date"}</div>
                                <div class="stat-value text-lg text-gray-400">{"Not projected"}</div>
                            </div>
                        }
                    }}
                </div>
            </div>
        },
    }
}

fn funding_color(ratio: Option<Decimal>) -> &'static str {
    match ratio {
        Some(r) if r >= Decimal::ONE => "text-success",
        Some(r) if r >= Decimal::new(50, 2) => "text-warning",
        Some(_) => "text-error",
        None => "",
    }
}

fn buffer_color(buffer: Option<Decimal>) -> &'static str {
    match buffer {
        Some(b) if b >= Decimal::ZERO => "text-success",
        Some(_) => "text-error",
        None => "",
    }
}

fn gain_color(value: Option<Decimal>) -> &'static str {
    match value {
        Some(v) if v >= Decimal::ZERO => "text-success",
        Some(_) => "text-error",
        None => "",
    }
}

/// Formats a percentage value that is already in percentage form (e.g. 15.5 -> "15.5%").
fn fmt_percent_direct(value: Option<Decimal>) -> String {
    match value {
        Some(d) => format!("{:.1}%", d),
        None => "N/A".to_string(),
    }
}

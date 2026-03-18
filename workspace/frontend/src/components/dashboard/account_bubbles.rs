use crate::api_client::account::{get_accounts_with_ignored, AccountKind, AccountResponse};
use crate::api_client::metrics::get_dashboard_metrics;
use crate::api_client::statistics::{get_all_accounts_statistics, AccountStatisticsCollection};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::formatting::{fmt_amount_opt, fmt_amount_f64_int};
use crate::hooks::FetchState;
use crate::router::Route;
use common::metrics::DashboardMetricsDto;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use yew::prelude::*;
use yew_router::prelude::Link;

/// Compact per-account dashboard bubbles grouped by account type.
#[function_component(AccountTypeBubbles)]
pub fn account_type_bubbles() -> Html {
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));
    let (statistics_state, _) = use_fetch_with_refetch(get_all_accounts_statistics);
    let (metrics_state, _) = use_fetch_with_refetch(get_dashboard_metrics);

    match (&*accounts_state, &*statistics_state) {
        (FetchState::Loading, _) | (_, FetchState::Loading) => html! {
            <div class="flex justify-center items-center py-6">
                <span class="loading loading-spinner loading-md"></span>
            </div>
        },
        (FetchState::Error(accounts_error), _) => html! {
            <div class="alert alert-warning">
                <span>{format!("Unable to load accounts: {}", accounts_error)}</span>
            </div>
        },
        (_, FetchState::Error(statistics_error)) => html! {
            <div class="alert alert-warning">
                <span>{format!("Unable to load account balances: {}", statistics_error)}</span>
            </div>
        },
        (FetchState::Success(accounts), FetchState::Success(statistics)) => {
            if accounts.is_empty() {
                return html! {
                    <div class="text-sm text-gray-500">{"No accounts to display."}</div>
                };
            }

            let stats_by_account = build_stats_by_account_id(statistics);
            let grouped_accounts = group_accounts_by_kind(accounts);

            let avg_flows = match &*metrics_state {
                FetchState::Success(dashboard) => build_avg_flow_map(dashboard),
                _ => HashMap::new(),
            };

            html! {
                <div class="space-y-4">
                    <div class="flex flex-wrap gap-2">
                        {for grouped_accounts.keys().map(|kind| {
                            let style = kind_style(kind);
                            html! {
                                <span class={classes!("badge", "badge-sm", "gap-2", "badge-outline", style.legend_badge_class)}>
                                    <span class={classes!("inline-block", "w-2", "h-2", "rounded-full", style.legend_dot_class)}></span>
                                    {kind.display_name()}
                                </span>
                            }
                        })}
                    </div>

                    <div class="space-y-4">
                        {for grouped_accounts.iter().map(|(kind, grouped)| {
                            let section_stats = compute_section_stats(grouped, &stats_by_account, &avg_flows);
                            html! {
                                <div>
                                    <div class="flex items-baseline flex-wrap gap-x-4 gap-y-1 mb-2">
                                        <h3 class="text-sm font-semibold">{kind.display_name()}</h3>
                                        <span class="text-xs opacity-70">{format!("{} accounts", section_stats.count)}</span>
                                        <span class="text-xs font-medium">
                                            {"Total: "}{fmt_amount_opt(section_stats.total_balance)}
                                        </span>
                                        {if let Some(avg) = section_stats.avg_cash_flow {
                                            let (arrow, color) = if avg >= Decimal::ZERO {
                                                ("↑", "text-success")
                                            } else {
                                                ("↓", "text-error")
                                            };
                                            html! {
                                                <span class={classes!("text-xs", "font-medium", color)}>
                                                    {format!("Avg flow: {} {}/mo", arrow, fmt_amount_f64_int(avg.to_string().parse::<f64>().unwrap_or(0.0)))}
                                                </span>
                                            }
                                        } else {
                                            html! {}
                                        }}
                                    </div>
                                    <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-2">
                                        {for grouped.iter().enumerate().map(|(idx, account)| {
                                            let (current_state, month_end_state) = stats_by_account
                                                .get(&account.id)
                                                .cloned()
                                                .unwrap_or((None, None));

                                            let avg_flow = avg_flows.get(&account.id).copied().flatten();
                                            let accent = account.color.clone()
                                                .unwrap_or_else(|| crate::colors::color_by_index(idx).to_string());
                                            let border_style = format!("border-left: 4px solid {}", accent);
                                            let kind_style = kind_style(&account.account_kind);

                                            html! {
                                                <Link<Route>
                                                    to={Route::AccountEdit { id: account.id }}
                                                    classes={classes!(
                                                        "card",
                                                        "card-compact",
                                                        "border",
                                                        "shadow-sm",
                                                        "hover:shadow-md",
                                                        "transition-shadow",
                                                        "cursor-pointer",
                                                        "bg-base-100",
                                                    )}
                                                >
                                                    <div class="card-body p-3 gap-1" style={border_style}>
                                                        <div class="flex items-start justify-between gap-2">
                                                            <div class="flex items-center gap-2">
                                                                <h4 class="card-title text-sm leading-tight">{&account.name}</h4>
                                                                <span class={classes!("badge", "badge-xs", "gap-1", "badge-outline", kind_style.legend_badge_class)}>
                                                                    <span class={classes!("inline-block", "w-1.5", "h-1.5", "rounded-full", kind_style.legend_dot_class)}></span>
                                                                    {account.account_kind.display_name()}
                                                                </span>
                                                            </div>
                                                            <span class="badge badge-ghost badge-xs">{&account.currency_code}</span>
                                                        </div>

                                                        <div class="text-base font-bold">
                                                            {fmt_amount_opt(current_state)}
                                                        </div>

                                                        <div class="text-xs opacity-80 leading-tight">
                                                            {"End of month: "}
                                                            {fmt_amount_opt(month_end_state)}
                                                        </div>

                                                        {render_avg_flow_badge(avg_flow)}

                                                        <div class="card-actions justify-end">
                                                            <span class="link link-hover text-xs">{"Open detail"}</span>
                                                        </div>
                                                    </div>
                                                </Link<Route>>
                                            }
                                        })}
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                </div>
            }
        }
        _ => html! { <></> },
    }
}

/// Renders a small badge showing the 3-month average net flow.
fn render_avg_flow_badge(avg_flow: Option<Decimal>) -> Html {
    match avg_flow {
        Some(flow) => {
            let (arrow, color) = if flow >= Decimal::ZERO {
                ("↑", "text-success")
            } else {
                ("↓", "text-error")
            };
            html! {
                <div class={classes!("text-xs", "font-medium", color)} title="3-month average net flow">
                    {format!("{} {}/mo avg", arrow, fmt_amount_f64_int(flow.to_string().parse::<f64>().unwrap_or(0.0)))}
                </div>
            }
        }
        None => html! {},
    }
}

/// Visual classes used by account-kind groups and legend.
struct KindStyle {
    card_class: &'static str,
    legend_badge_class: &'static str,
    legend_dot_class: &'static str,
}

/// Returns visual styling for a specific account kind.
fn kind_style(kind: &AccountKind) -> KindStyle {
    match kind {
        AccountKind::RealAccount => KindStyle {
            card_class: "bg-blue-50 border-blue-200",
            legend_badge_class: "text-blue-700",
            legend_dot_class: "bg-blue-500",
        },
        AccountKind::Allowance => KindStyle {
            card_class: "bg-sky-50 border-sky-200",
            legend_badge_class: "text-sky-700",
            legend_dot_class: "bg-sky-500",
        },
        AccountKind::Shared => KindStyle {
            card_class: "bg-indigo-50 border-indigo-200",
            legend_badge_class: "text-indigo-700",
            legend_dot_class: "bg-indigo-500",
        },
        AccountKind::EmergencyFund => KindStyle {
            card_class: "bg-teal-50 border-teal-200",
            legend_badge_class: "text-teal-700",
            legend_dot_class: "bg-teal-500",
        },
        AccountKind::Savings => KindStyle {
            card_class: "bg-green-50 border-green-200",
            legend_badge_class: "text-green-700",
            legend_dot_class: "bg-green-500",
        },
        AccountKind::Investment => KindStyle {
            card_class: "bg-purple-50 border-purple-200",
            legend_badge_class: "text-purple-700",
            legend_dot_class: "bg-purple-500",
        },
        AccountKind::Equity => KindStyle {
            card_class: "bg-violet-50 border-violet-200",
            legend_badge_class: "text-violet-700",
            legend_dot_class: "bg-violet-500",
        },
        AccountKind::House => KindStyle {
            card_class: "bg-orange-50 border-orange-200",
            legend_badge_class: "text-orange-700",
            legend_dot_class: "bg-orange-500",
        },
        AccountKind::Debt => KindStyle {
            card_class: "bg-red-50 border-red-200",
            legend_badge_class: "text-red-700",
            legend_dot_class: "bg-red-500",
        },
        AccountKind::Tax => KindStyle {
            card_class: "bg-yellow-50 border-yellow-200",
            legend_badge_class: "text-yellow-700",
            legend_dot_class: "bg-yellow-500",
        },
        AccountKind::Other => KindStyle {
            card_class: "bg-gray-50 border-gray-200",
            legend_badge_class: "text-gray-700",
            legend_dot_class: "bg-gray-500",
        },
        AccountKind::Goal => KindStyle {
            card_class: "bg-amber-50 border-amber-200",
            legend_badge_class: "text-amber-700",
            legend_dot_class: "bg-amber-500",
        },
    }
}

/// Groups accounts into a deterministic map by account kind.
fn group_accounts_by_kind(accounts: &[AccountResponse]) -> BTreeMap<AccountKind, Vec<&AccountResponse>> {
    let mut grouped: BTreeMap<AccountKind, Vec<&AccountResponse>> = BTreeMap::new();

    for account in accounts {
        grouped.entry(account.account_kind).or_default().push(account);
    }

    for grouped_accounts in grouped.values_mut() {
        grouped_accounts.sort_by(|left, right| left.name.cmp(&right.name));
    }

    grouped
}

/// Builds a map of account ID to current and end-of-month states.
fn build_stats_by_account_id(
    statistics: &[AccountStatisticsCollection],
) -> HashMap<i32, (Option<Decimal>, Option<Decimal>)> {
    let mut stats_by_account = HashMap::new();

    for collection in statistics {
        for account_stats in &collection.statistics {
            let entry = stats_by_account
                .entry(account_stats.account_id)
                .or_insert((None, None));

            if entry.0.is_none() {
                entry.0 = account_stats.current_state;
            }
            if entry.1.is_none() {
                entry.1 = account_stats.end_of_current_month_state;
            }
        }
    }

    stats_by_account
}

/// Builds a map of account ID to three_month_avg_net_flow from dashboard metrics.
fn build_avg_flow_map(dashboard: &DashboardMetricsDto) -> HashMap<i32, Option<Decimal>> {
    dashboard
        .account_metrics
        .iter()
        .map(|m| (m.account_id, m.three_month_avg_net_flow))
        .collect()
}

struct SectionStats {
    count: usize,
    total_balance: Option<Decimal>,
    avg_cash_flow: Option<Decimal>,
}

fn compute_section_stats(
    accounts: &[&AccountResponse],
    stats_by_account: &HashMap<i32, (Option<Decimal>, Option<Decimal>)>,
    avg_flows: &HashMap<i32, Option<Decimal>>,
) -> SectionStats {
    let count = accounts.len();

    let mut total = Decimal::ZERO;
    let mut has_balance = false;
    for a in accounts {
        if let Some((Some(current), _)) = stats_by_account.get(&a.id) {
            total += current;
            has_balance = true;
        }
    }

    let mut flow_sum = Decimal::ZERO;
    let mut flow_count = 0u32;
    for a in accounts {
        if let Some(Some(flow)) = avg_flows.get(&a.id) {
            flow_sum += flow;
            flow_count += 1;
        }
    }

    SectionStats {
        count,
        total_balance: if has_balance { Some(total) } else { None },
        avg_cash_flow: if flow_count > 0 {
            Some(flow_sum / Decimal::from(flow_count))
        } else {
            None
        },
    }
}


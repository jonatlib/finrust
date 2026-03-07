use crate::api_client::account::{get_accounts, AccountKind, AccountResponse};
use crate::api_client::metrics::get_dashboard_metrics;
use crate::api_client::statistics::{get_all_accounts_statistics, AccountStatisticsCollection};
use crate::common::fetch_hook::use_fetch_with_refetch;
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
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
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
                            let style = kind_style(kind);

                            html! {
                                <div>
                                    <h3 class="text-sm font-semibold mb-2">{kind.display_name()}</h3>
                                    <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-2">
                                        {for grouped.iter().map(|account| {
                                            let (current_state, month_end_state) = stats_by_account
                                                .get(&account.id)
                                                .cloned()
                                                .unwrap_or((None, None));

                                            let avg_flow = avg_flows.get(&account.id).copied().flatten();

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
                                                        style.card_class,
                                                    )}
                                                >
                                                    <div class="card-body p-3 gap-1">
                                                        <div class="flex items-start justify-between gap-2">
                                                            <h4 class="card-title text-sm leading-tight">{&account.name}</h4>
                                                            <span class="badge badge-ghost badge-xs">{&account.currency_code}</span>
                                                        </div>

                                                        <div class="text-base font-bold">
                                                            {format_decimal_option(current_state)}
                                                        </div>

                                                        <div class="text-xs opacity-80 leading-tight">
                                                            {"End of month: "}
                                                            {format_decimal_option(month_end_state)}
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
                    {format!("{} {:.0}/mo avg", arrow, flow)}
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
        AccountKind::Debt => KindStyle {
            card_class: "bg-red-50 border-red-200",
            legend_badge_class: "text-red-700",
            legend_dot_class: "bg-red-500",
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

/// Formats optional monetary values for bubble labels.
fn format_decimal_option(value: Option<Decimal>) -> String {
    match value {
        Some(decimal) => format!("{:.2}", decimal),
        None => "N/A".to_string(),
    }
}

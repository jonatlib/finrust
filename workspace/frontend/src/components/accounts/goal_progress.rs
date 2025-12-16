use yew::prelude::*;
use crate::api_client::account::AccountResponse;
use crate::api_client::statistics::{AccountStatistics, AccountStatisticsCollection};
use crate::hooks::FetchState;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account: AccountResponse,
    pub stats_state: FetchState<AccountStatisticsCollection>,
}

#[function_component(GoalProgress)]
pub fn goal_progress(props: &Props) -> Html {
    let account = &props.account;

    html! {
        <div class="card bg-base-100 shadow">
            <div class="card-body">
                <h3 class="card-title text-lg">{"Goal Progress"}</h3>
                {match &props.stats_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-4">
                            <span class="loading loading-spinner loading-md"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-error mt-4">
                            <span>{format!("Failed to load goal data: {}", error)}</span>
                        </div>
                    },
                    FetchState::Success(collection) => {
                        render_goal_progress(account, collection.statistics.first())
                    },
                    FetchState::NotStarted => html! { <div class="text-sm text-gray-400">{"Loading..."}</div> },
                }}
            </div>
        </div>
    }
}

fn render_goal_progress(account: &AccountResponse, stats: Option<&AccountStatistics>) -> Html {
    if let Some(stats) = stats {
        if let Some(target_str) = &account.target_amount {
            if let Ok(target) = target_str.parse::<f64>() {
                if let Some(balance_decimal) = &stats.end_of_period_state {
                    if let Ok(current) = balance_decimal.to_string().parse::<f64>() {
                        let progress = (current / target * 100.0).min(100.0).max(0.0);
                        let remaining = target - current;

                        return html! {
                            <div class="mt-4 space-y-4">
                                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Current Amount"}</div>
                                        <div class="stat-value text-2xl text-primary">
                                            {format!("{:.2}", current)}
                                        </div>
                                        <div class="stat-desc">{&account.currency_code}</div>
                                    </div>
                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Target Amount"}</div>
                                        <div class="stat-value text-2xl">
                                            {format!("{:.2}", target)}
                                        </div>
                                        <div class="stat-desc">{&account.currency_code}</div>
                                    </div>
                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Amount to Go"}</div>
                                        <div class={classes!("stat-value", "text-2xl", if remaining <= 0.0 { "text-success" } else { "text-warning" })}>
                                            {if remaining > 0.0 {
                                                format!("{:.2}", remaining)
                                            } else {
                                                "Goal Reached!".to_string()
                                            }}
                                        </div>
                                        <div class="stat-desc">{&account.currency_code}</div>
                                    </div>
                                </div>

                                <div class="w-full">
                                    <div class="flex justify-between mb-2">
                                        <span class="text-sm font-semibold">{"Progress"}</span>
                                        <span class="text-sm font-semibold">{format!("{:.1}%", progress)}</span>
                                    </div>
                                    <div class="w-full bg-base-300 rounded-full h-6 overflow-hidden">
                                        <div
                                            class={if progress >= 100.0 { "bg-success h-6 rounded-full transition-all flex items-center justify-center" } else { "bg-primary h-6 rounded-full transition-all" }}
                                            style={format!("width: {}%", progress)}
                                        >
                                            {if progress >= 100.0 {
                                                html! { <span class="text-white text-sm font-bold">{"🎉 Goal Reached!"}</span> }
                                            } else {
                                                html! {}
                                            }}
                                        </div>
                                    </div>
                                </div>

                                {if let Some(goal_date) = &stats.goal_reached_date {
                                    html! {
                                        <div class="alert alert-info">
                                            <i class="fas fa-calendar-check"></i>
                                            <span>
                                                <strong>{"Estimated Completion: "}</strong>
                                                {goal_date.format("%B %d, %Y").to_string()}
                                            </span>
                                        </div>
                                    }
                                } else if remaining > 0.0 {
                                    html! {
                                        <div class="alert alert-warning">
                                            <i class="fas fa-exclamation-triangle"></i>
                                            <span>{"Goal not projected to be reached within the forecast period. Consider increasing your savings rate."}</span>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                        };
                    } else {
                        return html! { <div class="text-sm text-gray-400">{"Unable to parse current balance"}</div> };
                    }
                } else {
                    return html! { <div class="text-sm text-gray-400">{"Balance data not available"}</div> };
                }
            } else {
                return html! { <div class="text-sm text-error">{"Invalid target amount"}</div> };
            }
        } else {
            return html! { <div class="text-sm text-gray-400">{"No target amount set for this goal"}</div> };
        }
    } else {
        return html! { <div class="text-sm text-gray-400">{"No statistics available"}</div> };
    }
}

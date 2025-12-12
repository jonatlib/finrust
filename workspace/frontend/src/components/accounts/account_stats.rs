use yew::prelude::*;
use crate::api_client::statistics::{get_account_statistics_with_ignored, AccountStatisticsCollection};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account_id: i32,
}

#[function_component(AccountStats)]
pub fn account_stats(props: &Props) -> Html {
    let account_id = props.account_id;
    let (fetch_state, _refetch) = use_fetch_with_refetch(move || get_account_statistics_with_ignored(account_id, true));

    html! {
        <div class="card bg-base-100 shadow mt-6">
            <div class="card-body">
                <h3 class="card-title text-lg">{"Account Statistics"}</h3>

                {match &*fetch_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-error">
                            <span>{error}</span>
                        </div>
                    },
                    FetchState::Success(collection) => {
                        if let Some(stats) = collection.statistics.first() {
                            html! {
                                <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Current Balance"}</div>
                                        <div class={classes!("stat-value", "text-lg")}>
                                            {format_decimal_option(stats.end_of_period_state)}
                                        </div>
                                        <div class="stat-desc">{"End of period state"}</div>
                                    </div>

                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Average Income"}</div>
                                        <div class="stat-value text-lg text-success">
                                            {format_decimal_option(stats.average_income)}
                                        </div>
                                        <div class="stat-desc">{"Per period"}</div>
                                    </div>

                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Average Expense"}</div>
                                        <div class="stat-value text-lg text-error">
                                            {format_decimal_option(stats.average_expense)}
                                        </div>
                                        <div class="stat-desc">{"Per period"}</div>
                                    </div>

                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Minimum State"}</div>
                                        <div class="stat-value text-lg">
                                            {format_decimal_option(stats.min_state)}
                                        </div>
                                        <div class="stat-desc">{"Lowest balance"}</div>
                                    </div>

                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Maximum State"}</div>
                                        <div class="stat-value text-lg">
                                            {format_decimal_option(stats.max_state)}
                                        </div>
                                        <div class="stat-desc">{"Highest balance"}</div>
                                    </div>

                                    <div class="stat bg-base-200 rounded-lg">
                                        <div class="stat-title">{"Upcoming Expenses"}</div>
                                        <div class="stat-value text-lg text-warning">
                                            {format_decimal_option(stats.upcoming_expenses)}
                                        </div>
                                        <div class="stat-desc">{"Forecasted"}</div>
                                    </div>
                                </div>
                            }
                        } else {
                            html! {
                                <div class="text-center py-8 text-gray-500">
                                    <i class="fas fa-chart-line text-4xl mb-4 opacity-50"></i>
                                    <p>{"No statistics available."}</p>
                                    <p class="text-sm mt-2">{"Make sure account has manual states and transactions."}</p>
                                </div>
                            }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }}
            </div>
        </div>
    }
}

fn format_decimal_option(value: Option<rust_decimal::Decimal>) -> String {
    match value {
        Some(d) => format!("{:.2}", d),
        None => "N/A".to_string(),
    }
}

use yew::prelude::*;
use crate::api_client::category::{get_category_stats, CategoryStatistics};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use chrono::{Local, Datelike};

#[function_component(CategoryStats)]
pub fn category_stats() -> Html {
    // Get current year date range
    let current_year = Local::now().year();
    let start_date = format!("{}-01-01", current_year);
    let end_date = format!("{}-12-31", current_year);

    let (fetch_state, _refetch) = use_fetch_with_refetch({
        let start_date = start_date.clone();
        let end_date = end_date.clone();
        move || {
            let start_date = start_date.clone();
            let end_date = end_date.clone();
            async move {
                get_category_stats(&start_date, &end_date).await
            }
        }
    });

    html! {
        <div class="card bg-base-100 shadow mt-6">
            <div class="card-body">
                <h2 class="card-title">
                    {"Spending by Category "}
                    <span class="text-sm font-normal text-gray-500">{format!("(Year {})", current_year)}</span>
                </h2>
                {
                    match &*fetch_state {
                        FetchState::Loading => html! {
                            <div class="flex justify-center items-center py-8">
                                <span class="loading loading-spinner loading-md"></span>
                            </div>
                        },
                        FetchState::Error(error) => html! {
                            <div class="alert alert-warning">
                                <span>{format!("Unable to load category statistics: {}", error)}</span>
                            </div>
                        },
                        FetchState::Success(stats) => {
                            if stats.is_empty() {
                                html! {
                                    <div class="text-center py-4 text-gray-500">
                                        {"No category data available for this period"}
                                    </div>
                                }
                            } else {
                                // Sort by total amount (absolute value for display)
                                let mut sorted_stats = stats.clone();
                                sorted_stats.sort_by(|a, b| {
                                    let a_val = a.total_amount.parse::<f64>().unwrap_or(0.0).abs();
                                    let b_val = b.total_amount.parse::<f64>().unwrap_or(0.0).abs();
                                    b_val.partial_cmp(&a_val).unwrap_or(std::cmp::Ordering::Equal)
                                });

                                // Take top 10 categories
                                let top_categories: Vec<&CategoryStatistics> = sorted_stats.iter().take(10).collect();

                                html! {
                                    <div class="overflow-x-auto">
                                        <table class="table table-zebra">
                                            <thead>
                                                <tr>
                                                    <th>{"Category"}</th>
                                                    <th class="text-right">{"Transactions"}</th>
                                                    <th class="text-right">{"Total Amount"}</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                { for top_categories.iter().map(|stat| {
                                                    let amount = stat.total_amount.parse::<f64>().unwrap_or(0.0);
                                                    let amount_class = if amount < 0.0 { "text-error" } else { "text-success" };
                                                    html! {
                                                        <tr key={stat.category_id}>
                                                            <td>
                                                                <div class="flex items-center gap-2">
                                                                    <i class="fas fa-tag text-gray-400"></i>
                                                                    <span class="font-medium">{&stat.category_name}</span>
                                                                </div>
                                                            </td>
                                                            <td class="text-right">{stat.transaction_count}</td>
                                                            <td class={classes!("text-right", "font-mono", amount_class)}>
                                                                {format!("{:.2}", amount)}
                                                            </td>
                                                        </tr>
                                                    }
                                                })}
                                            </tbody>
                                        </table>
                                        if sorted_stats.len() > 10 {
                                            <div class="text-sm text-center text-gray-500 mt-2">
                                                {format!("Showing top 10 of {} categories", sorted_stats.len())}
                                            </div>
                                        }
                                    </div>
                                }
                            }
                        },
                        FetchState::NotStarted => html! { <></> },
                    }
                }
            </div>
        </div>
    }
}

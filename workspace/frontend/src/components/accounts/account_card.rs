use yew::prelude::*;
use yew_router::prelude::*;
use crate::api_client::account::AccountResponse;
use crate::api_client::statistics::get_account_statistics_with_ignored;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use crate::Route;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account: AccountResponse,
}

#[function_component(AccountCard)]
pub fn account_card(props: &Props) -> Html {
    let account = &props.account;
    let account_id = account.id;
    let navigator = use_navigator().unwrap();

    // Fetch statistics for this account
    let (stats_state, _refetch) = use_fetch_with_refetch(move || get_account_statistics_with_ignored(account_id, true));

    log::debug!("Rendering account card for: {} (ID: {}), stats state={:?}",
        account.name, account.id,
        match &*stats_state {
            FetchState::Loading => "Loading",
            FetchState::Success(c) => {
                log::info!("Stats loaded for account {}: {} stats in collection", account_id, c.statistics.len());
                "Success"
            },
            FetchState::Error(e) => {
                log::error!("Stats error for account {}: {}", account_id, e);
                "Error"
            },
            FetchState::NotStarted => "NotStarted",
        });

    // Extract stats if available
    let stats = match &*stats_state {
        FetchState::Success(collection) => {
            log::debug!("Account {} stats: {:?}", account_id, collection.statistics.first());
            collection.statistics.first()
        },
        _ => None,
    };

    let on_card_click = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            log::info!("Navigating to account detail page for ID: {}", account_id);
            navigator.push(&Route::AccountEdit { id: account_id });
        })
    };

    html! {
        <div
            class="card bg-base-100 shadow hover:shadow-lg transition-shadow cursor-pointer"
            onclick={on_card_click}
        >
            <div class="card-body">
                <div class="flex justify-between items-start">
                    <div>
                        <h3 class="card-title text-base">{&account.name}</h3>
                        {if let Some(desc) = &account.description {
                            html! { <p class="text-xs text-gray-500 mt-1">{desc}</p> }
                        } else {
                            html! {}
                        }}
                    </div>
                    {if account.include_in_statistics {
                        html! { <div class="badge badge-primary badge-outline badge-sm" title="Included in Statistics"><i class="fas fa-chart-line"></i></div> }
                    } else {
                        html! { <div class="badge badge-ghost badge-sm" title="Excluded from Statistics"><i class="fas fa-eye-slash"></i></div> }
                    }}
                </div>
                <div class="mt-4">
                    <div class="text-xs text-gray-500 mb-2">{"Currency"}</div>
                    <div class="badge badge-secondary badge-outline badge-sm">{&account.currency_code}</div>
                </div>

                // Display stats section only if account is included in statistics
                {if account.include_in_statistics {
                    html! {
                        <div class="mt-4 space-y-2 bg-base-200 p-3 rounded-lg">
                            <div class="text-xs font-semibold text-gray-600 uppercase">{"Account Stats"}</div>

                            {match &*stats_state {
                                FetchState::Loading => html! {
                                    <div class="flex justify-center items-center py-4">
                                        <span class="loading loading-spinner loading-sm"></span>
                                    </div>
                                },
                                FetchState::Error(error) => html! {
                                    <div class="text-xs text-error">
                                        {format!("Failed to load stats: {}", error)}
                                    </div>
                                },
                        FetchState::Success(_) => {
                            if let Some(s) = stats {
                                html! {
                                    <div class="grid grid-cols-1 gap-2">
                                        <div class="flex justify-between items-center">
                                            <span class="text-xs text-gray-500">{"Current Balance:"}</span>
                                            {if let Some(balance) = &s.end_of_period_state {
                                                html! { <span class="text-sm font-bold">{balance}{" "}{&account.currency_code}</span> }
                                            } else {
                                                html! { <span class="text-xs text-gray-400">{"N/A"}</span> }
                                            }}
                                        </div>

                                        <div class="flex justify-between items-center">
                                            <span class="text-xs text-gray-500">{"Min State:"}</span>
                                            {if let Some(min) = &s.min_state {
                                                html! { <span class="text-sm font-bold">{min}{" "}{&account.currency_code}</span> }
                                            } else {
                                                html! { <span class="text-xs text-gray-400">{"N/A"}</span> }
                                            }}
                                        </div>

                                        <div class="flex justify-between items-center">
                                            <span class="text-xs text-gray-500">{"Max State:"}</span>
                                            {if let Some(max) = &s.max_state {
                                                html! { <span class="text-sm font-bold">{max}{" "}{&account.currency_code}</span> }
                                            } else {
                                                html! { <span class="text-xs text-gray-400">{"N/A"}</span> }
                                            }}
                                        </div>

                                        <div class="flex justify-between items-center">
                                            <span class="text-xs text-gray-500">{"Avg Income:"}</span>
                                            {if let Some(income) = &s.average_income {
                                                html! { <span class="text-sm font-bold text-success">{income}{" "}{&account.currency_code}</span> }
                                            } else {
                                                html! { <span class="text-xs text-gray-400">{"N/A"}</span> }
                                            }}
                                        </div>

                                        <div class="flex justify-between items-center">
                                            <span class="text-xs text-gray-500">{"Avg Expense:"}</span>
                                            {if let Some(expense) = &s.average_expense {
                                                html! { <span class="text-sm font-bold text-error">{expense}{" "}{&account.currency_code}</span> }
                                            } else {
                                                html! { <span class="text-xs text-gray-400">{"N/A"}</span> }
                                            }}
                                        </div>
                                    </div>
                                }
                            } else {
                                html! {
                                    <div class="text-xs text-gray-400">{"No stats available"}</div>
                                }
                            }
                        },
                                FetchState::NotStarted => html! {
                                    <div class="text-xs text-gray-400">{"Loading..."}</div>
                                },
                            }}
                        </div>
                    }
                } else {
                    html! {}
                }}
                {if let Some(ledger) = &account.ledger_name {
                    html! {
                        <div class="mt-2">
                            <div class="text-xs text-gray-500">{"Ledger"}</div>
                            <div class="text-sm mt-1">{ledger}</div>
                        </div>
                    }
                } else {
                    html! {}
                }}
            </div>
        </div>
    }
}

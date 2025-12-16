use super::transaction_modal::TransactionModal;
use crate::api_client::account::get_accounts;
use crate::api_client::category::get_categories;
use crate::api_client::scenario::get_scenarios;
use crate::api_client::transaction::{delete_transaction, get_transaction};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use crate::Route;
use std::collections::HashMap;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub transaction_id: i32,
}

#[function_component(TransactionEdit)]
pub fn transaction_edit(props: &Props) -> Html {
    let transaction_id = props.transaction_id;
    let navigator = use_navigator().unwrap();

    let (fetch_state, refetch) = use_fetch_with_refetch(move || get_transaction(transaction_id));
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let (categories_state, _) = use_fetch_with_refetch(get_categories);
    let (scenarios_state, _) = use_fetch_with_refetch(get_scenarios);
    let show_edit_modal = use_state(|| false);
    let show_delete_confirm = use_state(|| false);
    let is_deleting = use_state(|| false);
    let delete_error = use_state(|| None::<String>);

    // Build account ID -> name map
    let account_map: HashMap<i32, String> = match &*accounts_state {
        FetchState::Success(accounts) => accounts
            .iter()
            .map(|acc| (acc.id, acc.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Build category ID -> name map
    let category_map: HashMap<i32, String> = match &*categories_state {
        FetchState::Success(categories) => categories
            .iter()
            .map(|cat| (cat.id, cat.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Build scenario ID -> name map
    let scenario_map: HashMap<i32, String> = match &*scenarios_state {
        FetchState::Success(scenarios) => scenarios
            .iter()
            .map(|s| (s.id, s.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Get accounts list for the modal
    let accounts_list = match &*accounts_state {
        FetchState::Success(accounts) => accounts.clone(),
        _ => vec![],
    };

    // Get scenarios list for the modal
    let scenarios_list = match &*scenarios_state {
        FetchState::Success(scenarios) => scenarios.clone(),
        _ => vec![],
    };

    let on_open_edit = {
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| {
            log::info!("Opening edit modal for transaction ID: {}", transaction_id);
            show_edit_modal.set(true);
        })
    };

    let on_close_edit = {
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| {
            log::info!("Closing edit modal");
            show_edit_modal.set(false);
        })
    };

    let on_edit_success = {
        let refetch = refetch.clone();
        Callback::from(move |_| {
            log::info!("Transaction updated successfully, refetching");
            refetch.emit(());
        })
    };

    let on_delete_click = {
        let show_delete_confirm = show_delete_confirm.clone();
        Callback::from(move |_| {
            log::info!("Delete button clicked, showing confirmation");
            show_delete_confirm.set(true);
        })
    };

    let on_cancel_delete = {
        let show_delete_confirm = show_delete_confirm.clone();
        Callback::from(move |_| {
            log::info!("Delete cancelled");
            show_delete_confirm.set(false);
        })
    };

    let on_confirm_delete = {
        let is_deleting = is_deleting.clone();
        let delete_error = delete_error.clone();
        let navigator = navigator.clone();
        let show_delete_confirm = show_delete_confirm.clone();
        Callback::from(move |_| {
            if *is_deleting {
                return;
            }

            let is_deleting = is_deleting.clone();
            let delete_error = delete_error.clone();
            let navigator = navigator.clone();
            let show_delete_confirm = show_delete_confirm.clone();

            is_deleting.set(true);
            delete_error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                log::info!("Deleting transaction ID: {}", transaction_id);
                match delete_transaction(transaction_id).await {
                    Ok(_) => {
                        log::info!("Transaction deleted successfully, navigating to transactions list");
                        is_deleting.set(false);
                        show_delete_confirm.set(false);
                        navigator.push(&Route::Transactions);
                    }
                    Err(e) => {
                        log::error!("Failed to delete transaction: {}", e);
                        delete_error.set(Some(format!("Failed to delete transaction: {}", e)));
                        is_deleting.set(false);
                    }
                }
            });
        })
    };

    html! {
        <>
            {match &*fetch_state {
                FetchState::Loading => html! {
                    <div class="flex justify-center items-center py-8">
                        <span class="loading loading-spinner loading-lg"></span>
                    </div>
                },
                FetchState::Error(error) => html! {
                    <div class="alert alert-error">
                        <span>{error}</span>
                        <button class="btn btn-sm" onclick={move |_| refetch.emit(())}>
                            {"Retry"}
                        </button>
                    </div>
                },
                FetchState::Success(transaction) => {
                    let target_account_name = account_map
                        .get(&transaction.target_account_id)
                        .map(|name| name.as_str())
                        .unwrap_or("Unknown Account");
                    
                    let source_account_name = transaction.source_account_id
                        .and_then(|id| account_map.get(&id))
                        .map(|name| name.as_str());

                    let amount_class = if transaction.amount >= rust_decimal::Decimal::ZERO {
                        "text-success"
                    } else {
                        "text-error"
                    };

                    html! {
                        <>
                            <TransactionModal
                                show={*show_edit_modal}
                                on_close={on_close_edit}
                                on_success={on_edit_success}
                                accounts={accounts_list}
                                scenarios={scenarios_list}
                                transaction={Some(transaction.clone())}
                            />

                            // Delete Confirmation Modal
                            <dialog class={classes!("modal", (*show_delete_confirm).then_some("modal-open"))} id="delete_confirm_modal">
                                <div class="modal-box">
                                    <h3 class="font-bold text-lg text-error">{"Delete Transaction"}</h3>
                                    <p class="py-4">
                                        {"Are you sure you want to delete the transaction "}
                                        <strong>{&transaction.name}</strong>
                                        {"? This action cannot be undone."}
                                    </p>

                                    {if let Some(error) = (*delete_error).as_ref() {
                                        html! {
                                            <div class="alert alert-error mb-4">
                                                <span>{error}</span>
                                            </div>
                                        }
                                    } else {
                                        html! {}
                                    }}

                                    <div class="modal-action">
                                        <button
                                            type="button"
                                            class="btn"
                                            onclick={on_cancel_delete.clone()}
                                            disabled={*is_deleting}
                                        >
                                            {"Cancel"}
                                        </button>
                                        <button
                                            type="button"
                                            class="btn btn-error"
                                            onclick={on_confirm_delete}
                                            disabled={*is_deleting}
                                        >
                                            {if *is_deleting {
                                                html! { <><span class="loading loading-spinner loading-sm"></span>{" Deleting..."}</> }
                                            } else {
                                                html! { "Delete Transaction" }
                                            }}
                                        </button>
                                    </div>
                                </div>
                                <form class="modal-backdrop" method="dialog">
                                    <button onclick={on_cancel_delete} disabled={*is_deleting}>{"close"}</button>
                                </form>
                            </dialog>

                            <div class="space-y-6">
                                <div class="flex justify-between items-center">
                                    <h2 class="text-2xl font-bold">{&transaction.name}</h2>
                                    <div class="flex gap-2">
                                        <button
                                            class="btn btn-primary btn-sm"
                                            onclick={on_open_edit}
                                        >
                                            <i class="fas fa-edit"></i> {" Edit"}
                                        </button>
                                        <button
                                            class="btn btn-error btn-sm"
                                            onclick={on_delete_click}
                                        >
                                            <i class="fas fa-trash"></i> {" Delete"}
                                        </button>
                                    </div>
                                </div>

                                <div class="card bg-base-100 shadow">
                                    <div class="card-body">
                                        <h3 class="card-title text-lg">{"Transaction Details"}</h3>
                                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
                                            <div>
                                                <div class="text-sm text-gray-500">{"Name"}</div>
                                                <div class="text-base font-semibold">{&transaction.name}</div>
                                            </div>
                                            {if let Some(description) = &transaction.description {
                                                html! {
                                                    <div>
                                                        <div class="text-sm text-gray-500">{"Description"}</div>
                                                        <div class="text-base">{description}</div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                            <div>
                                                <div class="text-sm text-gray-500">{"Amount"}</div>
                                                <div class={classes!("text-xl", "font-bold", amount_class)}>
                                                    {format!("{:.2}", transaction.amount)}
                                                </div>
                                            </div>
                                            <div>
                                                <div class="text-sm text-gray-500">{"Date"}</div>
                                                <div class="text-base">{transaction.date.format("%Y-%m-%d").to_string()}</div>
                                            </div>
                                            <div>
                                                <div class="text-sm text-gray-500">{"Target Account"}</div>
                                                <div class="badge badge-primary badge-outline">{target_account_name}</div>
                                            </div>
                                            {if let Some(source_name) = source_account_name {
                                                html! {
                                                    <div>
                                                        <div class="text-sm text-gray-500">{"Source Account"}</div>
                                                        <div class="badge badge-secondary badge-outline">{source_name}</div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                            <div>
                                                <div class="text-sm text-gray-500">{"Statistics"}</div>
                                                {if transaction.include_in_statistics {
                                                    html! { <div class="badge badge-success badge-outline"><i class="fas fa-check"></i>{" Included"}</div> }
                                                } else {
                                                    html! { <div class="badge badge-ghost"><i class="fas fa-times"></i>{" Excluded"}</div> }
                                                }}
                                            </div>
                                            {if let Some(category_id) = transaction.category_id {
                                                html! {
                                                    <div>
                                                        <div class="text-sm text-gray-500">{"Category"}</div>
                                                        <div class="badge badge-info badge-outline">
                                                            <i class="fas fa-tag mr-1"></i>
                                                            {category_map.get(&category_id).map(|name| name.as_str()).unwrap_or("Unknown Category")}
                                                        </div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                            <div>
                                                <div class="text-sm text-gray-500">{"Simulated"}</div>
                                                {if transaction.is_simulated {
                                                    html! { <div class="badge badge-info badge-outline"><i class="fas fa-flask mr-1"></i>{" Yes"}</div> }
                                                } else {
                                                    html! { <div class="badge badge-ghost"><i class="fas fa-times mr-1"></i>{" No"}</div> }
                                                }}
                                            </div>
                                            {if let Some(scenario_id) = transaction.scenario_id {
                                                html! {
                                                    <div>
                                                        <div class="text-sm text-gray-500">{"Scenario"}</div>
                                                        <div class="badge badge-warning badge-outline">
                                                            <i class="fas fa-project-diagram mr-1"></i>
                                                            {scenario_map.get(&scenario_id).map(|name| name.as_str()).unwrap_or("Unknown Scenario")}
                                                        </div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                            {if let Some(ledger) = &transaction.ledger_name {
                                                html! {
                                                    <div>
                                                        <div class="text-sm text-gray-500">{"Ledger Name"}</div>
                                                        <div class="text-base font-mono">{ledger}</div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                            {if !transaction.tags.is_empty() {
                                                html! {
                                                    <div class="col-span-2">
                                                        <div class="text-sm text-gray-500 mb-2">{"Tags"}</div>
                                                        <div class="flex gap-2 flex-wrap">
                                                            { for transaction.tags.iter().map(|tag| {
                                                                html! {
                                                                    <span class="badge badge-lg badge-outline">{&tag.name}</span>
                                                                }
                                                            })}
                                                        </div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </>
                    }
                },
                FetchState::NotStarted => html! { <></> },
            }}
        </>
    }
}

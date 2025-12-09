use yew::prelude::*;
use std::collections::HashMap;
use crate::api_client::manual_account_state::{get_all_manual_states, delete_manual_state, ManualAccountStateResponse};
use crate::api_client::account::get_accounts;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use super::modal::ManualStateModal;

#[function_component(ManualStates)]
pub fn manual_states() -> Html {
    log::trace!("ManualStates component rendering");
    let (fetch_state, refetch) = use_fetch_with_refetch(get_all_manual_states);
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let show_modal = use_state(|| false);
    let edit_state = use_state(|| None::<ManualAccountStateResponse>);
    let show_delete_confirm = use_state(|| false);
    let delete_target = use_state(|| None::<i32>);
    let is_deleting = use_state(|| false);
    let delete_error = use_state(|| None::<String>);

    log::debug!("ManualStates component state: loading={}, success={}, error={}",
        fetch_state.is_loading(), fetch_state.is_success(), fetch_state.is_error());

    // Build account ID -> name map
    let account_map: HashMap<i32, String> = match &*accounts_state {
        FetchState::Success(accounts) => accounts
            .iter()
            .map(|acc| (acc.id, acc.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Get accounts list for the modal
    let accounts_list = match &*accounts_state {
        FetchState::Success(accounts) => accounts.clone(),
        _ => vec![],
    };

    let on_open_add_modal = {
        let show_modal = show_modal.clone();
        let edit_state = edit_state.clone();
        Callback::from(move |_| {
            log::info!("Opening Add Manual State modal");
            edit_state.set(None);
            show_modal.set(true);
        })
    };

    let on_open_edit_modal = {
        let show_modal = show_modal.clone();
        let edit_state = edit_state.clone();
        Callback::from(move |state: ManualAccountStateResponse| {
            log::info!("Opening Edit Manual State modal for ID: {}", state.id);
            edit_state.set(Some(state));
            show_modal.set(true);
        })
    };

    let on_close_modal = {
        let show_modal = show_modal.clone();
        let edit_state = edit_state.clone();
        Callback::from(move |_| {
            log::info!("Closing Manual State modal");
            show_modal.set(false);
            edit_state.set(None);
        })
    };

    let on_success = {
        let refetch = refetch.clone();
        Callback::from(move |_| {
            log::info!("Manual state saved successfully, refetching");
            refetch.emit(());
        })
    };

    let on_delete_click = {
        let show_delete_confirm = show_delete_confirm.clone();
        let delete_target = delete_target.clone();
        Callback::from(move |state_id: i32| {
            log::info!("Delete button clicked for state ID: {}", state_id);
            delete_target.set(Some(state_id));
            show_delete_confirm.set(true);
        })
    };

    let on_cancel_delete = {
        let show_delete_confirm = show_delete_confirm.clone();
        let delete_target = delete_target.clone();
        Callback::from(move |_| {
            log::info!("Delete cancelled");
            show_delete_confirm.set(false);
            delete_target.set(None);
        })
    };

    let on_confirm_delete = {
        let is_deleting = is_deleting.clone();
        let delete_error = delete_error.clone();
        let show_delete_confirm = show_delete_confirm.clone();
        let delete_target = delete_target.clone();
        let refetch = refetch.clone();
        Callback::from(move |_| {
            if *is_deleting {
                return;
            }

            if let Some(state_id) = *delete_target {
                let is_deleting = is_deleting.clone();
                let delete_error = delete_error.clone();
                let show_delete_confirm = show_delete_confirm.clone();
                let delete_target = delete_target.clone();
                let refetch = refetch.clone();

                is_deleting.set(true);
                delete_error.set(None);

                wasm_bindgen_futures::spawn_local(async move {
                    log::info!("Deleting manual account state ID: {}", state_id);
                    match delete_manual_state(state_id).await {
                        Ok(_) => {
                            log::info!("Manual account state deleted successfully");
                            is_deleting.set(false);
                            show_delete_confirm.set(false);
                            delete_target.set(None);
                            refetch.emit(());
                        }
                        Err(e) => {
                            log::error!("Failed to delete manual account state: {}", e);
                            delete_error.set(Some(format!("Failed to delete: {}", e)));
                            is_deleting.set(false);
                        }
                    }
                });
            }
        })
    };

    html! {
        <>
            <ManualStateModal
                show={*show_modal}
                on_close={on_close_modal}
                on_success={on_success}
                accounts={accounts_list}
                state={(*edit_state).clone()}
            />

            // Delete Confirmation Modal
            <dialog class={classes!("modal", (*show_delete_confirm).then_some("modal-open"))} id="delete_confirm_modal">
                <div class="modal-box">
                    <h3 class="font-bold text-lg text-error">{"Delete Account Balance"}</h3>
                    <p class="py-4">
                        {"Are you sure you want to delete this account balance entry? This action cannot be undone."}
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
                                html! { "Delete" }
                            }}
                        </button>
                    </div>
                </div>
                <form class="modal-backdrop" method="dialog">
                    <button onclick={on_cancel_delete} disabled={*is_deleting}>{"close"}</button>
                </form>
            </dialog>

            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Account Balances"}</h2>
                <button
                    class="btn btn-primary btn-sm"
                    onclick={on_open_add_modal}
                >
                    <i class="fas fa-plus"></i> {" Add Balance"}
                </button>
            </div>

            {
                match &*fetch_state {
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
                    FetchState::Success(states) => {
                        if states.is_empty() {
                            html! {
                                <div class="text-center py-8">
                                    <p class="text-gray-500 mb-4">{"No account balances found."}</p>
                                    <p class="text-sm text-gray-400">{"Add an initial balance for your accounts to enable statistics tracking."}</p>
                                </div>
                            }
                        } else {
                            // Sort by date descending (most recent first)
                            let mut sorted_states = states.clone();
                            sorted_states.sort_by(|a, b| b.date.cmp(&a.date));

                            html! {
                                <div class="overflow-x-auto bg-base-100 shadow rounded-box">
                                    <table class="table table-zebra">
                                        <thead>
                                            <tr>
                                                <th>{"Date"}</th>
                                                <th>{"Account"}</th>
                                                <th>{"Balance"}</th>
                                                <th>{"Actions"}</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            { for sorted_states.iter().map(|state| {
                                                let account_name = account_map
                                                    .get(&state.account_id)
                                                    .map(|name| name.as_str())
                                                    .unwrap_or("Unknown Account");

                                                let state_clone_edit = state.clone();
                                                let state_id_delete = state.id;

                                                html! {
                                                    <tr key={state.id}>
                                                        <td class="font-semibold">{state.date.format("%Y-%m-%d").to_string()}</td>
                                                        <td>
                                                            <span class="badge badge-sm badge-ghost">{account_name}</span>
                                                        </td>
                                                        <td class="font-mono font-semibold text-lg">{format!("{:.2}", state.amount)}</td>
                                                        <td>
                                                            <div class="flex gap-2">
                                                                <button
                                                                    class="btn btn-sm btn-ghost btn-square"
                                                                    title="Edit"
                                                                    onclick={on_open_edit_modal.reform(move |_| state_clone_edit.clone())}
                                                                >
                                                                    <i class="fas fa-edit"></i>
                                                                </button>
                                                                <button
                                                                    class="btn btn-sm btn-error btn-outline btn-square"
                                                                    title="Delete"
                                                                    onclick={on_delete_click.reform(move |_| state_id_delete)}
                                                                >
                                                                    <i class="fas fa-trash"></i>
                                                                </button>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                }
                                            })}
                                        </tbody>
                                    </table>
                                </div>
                            }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }
            }
        </>
    }
}

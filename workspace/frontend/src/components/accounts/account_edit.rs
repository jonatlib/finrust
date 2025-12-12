use yew::prelude::*;
use yew_router::prelude::*;
use crate::api_client::account::{get_account_with_ignored, delete_account};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use super::account_modal::AccountModal;
use super::{AccountStats, AccountChart, AccountForecast};
use crate::components::manual_states::ManualStatesAccountView;
use crate::Route;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account_id: i32,
}

#[function_component(AccountEdit)]
pub fn account_edit(props: &Props) -> Html {
    let account_id = props.account_id;
    let navigator = use_navigator().unwrap();

    let (fetch_state, refetch) = use_fetch_with_refetch(move || get_account_with_ignored(account_id, true));
    let show_edit_modal = use_state(|| false);
    let show_delete_confirm = use_state(|| false);
    let is_deleting = use_state(|| false);
    let delete_error = use_state(|| None::<String>);

    let on_open_edit = {
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| {
            log::info!("Opening edit modal for account ID: {}", account_id);
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
            log::info!("Account updated successfully, refetching");
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
                log::info!("Deleting account ID: {}", account_id);
                match delete_account(account_id).await {
                    Ok(_) => {
                        log::info!("Account deleted successfully, navigating to accounts list");
                        is_deleting.set(false);
                        show_delete_confirm.set(false);
                        navigator.push(&Route::Accounts);
                    }
                    Err(e) => {
                        log::error!("Failed to delete account: {}", e);
                        delete_error.set(Some(format!("Failed to delete account: {}", e)));
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
                FetchState::Success(account) => {
                    html! {
                        <>
                            <AccountModal
                                show={*show_edit_modal}
                                on_close={on_close_edit}
                                on_success={on_edit_success}
                                account={Some(account.clone())}
                            />

                            // Delete Confirmation Modal
                            <dialog class={classes!("modal", (*show_delete_confirm).then_some("modal-open"))} id="delete_confirm_modal">
                                <div class="modal-box">
                                    <h3 class="font-bold text-lg text-error">{"Delete Account"}</h3>
                                    <p class="py-4">
                                        {"Are you sure you want to delete the account "}
                                        <strong>{&account.name}</strong>
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
                                                html! { "Delete Account" }
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
                                    <h2 class="text-2xl font-bold">{&account.name}</h2>
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
                                        <h3 class="card-title text-lg">{"Account Details"}</h3>
                                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
                                            <div>
                                                <div class="text-sm text-gray-500">{"Name"}</div>
                                                <div class="text-base font-semibold">{&account.name}</div>
                                            </div>
                                            {if let Some(description) = &account.description {
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
                                                <div class="text-sm text-gray-500">{"Currency"}</div>
                                                <div class="badge badge-secondary badge-outline">{&account.currency_code}</div>
                                            </div>
                                            <div>
                                                <div class="text-sm text-gray-500">{"Account Type"}</div>
                                                <div class="badge badge-primary badge-outline">{account.account_kind.display_name()}</div>
                                            </div>
                                            <div>
                                                <div class="text-sm text-gray-500">{"Statistics"}</div>
                                                {if account.include_in_statistics {
                                                    html! { <div class="badge badge-success badge-outline"><i class="fas fa-check"></i>{" Included"}</div> }
                                                } else {
                                                    html! { <div class="badge badge-ghost"><i class="fas fa-times"></i>{" Excluded"}</div> }
                                                }}
                                            </div>
                                            {if let Some(ledger) = &account.ledger_name {
                                                html! {
                                                    <div>
                                                        <div class="text-sm text-gray-500">{"Ledger Name"}</div>
                                                        <div class="text-base font-mono">{ledger}</div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                        </div>
                                    </div>
                                </div>

                                <AccountStats account_id={account_id} />

                                <AccountChart account_id={account_id} />

                                <AccountForecast account_id={account_id} />

                                <ManualStatesAccountView account_id={account_id} />
                            </div>
                        </>
                    }
                },
                FetchState::NotStarted => html! { <></> },
            }}
        </>
    }
}

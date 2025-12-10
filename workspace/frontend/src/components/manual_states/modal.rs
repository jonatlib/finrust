use yew::prelude::*;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use crate::api_client::manual_account_state::{ManualAccountStateResponse, CreateManualAccountStateRequest, UpdateManualAccountStateRequest, create_manual_state, update_manual_state};
use crate::api_client::account::AccountResponse;

#[derive(Properties, PartialEq)]
pub struct ManualStateModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    pub accounts: Vec<AccountResponse>,
    /// If provided, the modal is in edit mode with this state
    pub state: Option<ManualAccountStateResponse>,
}

#[function_component(ManualStateModal)]
pub fn manual_state_modal(props: &ManualStateModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    let is_edit_mode = props.state.is_some();
    let title = if is_edit_mode { "Edit Account Balance" } else { "Add Account Balance" };

    let on_submit = {
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let form_ref = form_ref.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let state = props.state.clone();
        let is_edit = state.is_some();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *is_submitting {
                return;
            }

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let account_id_str = form_data.get("account_id").as_string().unwrap_or_default();
                let date_str = form_data.get("date").as_string().unwrap_or_default();
                let amount_str = form_data.get("amount").as_string().unwrap_or_default();

                // Parse account ID
                let account_id = if is_edit {
                    state.as_ref().unwrap().account_id
                } else {
                    match account_id_str.parse::<i32>() {
                        Ok(id) => id,
                        Err(_) => {
                            error_message.set(Some("Please select an account".to_string()));
                            return;
                        }
                    }
                };

                // Parse date
                let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => {
                        error_message.set(Some("Invalid date format".to_string()));
                        return;
                    }
                };

                // Parse amount
                let amount = match Decimal::from_str(&amount_str) {
                    Ok(amt) => amt,
                    Err(_) => {
                        error_message.set(Some("Invalid amount format".to_string()));
                        return;
                    }
                };

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

                if is_edit {
                    // Edit mode - update manual state
                    let existing_state = state.clone().unwrap();
                    let state_id = existing_state.id;
                    let existing_account_id = existing_state.account_id;
                    let request = UpdateManualAccountStateRequest {
                        date: Some(date),
                        amount: Some(amount),
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Updating manual account state ID {}", state_id);
                        match update_manual_state(existing_account_id, state_id, request).await {
                            Ok(_) => {
                                log::info!("Manual account state updated successfully");
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to update manual account state: {}", e);
                                error_message.set(Some(format!("Failed to update: {}", e)));
                                is_submitting.set(false);
                            }
                        }
                    });
                } else {
                    // Create mode - create new manual state
                    let request = CreateManualAccountStateRequest { date, amount };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Creating manual account state for account ID: {}", account_id);
                        match create_manual_state(account_id, request).await {
                            Ok(_) => {
                                log::info!("Manual account state created successfully");
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to create manual account state: {}", e);
                                error_message.set(Some(format!("Failed to create: {}", e)));
                                is_submitting.set(false);
                            }
                        }
                    });
                }
            }
        })
    };

    let on_close = {
        let on_close = props.on_close.clone();
        let is_submitting = *is_submitting;
        Callback::from(move |_| {
            if !is_submitting {
                on_close.emit(())
            }
        })
    };

    // Get default values from state if in edit mode
    let default_account_id = props.state.as_ref().map(|s| s.account_id).unwrap_or(0);
    let default_date = props.state.as_ref().map(|s| s.date.format("%Y-%m-%d").to_string()).unwrap_or_else(|| {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    });
    let default_amount = props.state.as_ref().map(|s| s.amount.to_string()).unwrap_or_default();

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="manual_state_modal">
            <div class="modal-box w-11/12 max-w-2xl">
                <h3 class="font-bold text-lg">{title}</h3>

                {if let Some(error) = (*error_message).as_ref() {
                    html! {
                        <div class="alert alert-error mt-4">
                            <span>{error}</span>
                        </div>
                    }
                } else {
                    html! {}
                }}

                <form ref={form_ref} onsubmit={on_submit} class="py-4 space-y-4">
                    {if !is_edit_mode {
                        html! {
                            <div class="form-control">
                                <label class="label"><span class="label-text">{"Account"}</span></label>
                                <select name="account_id" class="select select-bordered w-full" required={true} disabled={*is_submitting}>
                                    <option value="" disabled={true} selected={default_account_id == 0}>{"Select account"}</option>
                                    { for props.accounts.iter().map(|account| {
                                        html! {
                                            <option
                                                value={account.id.to_string()}
                                                selected={default_account_id == account.id}
                                            >
                                                {&account.name}
                                            </option>
                                        }
                                    })}
                                </select>
                            </div>
                        }
                    } else {
                        html! {}
                    }}

                    <div class="form-control">
                        <label class="label">
                            <span class="label-text">{"Date"}</span>
                            <span class="label-text-alt text-gray-500">{"Balance valid on this date"}</span>
                        </label>
                        <input
                            type="date"
                            name="date"
                            class="input input-bordered w-full"
                            value={default_date}
                            required={true}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label">
                            <span class="label-text">{"Amount"}</span>
                            <span class="label-text-alt text-gray-500">{"Account balance"}</span>
                        </label>
                        <input
                            type="number"
                            name="amount"
                            class="input input-bordered w-full"
                            placeholder="0.00"
                            step="0.01"
                            value={default_amount}
                            required={true}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="alert alert-info">
                        <i class="fas fa-info-circle"></i>
                        <span>{"This sets the account balance at a specific date, allowing the system to calculate statistics and track changes."}</span>
                    </div>

                    <div class="modal-action">
                        <button
                            type="button"
                            class="btn"
                            onclick={on_close.clone()}
                            disabled={*is_submitting}
                        >
                            {"Cancel"}
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled={*is_submitting}
                        >
                            {if *is_submitting {
                                if is_edit_mode {
                                    html! { <><span class="loading loading-spinner loading-sm"></span>{" Updating..."}</> }
                                } else {
                                    html! { <><span class="loading loading-spinner loading-sm"></span>{" Creating..."}</> }
                                }
                            } else {
                                if is_edit_mode {
                                    html! { "Update Balance" }
                                } else {
                                    html! { "Add Balance" }
                                }
                            }}
                        </button>
                    </div>
                </form>
            </div>
            <form class="modal-backdrop" method="dialog">
                <button onclick={on_close} disabled={*is_submitting}>{"close"}</button>
            </form>
        </dialog>
    }
}

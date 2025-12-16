use crate::api_client::account::AccountResponse;
use crate::api_client::category::{get_categories, CategoryResponse};
use crate::api_client::transaction::{create_transaction, update_transaction, CreateTransactionRequest, TransactionResponse, UpdateTransactionRequest};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TransactionModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    pub accounts: Vec<AccountResponse>,
    /// If provided, the modal is in edit mode with this transaction
    #[prop_or_default]
    pub transaction: Option<TransactionResponse>,
    /// If provided, transactions will be linked to this scenario and marked as simulated
    #[prop_or_default]
    pub scenario_id: Option<i32>,
}

#[function_component(TransactionModal)]
pub fn transaction_modal(props: &TransactionModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);
    let (categories_state, _) = use_fetch_with_refetch(get_categories);

    let is_edit_mode = props.transaction.is_some();
    let title = if is_edit_mode { "Edit Transaction" } else { "Add Transaction" };

    // Get categories list
    let categories_list = match &*categories_state {
        FetchState::Success(categories) => categories.clone(),
        _ => vec![],
    };

    let on_submit = {
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let form_ref = form_ref.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let transaction = props.transaction.clone();
        let scenario_id = props.scenario_id;
        let is_edit = transaction.is_some();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *is_submitting {
                return;
            }

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let name = form_data.get("name").as_string().unwrap_or_default();
                let description = form_data.get("description").as_string();
                let amount_str = form_data.get("amount").as_string().unwrap_or_default();
                let date_str = form_data.get("date").as_string().unwrap_or_default();
                let target_account_id_str = form_data.get("target_account_id").as_string().unwrap_or_default();
                let source_account_id_str = form_data.get("source_account_id").as_string();
                let ledger_name = form_data.get("ledger_name").as_string();
                let include_in_statistics = form_data.get("include_in_statistics").as_string().map(|v| v == "on").unwrap_or(true);
                let category_id_str = form_data.get("category_id").as_string();

                // Parse amount
                let amount = match Decimal::from_str(&amount_str) {
                    Ok(amt) => amt,
                    Err(_) => {
                        error_message.set(Some("Invalid amount format".to_string()));
                        return;
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

                // Parse target account ID
                let target_account_id = match target_account_id_str.parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        error_message.set(Some("Please select a target account".to_string()));
                        return;
                    }
                };

                // Parse source account ID (optional)
                let source_account_id = source_account_id_str.and_then(|s| {
                    if s.is_empty() || s == "none" {
                        None
                    } else {
                        s.parse::<i32>().ok()
                    }
                });

                // Parse category ID (optional)
                let category_id = category_id_str.and_then(|s| {
                    if s.is_empty() || s == "none" {
                        None
                    } else {
                        s.parse::<i32>().ok()
                    }
                });

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

                if is_edit {
                    // Edit mode - update transaction
                    let existing_transaction = transaction.clone().unwrap();
                    let transaction_id = existing_transaction.id;
                    let request = UpdateTransactionRequest {
                        name: Some(name.clone()),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                        amount: Some(amount),
                        date: Some(date),
                        include_in_statistics: Some(include_in_statistics),
                        target_account_id: Some(target_account_id),
                        source_account_id,
                        ledger_name: if ledger_name.as_ref().map(|l| l.is_empty()).unwrap_or(true) { None } else { ledger_name },
                        linked_import_id: None,
                        category_id,
                        scenario_id: None,
                        is_simulated: None,
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Updating transaction ID {}: {}", transaction_id, name);
                        match update_transaction(transaction_id, request).await {
                            Ok(transaction) => {
                                log::info!("Transaction updated successfully: {} (ID: {})", transaction.name, transaction.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to update transaction: {}", e);
                                error_message.set(Some(format!("Failed to update transaction: {}", e)));
                                is_submitting.set(false);
                            }
                        }
                    });
                } else {
                    // Create mode - create new transaction
                    let request = CreateTransactionRequest {
                        name: name.clone(),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                        amount,
                        date,
                        include_in_statistics: Some(include_in_statistics),
                        target_account_id,
                        source_account_id,
                        ledger_name: if ledger_name.as_ref().map(|l| l.is_empty()).unwrap_or(true) { None } else { ledger_name },
                        linked_import_id: None,
                        category_id,
                        scenario_id,
                        is_simulated: scenario_id.map(|_| true), // Mark as simulated if scenario is provided
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Creating transaction: {}", name);
                        match create_transaction(request).await {
                            Ok(transaction) => {
                                log::info!("Transaction created successfully: {} (ID: {})", transaction.name, transaction.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to create transaction: {}", e);
                                error_message.set(Some(format!("Failed to create transaction: {}", e)));
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

    // Get default values from transaction if in edit mode
    let default_name = props.transaction.as_ref().map(|t| t.name.clone()).unwrap_or_default();
    let default_description = props.transaction.as_ref().and_then(|t| t.description.clone()).unwrap_or_default();
    let default_amount = props.transaction.as_ref().map(|t| t.amount.to_string()).unwrap_or_default();
    let default_date = props.transaction.as_ref().map(|t| t.date.format("%Y-%m-%d").to_string()).unwrap_or_else(|| {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    });
    let default_target_account = props.transaction.as_ref().map(|t| t.target_account_id).unwrap_or(0);
    let default_source_account = props.transaction.as_ref().and_then(|t| t.source_account_id);
    let default_ledger = props.transaction.as_ref().and_then(|t| t.ledger_name.clone()).unwrap_or_default();
    let default_include_stats = props.transaction.as_ref().map(|t| t.include_in_statistics).unwrap_or(true);
    let default_category = props.transaction.as_ref().and_then(|t| t.category_id);

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="transaction_modal">
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
                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Transaction Name"}</span></label>
                        <input
                            type="text"
                            name="name"
                            class="input input-bordered w-full"
                            placeholder="e.g. Grocery Shopping"
                            value={default_name}
                            required={true}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Description (Optional)"}</span></label>
                        <textarea
                            name="description"
                            class="textarea textarea-bordered w-full"
                            placeholder="Additional details about this transaction"
                            value={default_description}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">{"Amount"}</span>
                                <span class="label-text-alt text-gray-500">{"Negative for expense"}</span>
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
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Date"}</span></label>
                            <input
                                type="date"
                                name="date"
                                class="input input-bordered w-full"
                                value={default_date}
                                required={true}
                                disabled={*is_submitting}
                            />
                        </div>
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Target Account"}</span></label>
                        <select name="target_account_id" class="select select-bordered w-full" required={true} disabled={*is_submitting}>
                            <option value="" disabled={true} selected={default_target_account == 0}>{"Select target account"}</option>
                            { for props.accounts.iter().map(|account| {
                                html! {
                                    <option 
                                        value={account.id.to_string()} 
                                        selected={default_target_account == account.id}
                                    >
                                        {&account.name}
                                    </option>
                                }
                            })}
                        </select>
                    </div>

                    <div class="form-control">
                        <label class="label">
                            <span class="label-text">{"Source Account (Optional)"}</span>
                            <span class="label-text-alt text-gray-500">{"For transfers"}</span>
                        </label>
                        <select name="source_account_id" class="select select-bordered w-full" disabled={*is_submitting}>
                            <option value="none" selected={default_source_account.is_none()}>{"No source account"}</option>
                            { for props.accounts.iter().map(|account| {
                                html! {
                                    <option 
                                        value={account.id.to_string()} 
                                        selected={default_source_account == Some(account.id)}
                                    >
                                        {&account.name}
                                    </option>
                                }
                            })}
                        </select>
                    </div>

                    <div class="form-control">
                        <label class="label">
                            <span class="label-text">{"Category (Optional)"}</span>
                        </label>
                        <select name="category_id" class="select select-bordered w-full" disabled={*is_submitting}>
                            <option value="none" selected={default_category.is_none()}>{"No category"}</option>
                            { for categories_list.iter().map(|category| {
                                html! {
                                    <option
                                        value={category.id.to_string()}
                                        selected={default_category == Some(category.id)}
                                    >
                                        {&category.name}
                                    </option>
                                }
                            })}
                        </select>
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Ledger Name (Optional)"}</span></label>
                        <input
                            type="text"
                            name="ledger_name"
                            class="input input-bordered w-full"
                            placeholder="e.g. Expenses:Groceries"
                            value={default_ledger}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label cursor-pointer justify-start gap-2">
                            <input
                                type="checkbox"
                                name="include_in_statistics"
                                class="checkbox checkbox-primary"
                                checked={default_include_stats}
                                disabled={*is_submitting}
                            />
                            <span class="label-text">{"Include in Statistics"}</span>
                        </label>
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
                                    html! { "Update Transaction" }
                                } else {
                                    html! { "Create Transaction" }
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

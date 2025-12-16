use yew::prelude::*;
use crate::api_client::recurring_transaction::{
    RecurringTransactionResponse, CreateRecurringTransactionRequest,
    UpdateRecurringTransactionRequest, create_recurring_transaction, update_recurring_transaction,
};
use crate::api_client::account::get_accounts;
use crate::api_client::category::get_categories;
use crate::api_client::scenario::get_scenarios;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

#[derive(Properties, PartialEq)]
pub struct RecurringModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    /// If provided, the modal is in edit mode with this recurring transaction
    pub transaction: Option<RecurringTransactionResponse>,
}

#[function_component(RecurringModal)]
pub fn recurring_modal(props: &RecurringModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    // Fetch accounts, categories and scenarios for dropdowns
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let (categories_state, _) = use_fetch_with_refetch(get_categories);
    let (scenarios_state, _) = use_fetch_with_refetch(get_scenarios);

    let is_edit_mode = props.transaction.is_some();
    let title = if is_edit_mode { "Edit Recurring Transaction" } else { "Add Recurring Transaction" };

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
                let amount = form_data.get("amount").as_string().unwrap_or_default();
                let start_date = form_data.get("start_date").as_string().unwrap_or_default();
                let end_date = form_data.get("end_date").as_string();
                let period = form_data.get("period").as_string().unwrap_or("Monthly".to_string());
                let target_account_id = form_data.get("target_account_id").as_string()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                let source_account_id = form_data.get("source_account_id").as_string()
                    .and_then(|s| if s.is_empty() { None } else { s.parse::<i32>().ok() });
                let ledger_name = form_data.get("ledger_name").as_string();
                let include_in_statistics = form_data.get("include_in_statistics").as_string().map(|v| v == "on").unwrap_or(true);
                let category_id = form_data.get("category_id").as_string()
                    .and_then(|s| if s.is_empty() || s == "none" { None } else { s.parse::<i32>().ok() });
                let scenario_id = form_data.get("scenario_id").as_string()
                    .and_then(|s| if s.is_empty() || s == "none" { None } else { s.parse::<i32>().ok() });
                let is_simulated = form_data.get("is_simulated").as_string().map(|v| v == "on").unwrap_or(false);

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

                if is_edit {
                    // Edit mode - update recurring transaction
                    let existing_transaction = transaction.clone().unwrap();
                    let transaction_id = existing_transaction.id;
                    let request = UpdateRecurringTransactionRequest {
                        name: Some(name.clone()),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                        amount: Some(amount),
                        start_date: Some(start_date),
                        end_date: if end_date.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { end_date },
                        period: Some(period),
                        include_in_statistics: Some(include_in_statistics),
                        target_account_id: Some(target_account_id),
                        source_account_id,
                        ledger_name: if ledger_name.as_ref().map(|l| l.is_empty()).unwrap_or(true) { None } else { ledger_name },
                        category_id,
                        scenario_id,
                        is_simulated: Some(is_simulated),
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Updating recurring transaction ID {}: {}", transaction_id, name);
                        match update_recurring_transaction(transaction_id, request).await {
                            Ok(transaction) => {
                                log::info!("Recurring transaction updated successfully: {} (ID: {})", transaction.name, transaction.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to update recurring transaction: {}", e);
                                error_message.set(Some(format!("Failed to update recurring transaction: {}", e)));
                                is_submitting.set(false);
                            }
                        }
                    });
                } else {
                    // Create mode - create new recurring transaction
                    let request = CreateRecurringTransactionRequest {
                        name: name.clone(),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                        amount,
                        start_date,
                        end_date: if end_date.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { end_date },
                        period,
                        include_in_statistics: Some(include_in_statistics),
                        target_account_id,
                        source_account_id,
                        ledger_name: if ledger_name.as_ref().map(|l| l.is_empty()).unwrap_or(true) { None } else { ledger_name },
                        category_id,
                        scenario_id,
                        is_simulated: Some(is_simulated),
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Creating recurring transaction: {}", name);
                        match create_recurring_transaction(request).await {
                            Ok(transaction) => {
                                log::info!("Recurring transaction created successfully: {} (ID: {})", transaction.name, transaction.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to create recurring transaction: {}", e);
                                error_message.set(Some(format!("Failed to create recurring transaction: {}", e)));
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
    let default_amount = props.transaction.as_ref().map(|t| t.amount.clone()).unwrap_or_default();
    let default_start_date = props.transaction.as_ref().map(|t| t.start_date.clone()).unwrap_or_default();
    let default_end_date = props.transaction.as_ref().and_then(|t| t.end_date.clone()).unwrap_or_default();
    let default_period = props.transaction.as_ref().map(|t| t.period.clone()).unwrap_or_else(|| "Monthly".to_string());
    let default_target_account = props.transaction.as_ref().map(|t| t.target_account_id).unwrap_or(0);
    let default_source_account = props.transaction.as_ref().and_then(|t| t.source_account_id);
    let default_ledger = props.transaction.as_ref().and_then(|t| t.ledger_name.clone()).unwrap_or_default();
    let default_include_stats = props.transaction.as_ref().map(|t| t.include_in_statistics).unwrap_or(true);
    let default_category = props.transaction.as_ref().and_then(|t| t.category_id);
    let default_scenario = props.transaction.as_ref().and_then(|t| t.scenario_id);
    let default_is_simulated = props.transaction.as_ref().map(|t| t.is_simulated).unwrap_or(false);

    // Get scenarios list
    let scenarios_list = match &*scenarios_state {
        FetchState::Success(scenarios) => scenarios.clone(),
        _ => vec![],
    };

    // Get today's date for default start date
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="recurring_modal">
            <div class="modal-box w-11/12 max-w-3xl">
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
                            placeholder="e.g. Monthly Rent, Weekly Groceries"
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
                            placeholder="Additional details about this recurring transaction"
                            value={default_description}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">{"Amount"}</span>
                                <span class="label-text-alt text-xs">{"(negative for expenses)"}</span>
                            </label>
                            <input
                                type="text"
                                name="amount"
                                class="input input-bordered w-full"
                                placeholder="e.g. -1500.00 or 5000.00"
                                value={default_amount}
                                required={true}
                                disabled={*is_submitting}
                            />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Recurrence Period"}</span></label>
                            <select name="period" class="select select-bordered w-full" disabled={*is_submitting}>
                                <option value="Daily" selected={default_period == "Daily"}>{"Daily"}</option>
                                <option value="Weekly" selected={default_period == "Weekly"}>{"Weekly"}</option>
                                <option value="WorkDay" selected={default_period == "WorkDay"}>{"Work Days (Mon-Fri)"}</option>
                                <option value="Monthly" selected={default_period == "Monthly"}>{"Monthly"}</option>
                                <option value="Quarterly" selected={default_period == "Quarterly"}>{"Quarterly"}</option>
                                <option value="HalfYearly" selected={default_period == "HalfYearly"}>{"Half-Yearly"}</option>
                                <option value="Yearly" selected={default_period == "Yearly"}>{"Yearly"}</option>
                            </select>
                        </div>
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Start Date"}</span></label>
                            <input
                                type="date"
                                name="start_date"
                                class="input input-bordered w-full"
                                value={if default_start_date.is_empty() { today.clone() } else { default_start_date }}
                                required={true}
                                disabled={*is_submitting}
                            />
                        </div>
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">{"End Date (Optional)"}</span>
                                <span class="label-text-alt text-xs">{"(leave empty for indefinite)"}</span>
                            </label>
                            <input
                                type="date"
                                name="end_date"
                                class="input input-bordered w-full"
                                value={default_end_date}
                                disabled={*is_submitting}
                            />
                        </div>
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Target Account"}</span></label>
                            {match &*accounts_state {
                                FetchState::Success(accounts) => html! {
                                    <select name="target_account_id" class="select select-bordered w-full" required={true} disabled={*is_submitting}>
                                        <option value="" selected={default_target_account == 0}>{"Select an account"}</option>
                                        {for accounts.iter().map(|account| html! {
                                            <option
                                                value={account.id.to_string()}
                                                selected={default_target_account == account.id}
                                            >
                                                {&account.name}
                                            </option>
                                        })}
                                    </select>
                                },
                                FetchState::Loading => html! {
                                    <select class="select select-bordered w-full" disabled={true}>
                                        <option>{"Loading accounts..."}</option>
                                    </select>
                                },
                                FetchState::Error(_) => html! {
                                    <select class="select select-bordered w-full" disabled={true}>
                                        <option>{"Error loading accounts"}</option>
                                    </select>
                                },
                                FetchState::NotStarted => html! {
                                    <select class="select select-bordered w-full" disabled={true}>
                                        <option>{"..."}</option>
                                    </select>
                                },
                            }}
                        </div>
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">{"Source Account (Optional)"}</span>
                                <span class="label-text-alt text-xs">{"(for transfers)"}</span>
                            </label>
                            {match &*accounts_state {
                                FetchState::Success(accounts) => html! {
                                    <select name="source_account_id" class="select select-bordered w-full" disabled={*is_submitting}>
                                        <option value="" selected={default_source_account.is_none()}>{"None"}</option>
                                        {for accounts.iter().map(|account| html! {
                                            <option
                                                value={account.id.to_string()}
                                                selected={default_source_account == Some(account.id)}
                                            >
                                                {&account.name}
                                            </option>
                                        })}
                                    </select>
                                },
                                FetchState::Loading => html! {
                                    <select class="select select-bordered w-full" disabled={true}>
                                        <option>{"Loading accounts..."}</option>
                                    </select>
                                },
                                FetchState::Error(_) => html! {
                                    <select class="select select-bordered w-full" disabled={true}>
                                        <option>{"Error loading accounts"}</option>
                                    </select>
                                },
                                FetchState::NotStarted => html! {
                                    <select class="select select-bordered w-full" disabled={true}>
                                        <option>{"..."}</option>
                                    </select>
                                },
                            }}
                        </div>
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
                        <label class="label">
                            <span class="label-text">{"Scenario (Optional)"}</span>
                        </label>
                        <select name="scenario_id" class="select select-bordered w-full" disabled={*is_submitting}>
                            <option value="none" selected={default_scenario.is_none()}>{"No scenario"}</option>
                            { for scenarios_list.iter().map(|scenario| {
                                html! {
                                    <option
                                        value={scenario.id.to_string()}
                                        selected={default_scenario == Some(scenario.id)}
                                    >
                                        {&scenario.name}
                                    </option>
                                }
                            })}
                        </select>
                    </div>

                    <div class="form-control">
                        <label class="label cursor-pointer justify-start gap-2">
                            <input
                                type="checkbox"
                                name="is_simulated"
                                class="checkbox checkbox-info"
                                checked={default_is_simulated}
                                disabled={*is_submitting}
                            />
                            <span class="label-text">{"Simulated Transaction"}</span>
                        </label>
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Ledger Name (Optional)"}</span></label>
                        <input
                            type="text"
                            name="ledger_name"
                            class="input input-bordered w-full"
                            placeholder="e.g. Expenses:Housing:Rent"
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

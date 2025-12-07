use yew::prelude::*;
use crate::api_client::account::{AccountKind, AccountResponse, CreateAccountRequest, UpdateAccountRequest, create_account, update_account};

#[derive(Properties, PartialEq)]
pub struct AccountModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    /// If provided, the modal is in edit mode with this account
    pub account: Option<AccountResponse>,
}

#[function_component(AccountModal)]
pub fn account_modal(props: &AccountModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    let is_edit_mode = props.account.is_some();
    let title = if is_edit_mode { "Edit Account" } else { "Add Account" };

    let on_submit = {
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let form_ref = form_ref.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let account = props.account.clone();
        let is_edit = account.is_some();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *is_submitting {
                return;
            }

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let name = form_data.get("name").as_string().unwrap_or_default();
                let description = form_data.get("description").as_string();
                let currency_code = form_data.get("currency_code").as_string().unwrap_or("CZK".to_string());
                let ledger_name = form_data.get("ledger_name").as_string();
                let account_kind_str = form_data.get("account_kind").as_string().unwrap_or("RealAccount".to_string());
                let include_in_statistics = form_data.get("include_in_statistics").as_string().map(|v| v == "on").unwrap_or(true);

                // Parse account kind
                let account_kind = match account_kind_str.as_str() {
                    "RealAccount" => AccountKind::RealAccount,
                    "Savings" => AccountKind::Savings,
                    "Investment" => AccountKind::Investment,
                    "Debt" => AccountKind::Debt,
                    "Other" => AccountKind::Other,
                    _ => AccountKind::RealAccount,
                };

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

                if is_edit {
                    // Edit mode - update account
                    let existing_account = account.clone().unwrap();
                    let account_id = existing_account.id;
                    let request = UpdateAccountRequest {
                        name: Some(name.clone()),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { Some(String::new()) } else { description },
                        currency_code: Some(currency_code),
                        include_in_statistics: Some(include_in_statistics),
                        ledger_name: if ledger_name.as_ref().map(|l| l.is_empty()).unwrap_or(true) { Some(String::new()) } else { ledger_name },
                        account_kind: Some(account_kind),
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Updating account ID {}: {}", account_id, name);
                        match update_account(account_id, request).await {
                            Ok(account) => {
                                log::info!("Account updated successfully: {} (ID: {})", account.name, account.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to update account: {}", e);
                                error_message.set(Some(format!("Failed to update account: {}", e)));
                                is_submitting.set(false);
                            }
                        }
                    });
                } else {
                    // Create mode - create new account
                    let request = CreateAccountRequest {
                        name: name.clone(),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                        currency_code,
                        owner_id: 1, // TODO: Get from user context
                        include_in_statistics: Some(include_in_statistics),
                        ledger_name: if ledger_name.as_ref().map(|l| l.is_empty()).unwrap_or(true) { None } else { ledger_name },
                        account_kind: Some(account_kind),
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Creating account: {}", name);
                        match create_account(request).await {
                            Ok(account) => {
                                log::info!("Account created successfully: {} (ID: {})", account.name, account.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to create account: {}", e);
                                error_message.set(Some(format!("Failed to create account: {}", e)));
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

    // Get default values from account if in edit mode
    let default_name = props.account.as_ref().map(|a| a.name.clone()).unwrap_or_default();
    let default_description = props.account.as_ref().and_then(|a| a.description.clone()).unwrap_or_default();
    let default_currency = props.account.as_ref().map(|a| a.currency_code.clone()).unwrap_or_else(|| "CZK".to_string());
    let default_ledger = props.account.as_ref().and_then(|a| a.ledger_name.clone()).unwrap_or_default();
    let default_include_stats = props.account.as_ref().map(|a| a.include_in_statistics).unwrap_or(true);
    let default_kind = props.account.as_ref().map(|a| a.account_kind).unwrap_or(AccountKind::RealAccount);

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="account_modal">
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
                        <label class="label"><span class="label-text">{"Account Name"}</span></label>
                        <input
                            type="text"
                            name="name"
                            class="input input-bordered w-full"
                            placeholder="e.g. Main Checking Account"
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
                            placeholder="Additional details about this account"
                            value={default_description}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Currency Code"}</span></label>
                            <input
                                type="text"
                                name="currency_code"
                                class="input input-bordered w-full"
                                placeholder="CZK"
                                value={default_currency}
                                required={true}
                                disabled={*is_submitting}
                            />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Account Type"}</span></label>
                            <select name="account_kind" class="select select-bordered w-full" disabled={*is_submitting}>
                                <option value="RealAccount" selected={default_kind == AccountKind::RealAccount}>{"Real Account"}</option>
                                <option value="Savings" selected={default_kind == AccountKind::Savings}>{"Savings"}</option>
                                <option value="Investment" selected={default_kind == AccountKind::Investment}>{"Investment"}</option>
                                <option value="Debt" selected={default_kind == AccountKind::Debt}>{"Debt"}</option>
                                <option value="Other" selected={default_kind == AccountKind::Other}>{"Other"}</option>
                            </select>
                        </div>
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Ledger Name (Optional)"}</span></label>
                        <input
                            type="text"
                            name="ledger_name"
                            class="input input-bordered w-full"
                            placeholder="e.g. Assets:Checking"
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
                                    html! { "Update Account" }
                                } else {
                                    html! { "Create Account" }
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

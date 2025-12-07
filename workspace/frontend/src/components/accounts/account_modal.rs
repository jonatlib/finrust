use yew::prelude::*;
use web_sys::HtmlInputElement;
use crate::api_client::account::{AccountKind, CreateAccountRequest, create_account};

#[derive(Properties, PartialEq)]
pub struct AccountModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
}

#[function_component(AccountModal)]
pub fn account_modal(props: &AccountModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    let on_submit = {
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let form_ref = form_ref.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();

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

                let request = CreateAccountRequest {
                    name: name.clone(),
                    description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                    currency_code,
                    owner_id: 1, // TODO: Get from user context
                    include_in_statistics: Some(include_in_statistics),
                    ledger_name: if ledger_name.as_ref().map(|l| l.is_empty()).unwrap_or(true) { None } else { ledger_name },
                    account_kind: Some(account_kind),
                };

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

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

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="account_modal">
            <div class="modal-box w-11/12 max-w-2xl">
                <h3 class="font-bold text-lg">{"Add Account"}</h3>

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
                                value="CZK"
                                required={true}
                                disabled={*is_submitting}
                            />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Account Type"}</span></label>
                            <select name="account_kind" class="select select-bordered w-full" disabled={*is_submitting}>
                                <option value="RealAccount">{"Real Account"}</option>
                                <option value="Savings">{"Savings"}</option>
                                <option value="Investment">{"Investment"}</option>
                                <option value="Debt">{"Debt"}</option>
                                <option value="Other">{"Other"}</option>
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
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label cursor-pointer justify-start gap-2">
                            <input
                                type="checkbox"
                                name="include_in_statistics"
                                class="checkbox checkbox-primary"
                                checked={true}
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
                                html! { <><span class="loading loading-spinner loading-sm"></span>{" Creating..."}</> }
                            } else {
                                html! { "Create Account" }
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

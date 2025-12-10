use yew::prelude::*;
use crate::api_client::recurring_transaction::{
    RecurringTransactionResponse, CreateRecurringInstanceRequest, create_recurring_instance,
};

#[derive(Properties, PartialEq)]
pub struct InstanceModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    /// The recurring transaction for which to create an instance
    pub transaction: RecurringTransactionResponse,
}

#[function_component(InstanceModal)]
pub fn instance_modal(props: &InstanceModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);
    let use_custom_amount = use_state(|| false);

    let on_submit = {
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let form_ref = form_ref.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let transaction_id = props.transaction.id;
        let use_custom_amount = *use_custom_amount;

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *is_submitting {
                return;
            }

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let date = form_data.get("date").as_string().unwrap_or_default();
                let amount = if use_custom_amount {
                    form_data.get("amount").as_string()
                } else {
                    None
                };

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

                let request = CreateRecurringInstanceRequest {
                    date,
                    amount,
                };

                wasm_bindgen_futures::spawn_local(async move {
                    log::info!("Creating instance for recurring transaction ID: {}", transaction_id);
                    match create_recurring_instance(transaction_id, request).await {
                        Ok(instance) => {
                            log::info!("Instance created successfully with ID: {}", instance.id);
                            is_submitting.set(false);
                            on_success.emit(());
                            on_close.emit(());
                        }
                        Err(e) => {
                            log::error!("Failed to create instance: {}", e);
                            error_message.set(Some(format!("Failed to create instance: {}", e)));
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

    let on_toggle_custom_amount = {
        let use_custom_amount = use_custom_amount.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            use_custom_amount.set(input.checked());
        })
    };

    // Get today's date for default
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Parse amount for display
    let default_amount_display = match props.transaction.amount.parse::<f64>() {
        Ok(val) => {
            if val >= 0.0 {
                format!("+{:.2}", val)
            } else {
                format!("{:.2}", val)
            }
        }
        Err(_) => props.transaction.amount.clone(),
    };

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="instance_modal">
            <div class="modal-box w-11/12 max-w-lg">
                <h3 class="font-bold text-lg">{"Create Transaction Instance"}</h3>

                <div class="bg-base-200 p-4 rounded-lg my-4">
                    <p class="text-sm font-semibold">{"Recurring Transaction:"}</p>
                    <p class="text-lg">{&props.transaction.name}</p>
                    <p class="text-sm opacity-70">{format!("Period: {}", props.transaction.period)}</p>
                    <p class="text-sm opacity-70">{format!("Default Amount: {}", default_amount_display)}</p>
                </div>

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
                        <label class="label">
                            <span class="label-text">{"Due Date"}</span>
                            <span class="label-text-alt text-xs">{"When should this occur?"}</span>
                        </label>
                        <input
                            type="date"
                            name="date"
                            class="input input-bordered w-full"
                            value={today}
                            required={true}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label cursor-pointer justify-start gap-2">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-primary"
                                checked={*use_custom_amount}
                                onchange={on_toggle_custom_amount}
                                disabled={*is_submitting}
                            />
                            <span class="label-text">{"Use custom amount (override default)"}</span>
                        </label>
                    </div>

                    {if *use_custom_amount {
                        html! {
                            <div class="form-control">
                                <label class="label">
                                    <span class="label-text">{"Custom Amount"}</span>
                                    <span class="label-text-alt text-xs">{"(negative for expenses)"}</span>
                                </label>
                                <input
                                    type="text"
                                    name="amount"
                                    class="input input-bordered w-full"
                                    placeholder={props.transaction.amount.clone()}
                                    value={props.transaction.amount.clone()}
                                    disabled={*is_submitting}
                                />
                            </div>
                        }
                    } else {
                        html! {}
                    }}

                    <div class="alert alert-info">
                        <i class="fas fa-info-circle"></i>
                        <div class="text-sm">
                            <p>{"This will create a new instance with status "}<strong>{"Pending"}</strong>{"."}</p>
                            <p class="mt-1">{"You can mark it as paid or skipped later."}</p>
                        </div>
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
                                html! { "Create Instance" }
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

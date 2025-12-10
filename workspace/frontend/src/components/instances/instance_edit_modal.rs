use yew::prelude::*;
use crate::api_client::recurring_transaction::{
    RecurringInstanceResponse, UpdateRecurringInstanceRequest, update_recurring_instance,
};

#[derive(Properties, PartialEq)]
pub struct InstanceEditModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    pub instance: RecurringInstanceResponse,
}

#[function_component(InstanceEditModal)]
pub fn instance_edit_modal(props: &InstanceEditModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    let on_submit = {
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let form_ref = form_ref.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let instance_id = props.instance.id;

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *is_submitting {
                return;
            }

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let status = form_data.get("status").as_string();
                let due_date = form_data.get("due_date").as_string();
                let expected_amount = form_data.get("expected_amount").as_string();
                let paid_date = form_data.get("paid_date").as_string();
                let paid_amount = form_data.get("paid_amount").as_string();

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

                let request = UpdateRecurringInstanceRequest {
                    status,
                    due_date,
                    expected_amount,
                    paid_date: if paid_date.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { paid_date },
                    paid_amount: if paid_amount.as_ref().map(|a| a.is_empty()).unwrap_or(true) { None } else { paid_amount },
                };

                wasm_bindgen_futures::spawn_local(async move {
                    log::info!("Updating recurring instance ID: {}", instance_id);
                    match update_recurring_instance(instance_id, request).await {
                        Ok(instance) => {
                            log::info!("Instance updated successfully: ID {}", instance.id);
                            is_submitting.set(false);
                            on_success.emit(());
                            on_close.emit(());
                        }
                        Err(e) => {
                            log::error!("Failed to update instance: {}", e);
                            error_message.set(Some(format!("Failed to update instance: {}", e)));
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
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="instance_edit_modal">
            <div class="modal-box w-11/12 max-w-2xl">
                <h3 class="font-bold text-lg">{"Edit Instance"}</h3>

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
                        <label class="label"><span class="label-text">{"Status"}</span></label>
                        <select name="status" class="select select-bordered w-full" disabled={*is_submitting}>
                            <option value="Pending" selected={props.instance.status == "Pending"}>{"Pending"}</option>
                            <option value="Paid" selected={props.instance.status == "Paid"}>{"Paid"}</option>
                            <option value="Skipped" selected={props.instance.status == "Skipped"}>{"Skipped"}</option>
                        </select>
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Due Date"}</span></label>
                            <input
                                type="date"
                                name="due_date"
                                class="input input-bordered w-full"
                                value={props.instance.due_date.clone()}
                                disabled={*is_submitting}
                            />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Expected Amount"}</span></label>
                            <input
                                type="text"
                                name="expected_amount"
                                class="input input-bordered w-full"
                                value={props.instance.expected_amount.clone()}
                                disabled={*is_submitting}
                            />
                        </div>
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Paid Date (Optional)"}</span></label>
                            <input
                                type="date"
                                name="paid_date"
                                class="input input-bordered w-full"
                                value={props.instance.paid_date.clone().unwrap_or_default()}
                                disabled={*is_submitting}
                            />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Paid Amount (Optional)"}</span></label>
                            <input
                                type="text"
                                name="paid_amount"
                                class="input input-bordered w-full"
                                value={props.instance.paid_amount.clone().unwrap_or_default()}
                                disabled={*is_submitting}
                            />
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
                                html! { <><span class="loading loading-spinner loading-sm"></span>{" Updating..."}</> }
                            } else {
                                html! { "Update Instance" }
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

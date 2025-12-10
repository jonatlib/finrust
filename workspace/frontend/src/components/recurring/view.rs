use yew::prelude::*;
use super::list::RecurringList;
use super::recurring_modal::RecurringModal;
use super::instance_modal::InstanceModal;
use crate::api_client::recurring_transaction::{
    RecurringTransactionResponse, get_recurring_transaction,
    create_recurring_instance, CreateRecurringInstanceRequest
};
use crate::common::toast::ToastContext;

#[derive(Properties, PartialEq)]
pub struct RecurringProps {
    #[prop_or_default]
    pub account_id: Option<i32>,
}

#[function_component(Recurring)]
pub fn recurring(props: &RecurringProps) -> Html {
    let refresh_trigger = use_state(|| 0);
    let show_modal = use_state(|| false);
    let show_instance_modal = use_state(|| false);
    let edit_transaction = use_state(|| None::<RecurringTransactionResponse>);
    let instance_transaction = use_state(|| None::<RecurringTransactionResponse>);
    let toast_ctx = use_context::<ToastContext>().expect("ToastContext not found");

    let on_edit = {
        let show_modal = show_modal.clone();
        let edit_transaction = edit_transaction.clone();
        Callback::from(move |id: i32| {
            log::info!("Edit recurring transaction with ID: {}", id);
            let show_modal = show_modal.clone();
            let edit_transaction = edit_transaction.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match get_recurring_transaction(id).await {
                    Ok(transaction) => {
                        log::info!("Loaded transaction for editing: {}", transaction.name);
                        edit_transaction.set(Some(transaction));
                        show_modal.set(true);
                    }
                    Err(e) => {
                        log::error!("Failed to load transaction for editing: {}", e);
                    }
                }
            });
        })
    };

    let on_create_instance = {
        let show_instance_modal = show_instance_modal.clone();
        let instance_transaction = instance_transaction.clone();
        Callback::from(move |id: i32| {
            log::info!("Create instance for recurring transaction ID: {}", id);
            let show_instance_modal = show_instance_modal.clone();
            let instance_transaction = instance_transaction.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match get_recurring_transaction(id).await {
                    Ok(transaction) => {
                        log::info!("Loaded transaction for instance creation: {}", transaction.name);
                        instance_transaction.set(Some(transaction));
                        show_instance_modal.set(true);
                    }
                    Err(e) => {
                        log::error!("Failed to load transaction for instance creation: {}", e);
                    }
                }
            });
        })
    };

    let on_add_click = {
        let show_modal = show_modal.clone();
        let edit_transaction = edit_transaction.clone();
        Callback::from(move |_: MouseEvent| {
            log::info!("Add new recurring transaction clicked");
            edit_transaction.set(None);
            show_modal.set(true);
        })
    };

    let on_modal_close = {
        let show_modal = show_modal.clone();
        let edit_transaction = edit_transaction.clone();
        Callback::from(move |_| {
            log::info!("Closing recurring transaction modal");
            show_modal.set(false);
            edit_transaction.set(None);
        })
    };

    let on_modal_success = {
        let refresh_trigger = refresh_trigger.clone();
        let show_modal = show_modal.clone();
        let edit_transaction = edit_transaction.clone();
        Callback::from(move |_| {
            log::info!("Recurring transaction saved successfully, refreshing list");
            refresh_trigger.set(*refresh_trigger + 1);
            show_modal.set(false);
            edit_transaction.set(None);
        })
    };

    let on_instance_modal_close = {
        let show_instance_modal = show_instance_modal.clone();
        let instance_transaction = instance_transaction.clone();
        Callback::from(move |_| {
            log::info!("Closing instance creation modal");
            show_instance_modal.set(false);
            instance_transaction.set(None);
        })
    };

    let on_instance_modal_success = {
        let refresh_trigger = refresh_trigger.clone();
        let show_instance_modal = show_instance_modal.clone();
        let instance_transaction = instance_transaction.clone();
        Callback::from(move |_| {
            log::info!("Instance created successfully");
            refresh_trigger.set(*refresh_trigger + 1);
            show_instance_modal.set(false);
            instance_transaction.set(None);
        })
    };

    let on_quick_create_instance = {
        let refresh_trigger = refresh_trigger.clone();
        let toast_ctx = toast_ctx.clone();
        Callback::from(move |id: i32| {
            log::info!("Quick create instance for recurring transaction ID: {}", id);
            let refresh_trigger = refresh_trigger.clone();
            let toast_ctx = toast_ctx.clone();

            wasm_bindgen_futures::spawn_local(async move {
                // Get today's date
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();

                let request = CreateRecurringInstanceRequest {
                    date: today.clone(),
                    amount: None, // Use default amount
                };

                match create_recurring_instance(id, request).await {
                    Ok(instance) => {
                        log::info!("Instance created successfully with ID: {}", instance.id);
                        toast_ctx.show_success(format!("Instance created for {}", today));
                        refresh_trigger.set(*refresh_trigger + 1);
                    }
                    Err(e) => {
                        log::error!("Failed to create instance: {}", e);
                        toast_ctx.show_error(format!("Failed to create instance: {}", e));
                    }
                }
            });
        })
    };

    html! {
        <>
            <div class="flex justify-end mb-4">
                <button class="btn btn-primary" onclick={on_add_click}>
                    <i class="fas fa-plus"></i> {" Add Recurring Transaction"}
                </button>
            </div>
            <RecurringList
                key={*refresh_trigger}
                on_edit={Some(on_edit)}
                on_create_instance={Some(on_create_instance)}
                on_quick_create_instance={Some(on_quick_create_instance)}
                account_id={props.account_id}
            />
            <RecurringModal
                show={*show_modal}
                on_close={on_modal_close}
                on_success={on_modal_success}
                transaction={(*edit_transaction).clone()}
            />
            {if let Some(transaction) = (*instance_transaction).clone() {
                html! {
                    <InstanceModal
                        show={*show_instance_modal}
                        on_close={on_instance_modal_close}
                        on_success={on_instance_modal_success}
                        transaction={transaction}
                    />
                }
            } else {
                html! {}
            }}
        </>
    }
}

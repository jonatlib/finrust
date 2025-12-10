use yew::prelude::*;
use super::list::RecurringList;
use super::recurring_modal::RecurringModal;
use crate::api_client::recurring_transaction::{RecurringTransactionResponse, get_recurring_transaction};

#[derive(Properties, PartialEq)]
pub struct RecurringProps {
    #[prop_or_default]
    pub account_id: Option<i32>,
}

#[function_component(Recurring)]
pub fn recurring(props: &RecurringProps) -> Html {
    let refresh_trigger = use_state(|| 0);
    let show_modal = use_state(|| false);
    let edit_transaction = use_state(|| None::<RecurringTransactionResponse>);

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
        Callback::from(move |id: i32| {
            log::info!("Create instance for recurring transaction ID: {}", id);
            // TODO: Implement instance creation modal
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
                account_id={props.account_id}
            />
            <RecurringModal
                show={*show_modal}
                on_close={on_modal_close}
                on_success={on_modal_success}
                transaction={(*edit_transaction).clone()}
            />
        </>
    }
}

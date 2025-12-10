use yew::prelude::*;
use super::list::InstancesList;
use super::instance_edit_modal::InstanceEditModal;
use crate::api_client::recurring_transaction::{RecurringInstanceResponse, get_recurring_instance};

#[derive(Properties, PartialEq)]
pub struct InstancesProps {
    #[prop_or_default]
    pub recurring_transaction_id: Option<i32>,
}

#[function_component(Instances)]
pub fn instances(props: &InstancesProps) -> Html {
    let refresh_trigger = use_state(|| 0);
    let show_edit_modal = use_state(|| false);
    let edit_instance = use_state(|| None::<RecurringInstanceResponse>);

    let on_edit = {
        let show_edit_modal = show_edit_modal.clone();
        let edit_instance = edit_instance.clone();
        Callback::from(move |id: i32| {
            log::info!("Edit instance with ID: {}", id);
            let show_edit_modal = show_edit_modal.clone();
            let edit_instance = edit_instance.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match get_recurring_instance(id).await {
                    Ok(instance) => {
                        log::info!("Loaded instance for editing: ID {}", instance.id);
                        edit_instance.set(Some(instance));
                        show_edit_modal.set(true);
                    }
                    Err(e) => {
                        log::error!("Failed to load instance for editing: {}", e);
                    }
                }
            });
        })
    };

    let on_edit_modal_close = {
        let show_edit_modal = show_edit_modal.clone();
        let edit_instance = edit_instance.clone();
        Callback::from(move |_| {
            log::info!("Closing instance edit modal");
            show_edit_modal.set(false);
            edit_instance.set(None);
        })
    };

    let on_edit_modal_success = {
        let refresh_trigger = refresh_trigger.clone();
        let show_edit_modal = show_edit_modal.clone();
        let edit_instance = edit_instance.clone();
        Callback::from(move |_| {
            log::info!("Instance updated successfully, refreshing list");
            refresh_trigger.set(*refresh_trigger + 1);
            show_edit_modal.set(false);
            edit_instance.set(None);
        })
    };

    html! {
        <>
            <InstancesList
                key={*refresh_trigger}
                on_edit={Some(on_edit)}
                recurring_transaction_id={props.recurring_transaction_id}
            />
            {if let Some(instance) = (*edit_instance).clone() {
                html! {
                    <InstanceEditModal
                        show={*show_edit_modal}
                        on_close={on_edit_modal_close}
                        on_success={on_edit_modal_success}
                        instance={instance}
                    />
                }
            } else {
                html! {}
            }}
        </>
    }
}

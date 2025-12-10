use yew::prelude::*;
use yew_router::prelude::*;
use std::collections::HashSet;
use crate::api_client::recurring_transaction::{
    MissingInstanceInfo, get_missing_instances, bulk_create_instances,
    BulkCreateInstancesRequest, BulkInstanceItem
};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::common::toast::ToastContext;
use crate::hooks::FetchState;
use crate::router::Route;

#[derive(Properties, PartialEq)]
pub struct MissingInstancesProps {
    #[prop_or_default]
    pub on_instances_created: Callback<()>,
}

#[function_component(MissingInstances)]
pub fn missing_instances(props: &MissingInstancesProps) -> Html {
    let toast_ctx = use_context::<ToastContext>().expect("ToastContext not found");
    let selected = use_state(|| HashSet::<String>::new());
    let is_creating = use_state(|| false);

    let (fetch_state, refetch) = use_fetch_with_refetch(move || {
        get_missing_instances(None, None, None)
    });

    let on_select_all = {
        let selected = selected.clone();
        let fetch_state = fetch_state.clone();
        Callback::from(move |e: Event| {
            let checked = e.target_unchecked_into::<web_sys::HtmlInputElement>().checked();
            if let FetchState::Success(instances) = &*fetch_state {
                if checked {
                    let all_keys: HashSet<String> = instances
                        .iter()
                        .map(|i| format!("{}-{}", i.recurring_transaction_id, i.due_date))
                        .collect();
                    selected.set(all_keys);
                } else {
                    selected.set(HashSet::new());
                }
            }
        })
    };

    let create_instances = {
        let selected = selected.clone();
        let fetch_state = fetch_state.clone();
        let is_creating = is_creating.clone();
        let toast_ctx = toast_ctx.clone();
        let refetch = refetch.clone();
        let on_instances_created = props.on_instances_created.clone();

        move |as_paid: bool| {
            if selected.is_empty() {
                return;
            }

            let selected = selected.clone();
            let fetch_state = fetch_state.clone();
            let is_creating = is_creating.clone();
            let toast_ctx = toast_ctx.clone();
            let refetch = refetch.clone();
            let on_instances_created = on_instances_created.clone();

            wasm_bindgen_futures::spawn_local(async move {
                is_creating.set(true);

                if let FetchState::Success(instances) = &*fetch_state {
                    // Build bulk request with selected instances
                    let bulk_items: Vec<BulkInstanceItem> = instances
                        .iter()
                        .filter(|instance| {
                            let key = format!("{}-{}", instance.recurring_transaction_id, instance.due_date);
                            selected.contains(&key)
                        })
                        .map(|instance| BulkInstanceItem {
                            recurring_transaction_id: instance.recurring_transaction_id,
                            due_date: instance.due_date.clone(),
                            expected_amount: instance.expected_amount.clone(),
                            instance_id: instance.instance_id,
                        })
                        .collect();

                    let request = BulkCreateInstancesRequest {
                        instances: bulk_items,
                        mark_as_paid: as_paid,
                    };

                    match bulk_create_instances(request).await {
                        Ok(response) => {
                            is_creating.set(false);

                            let mut messages = Vec::new();
                            if response.created > 0 {
                                messages.push(format!("{} created", response.created));
                            }
                            if response.updated > 0 {
                                messages.push(format!("{} updated", response.updated));
                            }
                            if response.skipped > 0 {
                                messages.push(format!("{} skipped", response.skipped));
                            }

                            if !messages.is_empty() {
                                toast_ctx.show_success(format!("Instances processed: {}", messages.join(", ")));
                            }

                            refetch.emit(());
                            on_instances_created.emit(());
                        }
                        Err(e) => {
                            is_creating.set(false);
                            log::error!("Failed to bulk create instances: {}", e);
                            toast_ctx.show_error(format!("Failed to create instances: {}", e));
                        }
                    }
                }
            });
        }
    };

    let on_create_pending = {
        let create_instances = create_instances.clone();
        Callback::from(move |_| {
            create_instances(false);
        })
    };

    let on_create_paid = {
        let create_instances = create_instances.clone();
        Callback::from(move |_| {
            create_instances(true);
        })
    };

    let format_currency = |amount: &str| -> String {
        match amount.parse::<f64>() {
            Ok(val) => format!("${:.2}", val.abs()),
            Err(_) => amount.to_string(),
        }
    };

    html! {
        <div class="card bg-base-100 shadow-xl">
            <div class="card-body">
                <h2 class="card-title">{"Missing Instances"}</h2>

                {match &*fetch_state {
                    FetchState::Success(instances) if !instances.is_empty() => {
                        let all_selected = instances.len() == selected.len();

                        html! {
                            <>
                                <div class="overflow-x-auto mb-4">
                                    <table class="table table-zebra table-sm">
                                        <thead>
                                            <tr>
                                                <th>
                                                    <input
                                                        type="checkbox"
                                                        class="checkbox checkbox-sm"
                                                        checked={all_selected}
                                                        onchange={on_select_all}
                                                    />
                                                </th>
                                                <th>{"Recurring Transaction"}</th>
                                                <th>{"Due Date"}</th>
                                                <th>{"Expected Amount"}</th>
                                                <th>{"Status"}</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            { for instances.iter().map(|instance| {
                                                let key = format!("{}-{}", instance.recurring_transaction_id, instance.due_date);
                                                let is_selected = selected.contains(&key);

                                                let expected_amount = match instance.expected_amount.parse::<f64>() {
                                                    Ok(val) => val,
                                                    Err(_) => 0.0,
                                                };
                                                let amount_class = if expected_amount >= 0.0 { "text-success" } else { "text-error" };
                                                let amount_prefix = if expected_amount >= 0.0 { "+" } else { "-" };

                                                let on_toggle = {
                                                    let selected = selected.clone();
                                                    let key = key.clone();
                                                    Callback::from(move |e: Event| {
                                                        let checked = e.target_unchecked_into::<web_sys::HtmlInputElement>().checked();
                                                        let mut new_selected = (*selected).clone();
                                                        if checked {
                                                            new_selected.insert(key.clone());
                                                        } else {
                                                            new_selected.remove(&key);
                                                        }
                                                        selected.set(new_selected);
                                                    })
                                                };

                                                html! {
                                                    <tr>
                                                        <td>
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-sm"
                                                                checked={is_selected}
                                                                onchange={on_toggle}
                                                            />
                                                        </td>
                                                        <td>
                                                            <Link<Route>
                                                                to={Route::RecurringDetail { id: instance.recurring_transaction_id }}
                                                                classes="link link-hover link-primary"
                                                            >
                                                                {&instance.recurring_transaction_name}
                                                            </Link<Route>>
                                                        </td>
                                                        <td>{&instance.due_date}</td>
                                                        <td class={classes!("font-mono", amount_class)}>
                                                            {format!("{}{}", amount_prefix, format_currency(&instance.expected_amount))}
                                                        </td>
                                                        <td>
                                                            if instance.is_pending {
                                                                <span class="badge badge-warning badge-sm">{"Pending"}</span>
                                                            } else {
                                                                <span class="badge badge-ghost badge-sm">{"Missing"}</span>
                                                            }
                                                        </td>
                                                    </tr>
                                                }
                                            })}
                                        </tbody>
                                    </table>
                                </div>

                                <p class="text-sm text-base-content/70 mb-4">{"Select instances to create them in bulk as Pending or Paid status"}</p>

                                <div class="flex gap-2">
                                    <button
                                        class="btn btn-primary btn-sm"
                                        disabled={selected.is_empty() || *is_creating}
                                        onclick={on_create_pending}
                                    >
                                        if *is_creating {
                                            <span class="loading loading-spinner loading-xs"></span>
                                        }
                                        {format!("Create {} as Pending", selected.len())}
                                    </button>
                                    <button
                                        class="btn btn-success btn-sm"
                                        disabled={selected.is_empty() || *is_creating}
                                        onclick={on_create_paid}
                                    >
                                        if *is_creating {
                                            <span class="loading loading-spinner loading-xs"></span>
                                        }
                                        {format!("Create {} as Paid", selected.len())}
                                    </button>
                                </div>
                            </>
                        }
                    }
                    FetchState::Success(_) => {
                        html! {
                            <div class="alert alert-success">
                                <i class="fas fa-check-circle"></i>
                                <span>{"No missing instances! All recurring transactions are up to date."}</span>
                            </div>
                        }
                    }
                    FetchState::Loading => {
                        html! {
                            <div class="flex justify-center p-8">
                                <span class="loading loading-spinner loading-lg"></span>
                            </div>
                        }
                    }
                    FetchState::Error(e) => {
                        html! {
                            <div class="alert alert-error">
                                <i class="fas fa-exclamation-circle"></i>
                                <span>{format!("Error loading missing instances: {}", e)}</span>
                            </div>
                        }
                    }
                    FetchState::NotStarted => html! { <></> }
                }}
            </div>
        </div>
    }
}

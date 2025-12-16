use yew::prelude::*;
use yew_router::prelude::*;
use crate::components::layout::layout::Layout;
use crate::router::Route;
use crate::api_client::recurring_transaction::{get_recurring_transaction, get_recurring_instances, delete_recurring_transaction, delete_recurring_instance, update_recurring_instance, UpdateRecurringInstanceRequest};
use crate::api_client::category::get_categories;
use crate::api_client::scenario::get_scenarios;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::common::toast::ToastContext;
use crate::hooks::FetchState;
use crate::components::instances::instance_edit_modal::InstanceEditModal;
use std::collections::HashMap;

#[derive(Properties, PartialEq)]
pub struct RecurringDetailPageProps {
    pub id: i32,
}

#[function_component(RecurringDetailPage)]
pub fn recurring_detail_page(props: &RecurringDetailPageProps) -> Html {
    let id = props.id;
    let navigator = use_navigator().unwrap();
    let toast_ctx = use_context::<ToastContext>().expect("ToastContext not found");

    // Fetch recurring transaction details
    let (transaction_state, transaction_refetch) = use_fetch_with_refetch(move || {
        get_recurring_transaction(id)
    });

    // Fetch instances for this recurring transaction
    let (instances_state, instances_refetch) = use_fetch_with_refetch(move || {
        get_recurring_instances(None, None, Some(id), None)
    });

    // Fetch categories
    let (categories_state, _) = use_fetch_with_refetch(get_categories);
    let (scenarios_state, _) = use_fetch_with_refetch(get_scenarios);

    // Build category ID -> name map
    let category_map: HashMap<i32, String> = match &*categories_state {
        FetchState::Success(categories) => categories
            .iter()
            .map(|cat| (cat.id, cat.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Build scenario ID -> name map
    let scenario_map: HashMap<i32, String> = match &*scenarios_state {
        FetchState::Success(scenarios) => scenarios
            .iter()
            .map(|s| (s.id, s.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    let edit_instance = use_state(|| None::<crate::api_client::recurring_transaction::RecurringInstanceResponse>);
    let show_edit_modal = use_state(|| false);

    let on_edit_recurring = {
        let navigator = navigator.clone();
        let id = id;
        Callback::from(move |_| {
            navigator.push(&Route::Recurring);
            // Note: The recurring page would need to handle opening the edit modal
            // For now, we just navigate back to the recurring list
        })
    };

    let on_delete_recurring = {
        let toast_ctx = toast_ctx.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            let id = id;
            let toast_ctx = toast_ctx.clone();
            let navigator = navigator.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match delete_recurring_transaction(id).await {
                    Ok(_) => {
                        toast_ctx.show_success("Recurring transaction deleted successfully".to_string());
                        navigator.push(&Route::Recurring);
                    }
                    Err(e) => {
                        toast_ctx.show_error(format!("Failed to delete recurring transaction: {}", e));
                    }
                }
            });
        })
    };

    let on_edit_instance = {
        let edit_instance = edit_instance.clone();
        let show_edit_modal = show_edit_modal.clone();
        let instances_state = instances_state.clone();
        Callback::from(move |instance_id: i32| {
            // Find the instance in the current list
            if let FetchState::Success(instances) = &*instances_state {
                if let Some(instance) = instances.iter().find(|i| i.id == instance_id) {
                    edit_instance.set(Some(instance.clone()));
                    show_edit_modal.set(true);
                }
            }
        })
    };

    let on_close_edit_modal = {
        let show_edit_modal = show_edit_modal.clone();
        let edit_instance = edit_instance.clone();
        Callback::from(move |_| {
            show_edit_modal.set(false);
            edit_instance.set(None);
        })
    };

    let on_instance_updated = {
        let instances_refetch = instances_refetch.clone();
        let show_edit_modal = show_edit_modal.clone();
        let edit_instance = edit_instance.clone();
        Callback::from(move |_| {
            instances_refetch.emit(());
            show_edit_modal.set(false);
            edit_instance.set(None);
        })
    };

    let format_currency = |amount: &str| -> String {
        match amount.parse::<f64>() {
            Ok(val) => format!("${:.2}", val.abs()),
            Err(_) => amount.to_string(),
        }
    };

    let render_transaction_details = || -> Html {
        match &*transaction_state {
            FetchState::Success(transaction) => {
                let amount = match transaction.amount.parse::<f64>() {
                    Ok(val) => val,
                    Err(_) => 0.0,
                };
                let amount_class = if amount >= 0.0 { "text-success" } else { "text-error" };
                let amount_prefix = if amount >= 0.0 { "+" } else { "-" };

                html! {
                    <div class="card bg-base-100 shadow-xl">
                        <div class="card-body">
                            <div class="flex justify-between items-start">
                                <h2 class="card-title text-2xl">{&transaction.name}</h2>
                                <div class="flex gap-2">
                                    <button
                                        class="btn btn-sm btn-ghost"
                                        onclick={on_edit_recurring}
                                    >
                                        <i class="fas fa-edit"></i>
                                        {"Edit"}
                                    </button>
                                    <button
                                        class="btn btn-sm btn-error btn-outline"
                                        onclick={on_delete_recurring}
                                    >
                                        <i class="fas fa-trash"></i>
                                        {"Delete"}
                                    </button>
                                </div>
                            </div>

                            {if let Some(desc) = &transaction.description {
                                html! { <p class="text-base-content/70">{desc}</p> }
                            } else {
                                html! { <></> }
                            }}

                            <div class="divider"></div>

                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                <div>
                                    <div class="stat bg-base-200 rounded-box">
                                        <div class="stat-title">{"Amount"}</div>
                                        <div class={classes!("stat-value", "text-2xl", "font-mono", amount_class)}>
                                            {format!("{}{}", amount_prefix, format_currency(&transaction.amount))}
                                        </div>
                                    </div>
                                </div>

                                <div>
                                    <div class="stat bg-base-200 rounded-box">
                                        <div class="stat-title">{"Period"}</div>
                                        <div class="stat-value text-2xl">{&transaction.period}</div>
                                    </div>
                                </div>

                                <div>
                                    <div class="stat bg-base-200 rounded-box">
                                        <div class="stat-title">{"Start Date"}</div>
                                        <div class="stat-value text-xl">{&transaction.start_date}</div>
                                    </div>
                                </div>

                                <div>
                                    <div class="stat bg-base-200 rounded-box">
                                        <div class="stat-title">{"End Date"}</div>
                                        <div class="stat-value text-xl">
                                            {transaction.end_date.as_ref().unwrap_or(&"Ongoing".to_string())}
                                        </div>
                                    </div>
                                </div>

                                <div>
                                    <div class="stat bg-base-200 rounded-box">
                                        <div class="stat-title">{"Target Account"}</div>
                                        <div class="stat-value text-lg">
                                            <Link<Route>
                                                to={Route::AccountEdit { id: transaction.target_account_id }}
                                                classes="link link-hover"
                                            >
                                                {format!("Account #{}", transaction.target_account_id)}
                                            </Link<Route>>
                                        </div>
                                    </div>
                                </div>

                                {if let Some(source_id) = transaction.source_account_id {
                                    html! {
                                        <div>
                                            <div class="stat bg-base-200 rounded-box">
                                                <div class="stat-title">{"Source Account"}</div>
                                                <div class="stat-value text-lg">
                                                    <Link<Route>
                                                        to={Route::AccountEdit { id: source_id }}
                                                        classes="link link-hover"
                                                    >
                                                        {format!("Account #{}", source_id)}
                                                    </Link<Route>>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! { <></> }
                                }}

                                {if let Some(ledger) = &transaction.ledger_name {
                                    html! {
                                        <div>
                                            <div class="stat bg-base-200 rounded-box">
                                                <div class="stat-title">{"Ledger Name"}</div>
                                                <div class="stat-value text-lg">{ledger}</div>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! { <></> }
                                }}

                                <div>
                                    <div class="stat bg-base-200 rounded-box">
                                        <div class="stat-title">{"Include in Statistics"}</div>
                                        <div class="stat-value text-lg">
                                            {if transaction.include_in_statistics {
                                                html! { <span class="badge badge-success">{"Yes"}</span> }
                                            } else {
                                                html! { <span class="badge badge-ghost">{"No"}</span> }
                                            }}
                                        </div>
                                    </div>
                                </div>

                                {if let Some(category_id) = transaction.category_id {
                                    html! {
                                        <div>
                                            <div class="stat bg-base-200 rounded-box">
                                                <div class="stat-title">{"Category"}</div>
                                                <div class="stat-value text-lg">
                                                    <span class="badge badge-info badge-lg">
                                                        <i class="fas fa-tag mr-1"></i>
                                                        {category_map.get(&category_id).map(|name| name.as_str()).unwrap_or("Unknown Category")}
                                                    </span>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! { <></> }
                                }}

                                <div>
                                    <div class="stat bg-base-200 rounded-box">
                                        <div class="stat-title">{"Simulated"}</div>
                                        <div class="stat-value text-lg">
                                            {if transaction.is_simulated {
                                                html! { <span class="badge badge-info badge-lg"><i class="fas fa-flask mr-1"></i>{"Yes"}</span> }
                                            } else {
                                                html! { <span class="badge badge-ghost badge-lg">{"No"}</span> }
                                            }}
                                        </div>
                                    </div>
                                </div>

                                {if let Some(scenario_id) = transaction.scenario_id {
                                    html! {
                                        <div>
                                            <div class="stat bg-base-200 rounded-box">
                                                <div class="stat-title">{"Scenario"}</div>
                                                <div class="stat-value text-lg">
                                                    <span class="badge badge-warning badge-lg">
                                                        <i class="fas fa-project-diagram mr-1"></i>
                                                        {scenario_map.get(&scenario_id).map(|name| name.as_str()).unwrap_or("Unknown Scenario")}
                                                    </span>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! { <></> }
                                }}
                            </div>

                            {if !transaction.tags.is_empty() {
                                html! {
                                    <>
                                        <div class="divider"></div>
                                        <div>
                                            <h3 class="text-lg font-semibold mb-2">{"Tags"}</h3>
                                            <div class="flex flex-wrap gap-2">
                                                { for transaction.tags.iter().map(|tag| html! {
                                                    <span class="badge badge-lg badge-primary">{&tag.name}</span>
                                                })}
                                            </div>
                                        </div>
                                    </>
                                }
                            } else {
                                html! { <></> }
                            }}
                        </div>
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
                        <span>{format!("Error loading recurring transaction: {}", e)}</span>
                    </div>
                }
            }
            FetchState::NotStarted => html! { <></> }
        }
    };

    let render_instances = || -> Html {
        match &*instances_state {
            FetchState::Success(instances) if !instances.is_empty() => {
                html! {
                    <div class="card bg-base-100 shadow-xl mt-6">
                        <div class="card-body">
                            <h2 class="card-title text-xl">{"Instances"}</h2>
                            <div>
                                <table class="table table-zebra">
                                    <thead>
                                        <tr>
                                            <th>{"Status"}</th>
                                            <th>{"Due Date"}</th>
                                            <th>{"Expected Amount"}</th>
                                            <th>{"Paid Date"}</th>
                                            <th>{"Paid Amount"}</th>
                                            <th>{"Actions"}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        { for instances.iter().map(|instance| {
                                            let expected_amount = match instance.expected_amount.parse::<f64>() {
                                                Ok(val) => val,
                                                Err(_) => 0.0,
                                            };
                                            let amount_class = if expected_amount >= 0.0 { "text-success" } else { "text-error" };

                                            let status_badge = match instance.status.as_str() {
                                                "Pending" => "badge-warning",
                                                "Paid" => "badge-success",
                                                "Skipped" => "badge-ghost",
                                                _ => "badge-default",
                                            };

                                            let on_edit_click = {
                                                let on_edit_instance = on_edit_instance.clone();
                                                let id = instance.id;
                                                Callback::from(move |e: MouseEvent| {
                                                    e.prevent_default();
                                                    on_edit_instance.emit(id);
                                                })
                                            };

                                            let on_delete_click = {
                                                let id = instance.id;
                                                let instances_refetch = instances_refetch.clone();
                                                let toast_ctx = toast_ctx.clone();
                                                Callback::from(move |e: MouseEvent| {
                                                    e.prevent_default();
                                                    let id = id;
                                                    let instances_refetch = instances_refetch.clone();
                                                    let toast_ctx = toast_ctx.clone();

                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        match delete_recurring_instance(id).await {
                                                            Ok(_) => {
                                                                log::info!("Successfully deleted instance ID: {}", id);
                                                                toast_ctx.show_success("Instance deleted successfully".to_string());
                                                                instances_refetch.emit(());
                                                            }
                                                            Err(e) => {
                                                                log::error!("Failed to delete instance: {}", e);
                                                                toast_ctx.show_error(format!("Failed to delete instance: {}", e));
                                                            }
                                                        }
                                                    });
                                                })
                                            };

                                            let instance_id = instance.id;
                                            let current_status = instance.status.clone();
                                            let status_refetch = instances_refetch.clone();
                                            let status_toast_ctx = toast_ctx.clone();
                                            let instance_paid_date = instance.paid_date.clone();
                                            let instance_paid_amount = instance.paid_amount.clone();
                                            let instance_expected_amount = instance.expected_amount.clone();

                                            html! {
                                                <tr>
                                                    <td>
                                                        <span class={classes!("badge", "badge-sm", status_badge)}>
                                                            {&instance.status}
                                                        </span>
                                                    </td>
                                                    <td>{&instance.due_date}</td>
                                                    <td class={classes!("font-mono", amount_class)}>
                                                        {if expected_amount >= 0.0 {
                                                            format!("+{}", format_currency(&instance.expected_amount))
                                                        } else {
                                                            format!("-{}", format_currency(&instance.expected_amount))
                                                        }}
                                                    </td>
                                                    <td>
                                                        {instance.paid_date.as_ref().unwrap_or(&"-".to_string())}
                                                    </td>
                                                    <td class={classes!("font-mono", amount_class)}>
                                                        {if let Some(paid_amt) = &instance.paid_amount {
                                                            let paid = match paid_amt.parse::<f64>() {
                                                                Ok(v) => v,
                                                                Err(_) => 0.0,
                                                            };
                                                            if paid >= 0.0 {
                                                                format!("+{}", format_currency(paid_amt))
                                                            } else {
                                                                format!("-{}", format_currency(paid_amt))
                                                            }
                                                        } else {
                                                            "-".to_string()
                                                        }}
                                                    </td>
                                                    <td>
                                                        <div class="flex gap-2">
                                                            <div class="dropdown dropdown-end">
                                                                <button
                                                                    tabindex="0"
                                                                    class="btn btn-sm btn-outline btn-square"
                                                                    title="Change Status"
                                                                >
                                                                    <i class="fas fa-sync-alt"></i>
                                                                </button>
                                                                <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-40 z-50">
                                                                    {
                                                                        vec!["Pending", "Paid", "Skipped"].into_iter().map(|status| {
                                                                            let id = instance_id;
                                                                            let is_current = status == current_status.as_str();
                                                                            let on_status_click = {
                                                                                let status = status.to_string();
                                                                                let instances_refetch = status_refetch.clone();
                                                                                let toast_ctx = status_toast_ctx.clone();
                                                                                let paid_date = instance_paid_date.clone();
                                                                                let paid_amount = instance_paid_amount.clone();
                                                                                let expected_amount = instance_expected_amount.clone();
                                                                                Callback::from(move |_: MouseEvent| {
                                                                                    let id = id;
                                                                                    let status = status.clone();
                                                                                    let instances_refetch = instances_refetch.clone();
                                                                                    let toast_ctx = toast_ctx.clone();
                                                                                    let paid_date = paid_date.clone();
                                                                                    let paid_amount = paid_amount.clone();
                                                                                    let expected_amount = expected_amount.clone();

                                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                                        // If changing to Paid status, ensure paid_date and paid_amount are set
                                                                                        let (final_paid_date, final_paid_amount) = if status == "Paid" {
                                                                                            let pd = if paid_date.is_none() {
                                                                                                Some(chrono::Local::now().format("%Y-%m-%d").to_string())
                                                                                            } else {
                                                                                                paid_date
                                                                                            };
                                                                                            let pa = if paid_amount.is_none() {
                                                                                                Some(expected_amount)
                                                                                            } else {
                                                                                                paid_amount
                                                                                            };
                                                                                            (pd, pa)
                                                                                        } else {
                                                                                            (None, None)
                                                                                        };

                                                                                        let request = UpdateRecurringInstanceRequest {
                                                                                            status: Some(status.clone()),
                                                                                            due_date: None,
                                                                                            expected_amount: None,
                                                                                            paid_date: final_paid_date,
                                                                                            paid_amount: final_paid_amount,
                                                                                        };
                                                                                        match update_recurring_instance(id, request).await {
                                                                                            Ok(_) => {
                                                                                                toast_ctx.show_success(format!("Status changed to {}", status));
                                                                                                instances_refetch.emit(());
                                                                                            }
                                                                                            Err(e) => {
                                                                                                toast_ctx.show_error(format!("Failed to update status: {}", e));
                                                                                            }
                                                                                        }
                                                                                    });
                                                                                })
                                                                            };
                                                                            html! {
                                                                                <li>
                                                                                    <a onclick={on_status_click} class={if is_current { "active" } else { "" }}>
                                                                                        {status}
                                                                                        {if is_current {
                                                                                            html! { <i class="fas fa-check ml-2"></i> }
                                                                                        } else {
                                                                                            html! { <></> }
                                                                                        }}
                                                                                    </a>
                                                                                </li>
                                                                            }
                                                                        }).collect::<Html>()
                                                                    }
                                                                </ul>
                                                            </div>
                                                            <button
                                                                class="btn btn-sm btn-ghost btn-square"
                                                                title="Edit"
                                                                onclick={on_edit_click}
                                                            >
                                                                <i class="fas fa-edit"></i>
                                                            </button>
                                                            <button
                                                                class="btn btn-sm btn-error btn-outline btn-square"
                                                                title="Delete"
                                                                onclick={on_delete_click}
                                                            >
                                                                <i class="fas fa-trash"></i>
                                                            </button>
                                                        </div>
                                                    </td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                }
            }
            FetchState::Success(_) => {
                html! {
                    <div class="card bg-base-100 shadow-xl mt-6">
                        <div class="card-body">
                            <h2 class="card-title text-xl">{"Instances"}</h2>
                            <div class="alert alert-info">
                                <i class="fas fa-info-circle"></i>
                                <span>{"No instances created yet for this recurring transaction."}</span>
                            </div>
                        </div>
                    </div>
                }
            }
            FetchState::Loading => {
                html! {
                    <div class="card bg-base-100 shadow-xl mt-6">
                        <div class="card-body">
                            <div class="flex justify-center p-8">
                                <span class="loading loading-spinner loading-lg"></span>
                            </div>
                        </div>
                    </div>
                }
            }
            FetchState::Error(e) => {
                html! {
                    <div class="card bg-base-100 shadow-xl mt-6">
                        <div class="card-body">
                            <div class="alert alert-error">
                                <i class="fas fa-exclamation-circle"></i>
                                <span>{format!("Error loading instances: {}", e)}</span>
                            </div>
                        </div>
                    </div>
                }
            }
            FetchState::NotStarted => html! { <></> }
        }
    };

    html! {
        <Layout title="Recurring Transaction Detail">
            <div class="container mx-auto p-4">
                {render_transaction_details()}
                {render_instances()}
            </div>

            if *show_edit_modal {
                if let Some(instance) = (*edit_instance).clone() {
                    <InstanceEditModal
                        show={*show_edit_modal}
                        instance={instance}
                        on_close={on_close_edit_modal}
                        on_success={on_instance_updated}
                    />
                }
            }
        </Layout>
    }
}

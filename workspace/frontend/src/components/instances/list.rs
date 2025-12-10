use yew::prelude::*;
use yew_router::prelude::*;
use crate::api_client::recurring_transaction::{RecurringInstanceResponse, get_recurring_instances, delete_recurring_instance};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::common::toast::ToastContext;
use crate::hooks::FetchState;
use crate::router::Route;

#[derive(Properties, PartialEq)]
pub struct InstancesListProps {
    #[prop_or_default]
    pub on_edit: Option<Callback<i32>>,
    #[prop_or_default]
    pub recurring_transaction_id: Option<i32>,
}

#[function_component(InstancesList)]
pub fn instances_list(props: &InstancesListProps) -> Html {
    let recurring_id = props.recurring_transaction_id;
    let (fetch_state, refetch) = use_fetch_with_refetch(move || {
        get_recurring_instances(None, None, recurring_id, None)
    });
    let toast_ctx = use_context::<ToastContext>().expect("ToastContext not found");

    let format_currency = |amount: &str| -> String {
        match amount.parse::<f64>() {
            Ok(val) => format!("${:.2}", val.abs()),
            Err(_) => amount.to_string(),
        }
    };

    let render_content = || -> Html {
        match &*fetch_state {
            FetchState::Success(instances) if !instances.is_empty() => {
                html! {
                    <div class="overflow-x-auto bg-base-100 shadow rounded-box">
                        <table class="table table-zebra">
                            <thead>
                                <tr>
                                    <th>{"Recurring Transaction"}</th>
                                    <th>{"Status"}</th>
                                    <th>{"Due Date"}</th>
                                    <th>{"Expected Amount"}</th>
                                    <th>{"Target Account"}</th>
                                    <th>{"Source Account"}</th>
                                    <th>{"Paid Date"}</th>
                                    <th>{"Paid Amount"}</th>
                                    <th>{"Tags"}</th>
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
                                        let on_edit = props.on_edit.clone();
                                        let id = instance.id;
                                        Callback::from(move |e: MouseEvent| {
                                            e.prevent_default();
                                            if let Some(callback) = &on_edit {
                                                callback.emit(id);
                                            }
                                        })
                                    };

                                    let on_delete_click = {
                                        let id = instance.id;
                                        let refetch = refetch.clone();
                                        let toast_ctx = toast_ctx.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.prevent_default();
                                            let id = id;
                                            let refetch = refetch.clone();
                                            let toast_ctx = toast_ctx.clone();

                                            wasm_bindgen_futures::spawn_local(async move {
                                                match delete_recurring_instance(id).await {
                                                    Ok(_) => {
                                                        log::info!("Successfully deleted instance ID: {}", id);
                                                        toast_ctx.show_success("Instance deleted successfully".to_string());
                                                        refetch.emit(());
                                                    }
                                                    Err(e) => {
                                                        log::error!("Failed to delete instance: {}", e);
                                                        toast_ctx.show_error(format!("Failed to delete instance: {}", e));
                                                    }
                                                }
                                            });
                                        })
                                    };

                                    html! {
                                        <tr>
                                            <td>
                                                {if let Some(name) = &instance.recurring_transaction_name {
                                                    html! {
                                                        <Link<Route> to={Route::Recurring} classes="link link-hover link-primary">
                                                            {name}
                                                        </Link<Route>>
                                                    }
                                                } else {
                                                    html! { <span class="text-base-content/50">{"N/A"}</span> }
                                                }}
                                            </td>
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
                                                {if let Some(target_name) = &instance.target_account_name {
                                                    if let Some(target_id) = instance.target_account_id {
                                                        html! {
                                                            <Link<Route> to={Route::AccountEdit { id: target_id }} classes="link link-hover">
                                                                {target_name}
                                                            </Link<Route>>
                                                        }
                                                    } else {
                                                        html! { <span>{target_name}</span> }
                                                    }
                                                } else {
                                                    html! { <span class="text-base-content/50">{"-"}</span> }
                                                }}
                                            </td>
                                            <td>
                                                {if let Some(source_name) = &instance.source_account_name {
                                                    if let Some(source_id) = instance.source_account_id {
                                                        html! {
                                                            <Link<Route> to={Route::AccountEdit { id: source_id }} classes="link link-hover">
                                                                {source_name}
                                                            </Link<Route>>
                                                        }
                                                    } else {
                                                        html! { <span>{source_name}</span> }
                                                    }
                                                } else {
                                                    html! { <span class="text-base-content/50">{"-"}</span> }
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
                                                <div class="flex flex-wrap gap-1">
                                                    { for instance.tags.iter().map(|tag| html! {
                                                        <span class="badge badge-sm badge-ghost">{&tag.name}</span>
                                                    })}
                                                </div>
                                            </td>
                                            <td>
                                                <div class="flex gap-2">
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
                }
            }
            FetchState::Success(_) => {
                html! {
                    <div class="alert alert-info">
                        <i class="fas fa-info-circle"></i>
                        <span>{"No instances found. Create instances from recurring transactions!"}</span>
                    </div>
                }
            }
            FetchState::Error(e) => {
                html! {
                    <div class="alert alert-error">
                        <i class="fas fa-exclamation-circle"></i>
                        <span>{format!("Error loading instances: {}", e)}</span>
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
            FetchState::NotStarted => {
                html! { <></> }
            }
        }
    };

    html! {
        <div>
            {render_content()}
        </div>
    }
}

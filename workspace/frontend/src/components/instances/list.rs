use yew::prelude::*;
use yew_router::prelude::*;
use std::collections::HashMap;
use crate::api_client::recurring_transaction::{RecurringInstanceResponse, get_recurring_instances, delete_recurring_instance, update_recurring_instance, UpdateRecurringInstanceRequest};
use crate::api_client::account::get_accounts;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::common::toast::ToastContext;
use crate::hooks::FetchState;
use crate::router::Route;

#[derive(Clone, Copy, PartialEq)]
enum SortColumn {
    RecurringTransaction,
    Status,
    DueDate,
    ExpectedAmount,
}

#[derive(Clone, Copy, PartialEq)]
enum SortDirection {
    Ascending,
    Descending,
}

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
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let toast_ctx = use_context::<ToastContext>().expect("ToastContext not found");

    let sort_column = use_state(|| SortColumn::DueDate);
    let sort_direction = use_state(|| SortDirection::Descending);
    let selected_target_account = use_state(|| None::<i32>);
    let selected_status = use_state(|| None::<String>);

    // Get accounts list
    let accounts_list = match &*accounts_state {
        FetchState::Success(accounts) => accounts.clone(),
        _ => vec![],
    };

    let format_currency = |amount: &str| -> String {
        match amount.parse::<f64>() {
            Ok(val) => format!("${:.2}", val.abs()),
            Err(_) => amount.to_string(),
        }
    };

    let on_sort = {
        let sort_column = sort_column.clone();
        let sort_direction = sort_direction.clone();
        Callback::from(move |column: SortColumn| {
            if *sort_column == column {
                sort_direction.set(match *sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                });
            } else {
                sort_column.set(column);
                sort_direction.set(SortDirection::Descending);
            }
        })
    };

    let on_target_account_change = {
        let selected_target_account = selected_target_account.clone();
        Callback::from(move |e: Event| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = target.value();
                if value.is_empty() {
                    selected_target_account.set(None);
                } else if let Ok(acc_id) = value.parse::<i32>() {
                    selected_target_account.set(Some(acc_id));
                }
            }
        })
    };

    let on_status_change = {
        let selected_status = selected_status.clone();
        Callback::from(move |e: Event| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = target.value();
                if value.is_empty() {
                    selected_status.set(None);
                } else {
                    selected_status.set(Some(value));
                }
            }
        })
    };

    let render_content = || -> Html {
        match &*fetch_state {
            FetchState::Success(instances) if !instances.is_empty() => {
                // Filter instances
                let filtered_instances: Vec<_> = instances.iter()
                    .filter(|i| {
                        // Filter by target account
                        if let Some(tgt_acc_id) = *selected_target_account {
                            if i.target_account_id != Some(tgt_acc_id) {
                                return false;
                            }
                        }
                        // Filter by status
                        if let Some(status) = &*selected_status {
                            if &i.status != status {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect();

                if filtered_instances.is_empty() {
                    return html! {
                        <div class="text-center py-8">
                            <p class="text-gray-500">{"No instances found."}</p>
                        </div>
                    };
                }

                // Sort instances
                let mut sorted_instances = filtered_instances.clone();
                sorted_instances.sort_by(|a, b| {
                    let cmp = match *sort_column {
                        SortColumn::RecurringTransaction => {
                            let a_name = a.recurring_transaction_name.as_deref().unwrap_or("");
                            let b_name = b.recurring_transaction_name.as_deref().unwrap_or("");
                            a_name.to_lowercase().cmp(&b_name.to_lowercase())
                        },
                        SortColumn::Status => a.status.to_lowercase().cmp(&b.status.to_lowercase()),
                        SortColumn::DueDate => a.due_date.cmp(&b.due_date),
                        SortColumn::ExpectedAmount => {
                            let a_amt = a.expected_amount.parse::<f64>().unwrap_or(0.0);
                            let b_amt = b.expected_amount.parse::<f64>().unwrap_or(0.0);
                            a_amt.partial_cmp(&b_amt).unwrap_or(std::cmp::Ordering::Equal)
                        },
                    };
                    match *sort_direction {
                        SortDirection::Ascending => cmp,
                        SortDirection::Descending => cmp.reverse(),
                    }
                });

                let current_sort_column = *sort_column;
                let current_sort_direction = *sort_direction;

                html! {
                    <div class="overflow-x-auto bg-base-100 shadow rounded-box">
                        <table class="table table-zebra">
                            <thead>
                                <tr>
                                    {render_sortable_header("Recurring Transaction", SortColumn::RecurringTransaction, current_sort_column, current_sort_direction, on_sort.clone())}
                                    {render_sortable_header("Status", SortColumn::Status, current_sort_column, current_sort_direction, on_sort.clone())}
                                    {render_sortable_header("Due Date", SortColumn::DueDate, current_sort_column, current_sort_direction, on_sort.clone())}
                                    {render_sortable_header("Expected Amount", SortColumn::ExpectedAmount, current_sort_column, current_sort_direction, on_sort.clone())}
                                    <th>{"Target Account"}</th>
                                    <th>{"Source Account"}</th>
                                    <th>{"Paid Date"}</th>
                                    <th>{"Paid Amount"}</th>
                                    <th>{"Tags"}</th>
                                    <th>{"Actions"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for sorted_instances.iter().map(|instance| {
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

                                    let instance_id = instance.id;
                                    let current_status = instance.status.clone();
                                    let status_refetch = refetch.clone();
                                    let status_toast_ctx = toast_ctx.clone();
                                    let instance_paid_date = instance.paid_date.clone();
                                    let instance_paid_amount = instance.paid_amount.clone();
                                    let instance_expected_amount = instance.expected_amount.clone();

                                    html! {
                                        <tr>
                                            <td>
                                                {if let Some(name) = &instance.recurring_transaction_name {
                                                    html! {
                                                        <Link<Route> to={Route::RecurringDetail { id: instance.recurring_transaction_id }} classes="link link-hover link-primary">
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
                                                                        let refetch = status_refetch.clone();
                                                                        let toast_ctx = status_toast_ctx.clone();
                                                                        let paid_date = instance_paid_date.clone();
                                                                        let paid_amount = instance_paid_amount.clone();
                                                                        let expected_amount = instance_expected_amount.clone();
                                                                        Callback::from(move |_: MouseEvent| {
                                                                            let id = id;
                                                                            let status = status.clone();
                                                                            let refetch = refetch.clone();
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
                                                                                        refetch.emit(());
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
            <div class="flex gap-4 mb-4">
                <div class="form-control">
                    <label class="label">
                        <span class="label-text">{"Filter by Status"}</span>
                    </label>
                    <select class="select select-bordered select-sm" onchange={on_status_change} value={selected_status.as_ref().cloned().unwrap_or_default()}>
                        <option value="" selected={selected_status.is_none()}>{"All Statuses"}</option>
                        <option value="Pending">{"Pending"}</option>
                        <option value="Paid">{"Paid"}</option>
                        <option value="Skipped">{"Skipped"}</option>
                    </select>
                </div>

                <div class="form-control">
                    <label class="label">
                        <span class="label-text">{"Filter by Target Account"}</span>
                    </label>
                    <select class="select select-bordered select-sm" onchange={on_target_account_change} value={selected_target_account.as_ref().map(|id| id.to_string()).unwrap_or_default()}>
                        <option value="" selected={selected_target_account.is_none()}>{"All Target Accounts"}</option>
                        {for accounts_list.iter().map(|acc| {
                            html! {
                                <option value={acc.id.to_string()}>
                                    {&acc.name}
                                </option>
                            }
                        })}
                    </select>
                </div>
            </div>

            {render_content()}
        </div>
    }
}

fn render_sortable_header(
    label: &str,
    column: SortColumn,
    current_sort_column: SortColumn,
    current_sort_direction: SortDirection,
    on_sort: Callback<SortColumn>,
) -> Html {
    let is_active = current_sort_column == column;
    let icon = if is_active {
        match current_sort_direction {
            SortDirection::Ascending => html! { <i class="fas fa-sort-up ml-1"></i> },
            SortDirection::Descending => html! { <i class="fas fa-sort-down ml-1"></i> },
        }
    } else {
        html! { <i class="fas fa-sort ml-1 opacity-30"></i> }
    };

    let onclick = {
        let column = column;
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_sort.emit(column);
        })
    };

    html! {
        <th class="cursor-pointer hover:bg-base-200 select-none" onclick={onclick}>
            <div class="flex items-center gap-1">
                {label}
                {icon}
            </div>
        </th>
    }
}

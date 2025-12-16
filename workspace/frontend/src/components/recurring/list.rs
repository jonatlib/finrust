use yew::prelude::*;
use yew_router::prelude::*;
use std::collections::HashMap;
use crate::api_client::recurring_transaction::get_recurring_transactions;
use crate::api_client::category::get_categories;
use crate::api_client::account::get_accounts;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use crate::router::Route;

#[derive(Clone, Copy, PartialEq)]
enum SortColumn {
    Name,
    Period,
    Amount,
    StartDate,
}

#[derive(Clone, Copy, PartialEq)]
enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Properties, PartialEq)]
pub struct RecurringListProps {
    #[prop_or_default]
    pub on_edit: Option<Callback<i32>>,
    #[prop_or_default]
    pub on_create_instance: Option<Callback<i32>>,
    #[prop_or_default]
    pub on_quick_create_instance: Option<Callback<i32>>,
    #[prop_or_default]
    pub account_id: Option<i32>,
}

#[function_component(RecurringList)]
pub fn recurring_list(props: &RecurringListProps) -> Html {
    let account_id = props.account_id;
    let (fetch_state, _refetch) = use_fetch_with_refetch(move || {
        get_recurring_transactions(None, Some(1000), account_id, None)
    });

    let (categories_state, _) = use_fetch_with_refetch(get_categories);
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);

    let sort_column = use_state(|| SortColumn::StartDate);
    let sort_direction = use_state(|| SortDirection::Descending);
    let selected_category = use_state(|| None::<i32>);
    let selected_target_account = use_state(|| None::<i32>);

    // Build category ID -> name map
    let category_map: HashMap<i32, String> = match &*categories_state {
        FetchState::Success(categories) => categories
            .iter()
            .map(|cat| (cat.id, cat.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Build account ID -> name map
    let account_map: HashMap<i32, String> = match &*accounts_state {
        FetchState::Success(accounts) => accounts
            .iter()
            .map(|acc| (acc.id, acc.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Get categories list
    let categories_list = match &*categories_state {
        FetchState::Success(categories) => categories.clone(),
        _ => vec![],
    };

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

    let on_category_change = {
        let selected_category = selected_category.clone();
        Callback::from(move |e: Event| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = target.value();
                if value.is_empty() {
                    selected_category.set(None);
                } else if let Ok(cat_id) = value.parse::<i32>() {
                    selected_category.set(Some(cat_id));
                }
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

    let render_content = || -> Html {
        match &*fetch_state {
            FetchState::Success(transactions) if !transactions.is_empty() => {
                // Filter transactions
                let filtered_transactions: Vec<_> = transactions.iter()
                    .filter(|t| {
                        // Filter by category
                        if let Some(cat_id) = *selected_category {
                            if t.category_id != Some(cat_id) {
                                return false;
                            }
                        }
                        // Filter by target account
                        if let Some(tgt_acc_id) = *selected_target_account {
                            if t.target_account_id != tgt_acc_id {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect();

                if filtered_transactions.is_empty() {
                    return html! {
                        <div class="text-center py-8">
                            <p class="text-gray-500">{"No recurring transactions found."}</p>
                        </div>
                    };
                }

                // Sort transactions
                let mut sorted_transactions = filtered_transactions.clone();
                sorted_transactions.sort_by(|a, b| {
                    let cmp = match *sort_column {
                        SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                        SortColumn::Period => a.period.to_lowercase().cmp(&b.period.to_lowercase()),
                        SortColumn::Amount => {
                            let a_amt = a.amount.parse::<f64>().unwrap_or(0.0);
                            let b_amt = b.amount.parse::<f64>().unwrap_or(0.0);
                            a_amt.partial_cmp(&b_amt).unwrap_or(std::cmp::Ordering::Equal)
                        },
                        SortColumn::StartDate => a.start_date.cmp(&b.start_date),
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
                                    {render_sortable_header("Name", SortColumn::Name, current_sort_column, current_sort_direction, on_sort.clone())}
                                    {render_sortable_header("Period", SortColumn::Period, current_sort_column, current_sort_direction, on_sort.clone())}
                                    {render_sortable_header("Amount", SortColumn::Amount, current_sort_column, current_sort_direction, on_sort.clone())}
                                    {render_sortable_header("Start Date", SortColumn::StartDate, current_sort_column, current_sort_direction, on_sort.clone())}
                                    <th>{"End Date"}</th>
                                    <th>{"Category"}</th>
                                    <th>{"Tags"}</th>
                                    <th>{"Actions"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for sorted_transactions.iter().map(|t| {
                                    let amount = match t.amount.parse::<f64>() {
                                        Ok(val) => val,
                                        Err(_) => 0.0,
                                    };
                                    let amount_class = if amount >= 0.0 { "text-success" } else { "text-error" };

                                    let on_edit_click = {
                                        let on_edit = props.on_edit.clone();
                                        let id = t.id;
                                        Callback::from(move |e: MouseEvent| {
                                            e.prevent_default();
                                            if let Some(callback) = &on_edit {
                                                callback.emit(id);
                                            }
                                        })
                                    };

                                    let on_create_instance_click = {
                                        let on_create = props.on_create_instance.clone();
                                        let id = t.id;
                                        Callback::from(move |e: MouseEvent| {
                                            e.prevent_default();
                                            if let Some(callback) = &on_create {
                                                callback.emit(id);
                                            }
                                        })
                                    };

                                    let on_quick_create_click = {
                                        let on_quick = props.on_quick_create_instance.clone();
                                        let id = t.id;
                                        Callback::from(move |e: MouseEvent| {
                                            e.prevent_default();
                                            if let Some(callback) = &on_quick {
                                                callback.emit(id);
                                            }
                                        })
                                    };

                                    html! {
                                        <tr>
                                            <td class="font-bold">
                                                <Link<Route>
                                                    to={Route::RecurringDetail { id: t.id }}
                                                    classes="link link-hover link-primary"
                                                >
                                                    {&t.name}
                                                </Link<Route>>
                                                if let Some(desc) = &t.description {
                                                    <div class="text-xs font-normal opacity-50">
                                                        {desc}
                                                    </div>
                                                }
                                                <div class="flex gap-1 mt-1 flex-wrap">
                                                    {if t.is_simulated {
                                                        html! { <span class="badge badge-xs badge-info">{"simulated"}</span> }
                                                    } else {
                                                        html! {}
                                                    }}
                                                    {if t.scenario_id.is_some() {
                                                        html! { <span class="badge badge-xs badge-warning">{"scenario"}</span> }
                                                    } else {
                                                        html! {}
                                                    }}
                                                </div>
                                            </td>
                                            <td>{&t.period}</td>
                                            <td class={classes!("font-mono", amount_class)}>
                                                {if amount >= 0.0 {
                                                    format!("+{}", format_currency(&t.amount))
                                                } else {
                                                    format!("-{}", format_currency(&t.amount))
                                                }}
                                            </td>
                                            <td>{&t.start_date}</td>
                                            <td>{t.end_date.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td>
                                                {if let Some(category_id) = t.category_id {
                                                    html! {
                                                        <span class="badge badge-sm badge-info badge-outline">
                                                            <i class="fas fa-tag mr-1"></i>
                                                            {category_map.get(&category_id).map(|name| name.as_str()).unwrap_or("Unknown")}
                                                        </span>
                                                    }
                                                } else {
                                                    html! { <span class="text-xs text-gray-500">{"—"}</span> }
                                                }}
                                            </td>
                                            <td>
                                                <div class="flex flex-wrap gap-1">
                                                    { for t.tags.iter().map(|tag| html! {
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
                                                        class="btn btn-sm btn-primary btn-square"
                                                        title="Quick Create Instance (today, default amount)"
                                                        onclick={on_quick_create_click}
                                                    >
                                                        <i class="fas fa-plus"></i>
                                                    </button>
                                                    <button
                                                        class="btn btn-sm btn-success btn-outline gap-2"
                                                        onclick={on_create_instance_click}
                                                    >
                                                        <i class="fas fa-calendar-plus"></i> {"Custom Instance"}
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
                        <span>{"No recurring transactions found. Create one to get started!"}</span>
                    </div>
                }
            }
            FetchState::Error(e) => {
                html! {
                    <div class="alert alert-error">
                        <i class="fas fa-exclamation-circle"></i>
                        <span>{format!("Error loading recurring transactions: {}", e)}</span>
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
                        <span class="label-text">{"Filter by Category"}</span>
                    </label>
                    <select class="select select-bordered select-sm" onchange={on_category_change} value={selected_category.as_ref().map(|id| id.to_string()).unwrap_or_default()}>
                        <option value="" selected={selected_category.is_none()}>{"All Categories"}</option>
                        {for categories_list.iter().map(|cat| {
                            html! {
                                <option value={cat.id.to_string()}>
                                    {&cat.name}
                                </option>
                            }
                        })}
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

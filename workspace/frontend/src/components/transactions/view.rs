use yew::prelude::*;
use yew_router::prelude::*;
use std::collections::HashMap;
use chrono::Datelike;
use crate::api_client::transaction::{get_transactions, TransactionResponse};
use crate::api_client::account::get_accounts;
use crate::api_client::category::get_categories;
use crate::api_client::scenario::get_scenarios;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::components::common::pagination::Pagination;
use crate::hooks::FetchState;
use super::transaction_modal::TransactionModal;
use crate::Route;

#[derive(Clone, Copy, PartialEq)]
enum SortColumn {
    Date,
    Name,
    Account,
    Amount,
}

#[derive(Clone, Copy, PartialEq)]
enum SortDirection {
    Ascending,
    Descending,
}

#[function_component(Transactions)]
pub fn transactions() -> Html {
    log::trace!("Transactions component rendering");
    let current_page = use_state(|| 1u64);
    let items_per_page = 50u64;
    let fetch_state = use_state(|| FetchState::Loading);

    // Fetch data when page changes
    {
        let fetch_state = fetch_state.clone();
        let page = *current_page;
        use_effect_with(page, move |_| {
            let fetch_state = fetch_state.clone();
            fetch_state.set(FetchState::Loading);

            wasm_bindgen_futures::spawn_local(async move {
                match get_transactions(Some(page), Some(items_per_page)).await {
                    Ok(data) => {
                        fetch_state.set(FetchState::Success(data));
                    }
                    Err(err) => {
                        fetch_state.set(FetchState::Error(err));
                    }
                }
            });
            || ()
        });
    }

    // Create a refetch callback for manual refresh (e.g., after creating a transaction)
    let refetch = {
        let fetch_state = fetch_state.clone();
        let current_page = current_page.clone();
        Callback::from(move |_| {
            let fetch_state = fetch_state.clone();
            let page = *current_page;
            fetch_state.set(FetchState::Loading);

            wasm_bindgen_futures::spawn_local(async move {
                match get_transactions(Some(page), Some(items_per_page)).await {
                    Ok(data) => {
                        fetch_state.set(FetchState::Success(data));
                    }
                    Err(err) => {
                        fetch_state.set(FetchState::Error(err));
                    }
                }
            });
        })
    };

    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let (categories_state, _) = use_fetch_with_refetch(get_categories);
    let (scenarios_state, _) = use_fetch_with_refetch(get_scenarios);
    let show_modal = use_state(|| false);
    let sort_column = use_state(|| SortColumn::Date);
    let sort_direction = use_state(|| SortDirection::Descending);
    let selected_month = use_state(|| None::<(i32, u32)>); // (year, month)
    let selected_category = use_state(|| None::<i32>);
    let selected_source_account = use_state(|| None::<i32>);
    let selected_target_account = use_state(|| None::<i32>);

    let on_page_change = {
        let current_page = current_page.clone();
        Callback::from(move |page: u64| {
            current_page.set(page);
        })
    };

    log::debug!("Transactions component state: loading={}, success={}, error={}",
        fetch_state.is_loading(), fetch_state.is_success(), fetch_state.is_error());

    // Build account ID -> name map
    let account_map: HashMap<i32, String> = match &*accounts_state {
        FetchState::Success(accounts) => accounts
            .iter()
            .map(|acc| (acc.id, acc.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Build category ID -> name map
    let category_map: HashMap<i32, String> = match &*categories_state {
        FetchState::Success(categories) => categories
            .iter()
            .map(|cat| (cat.id, cat.name.clone()))
            .collect(),
        _ => HashMap::new(),
    };

    // Get accounts list for the modal
    let accounts_list = match &*accounts_state {
        FetchState::Success(accounts) => accounts.clone(),
        _ => vec![],
    };

    // Get scenarios list for the modal
    let scenarios_list = match &*scenarios_state {
        FetchState::Success(scenarios) => scenarios.clone(),
        _ => vec![],
    };

    let on_open_modal = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| {
            log::info!("Opening Add Transaction modal");
            show_modal.set(true);
        })
    };

    let on_close_modal = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| {
            log::info!("Closing Add Transaction modal");
            show_modal.set(false);
        })
    };

    let on_success = {
        let refetch = refetch.clone();
        Callback::from(move |_| {
            log::info!("Transaction created successfully, refetching transactions");
            refetch.emit(());
        })
    };

    let on_sort = {
        let sort_column = sort_column.clone();
        let sort_direction = sort_direction.clone();
        Callback::from(move |column: SortColumn| {
            if *sort_column == column {
                // Toggle direction if clicking the same column
                sort_direction.set(match *sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                });
            } else {
                // Set new column with default descending direction
                sort_column.set(column);
                sort_direction.set(SortDirection::Descending);
            }
        })
    };

    let on_month_change = {
        let selected_month = selected_month.clone();
        Callback::from(move |e: Event| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                let value = target.value();
                if value.is_empty() {
                    selected_month.set(None);
                } else {
                    // Parse "YYYY-MM" format
                    let parts: Vec<&str> = value.split('-').collect();
                    if parts.len() == 2 {
                        if let (Ok(year), Ok(month)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>()) {
                            selected_month.set(Some((year, month)));
                        }
                    }
                }
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

    let on_source_account_change = {
        let selected_source_account = selected_source_account.clone();
        Callback::from(move |e: Event| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                let value = target.value();
                if value.is_empty() {
                    selected_source_account.set(None);
                } else if let Ok(acc_id) = value.parse::<i32>() {
                    selected_source_account.set(Some(acc_id));
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

    // Get unique months from transactions
    let available_months = match &*fetch_state {
        FetchState::Success(transactions) => {
            let mut months = std::collections::BTreeSet::new();
            for t in transactions {
                months.insert((t.date.year(), t.date.month()));
            }
            months.into_iter().collect::<Vec<_>>()
        },
        _ => vec![],
    };

    // Get categories list
    let categories_list = match &*categories_state {
        FetchState::Success(categories) => categories.clone(),
        _ => vec![],
    };

    html! {
        <>
            <TransactionModal
                show={*show_modal}
                on_close={on_close_modal}
                on_success={on_success}
                accounts={accounts_list.clone()}
                scenarios={scenarios_list.clone()}
                transaction={None}
            />

            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Transactions"}</h2>
                <button
                    class="btn btn-primary btn-sm"
                    onclick={on_open_modal}
                >
                    <i class="fas fa-plus"></i> {" Add Transaction"}
                </button>
            </div>

            <div class="flex gap-4 mb-4">
                <div class="form-control">
                    <label class="label">
                        <span class="label-text">{"Filter by Month"}</span>
                    </label>
                    <input
                        type="month"
                        class="input input-bordered input-sm"
                        onchange={on_month_change}
                        value={selected_month.as_ref().map(|(y, m)| format!("{}-{:02}", y, m)).unwrap_or_default()}
                    />
                </div>

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
                        <span class="label-text">{"Filter by Source Account"}</span>
                    </label>
                    <select class="select select-bordered select-sm" onchange={on_source_account_change} value={selected_source_account.as_ref().map(|id| id.to_string()).unwrap_or_default()}>
                        <option value="" selected={selected_source_account.is_none()}>{"All Source Accounts"}</option>
                        {for accounts_list.iter().map(|acc| {
                            html! {
                                <option value={acc.id.to_string()}>
                                    {&acc.name}
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

            {
                match &*fetch_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-error">
                            <span>{error}</span>
                            <button class="btn btn-sm" onclick={move |_| refetch.emit(())}>
                                {"Retry"}
                            </button>
                        </div>
                    },
                    FetchState::Success(transactions) => {
                        // Filter transactions
                        let filtered_transactions: Vec<_> = transactions.iter()
                            .filter(|t| {
                                // Filter by month
                                if let Some((year, month)) = *selected_month {
                                    let tx_date = t.date;
                                    if tx_date.year() != year || tx_date.month() != month {
                                        return false;
                                    }
                                }
                                // Filter by category
                                if let Some(cat_id) = *selected_category {
                                    if t.category_id != Some(cat_id) {
                                        return false;
                                    }
                                }
                                // Filter by source account
                                if let Some(src_acc_id) = *selected_source_account {
                                    if t.source_account_id != Some(src_acc_id) {
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
                            html! {
                                <div class="text-center py-8">
                                    <p class="text-gray-500">{"No transactions found."}</p>
                                </div>
                            }
                        } else {
                            // Sort transactions
                            let mut sorted_transactions = filtered_transactions.clone();
                            sorted_transactions.sort_by(|a, b| {
                                let cmp = match *sort_column {
                                    SortColumn::Date => a.date.cmp(&b.date),
                                    SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                                    SortColumn::Account => {
                                        let a_name = account_map.get(&a.target_account_id).map(|s| s.as_str()).unwrap_or("");
                                        let b_name = account_map.get(&b.target_account_id).map(|s| s.as_str()).unwrap_or("");
                                        a_name.to_lowercase().cmp(&b_name.to_lowercase())
                                    },
                                    SortColumn::Amount => a.amount.cmp(&b.amount),
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
                                                {render_sortable_header("Date", SortColumn::Date, current_sort_column, current_sort_direction, on_sort.clone())}
                                                {render_sortable_header("Transaction", SortColumn::Name, current_sort_column, current_sort_direction, on_sort.clone())}
                                                <th>{"Source Account"}</th>
                                                <th>{"Target Account"}</th>
                                                {render_sortable_header("Amount", SortColumn::Amount, current_sort_column, current_sort_direction, on_sort.clone())}
                                                <th>{"Category"}</th>
                                                <th>{"Tags"}</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            { for sorted_transactions.iter().map(|transaction| {
                                                render_transaction_row(transaction, &account_map, &category_map)
                                            })}
                                        </tbody>
                                    </table>
                                </div>
                            }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }
            }

            // Add pagination
            {match &*fetch_state {
                FetchState::Success(transactions) if !transactions.is_empty() => {
                    // Estimate total items based on page and items retrieved
                    let items_on_page = transactions.len() as u64;
                    let estimated_total = if items_on_page < items_per_page {
                        // Last page
                        (*current_page - 1) * items_per_page + items_on_page
                    } else {
                        // Assume there might be more pages
                        (*current_page) * items_per_page + 1
                    };

                    html! {
                        <Pagination
                            current_page={*current_page}
                            total_items={estimated_total}
                            items_per_page={items_per_page}
                            on_page_change={on_page_change.clone()}
                        />
                    }
                }
                _ => html! {}
            }}
        </>
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

fn render_transaction_row(transaction: &TransactionResponse, account_map: &HashMap<i32, String>, category_map: &HashMap<i32, String>) -> Html {
    let amount_class = if transaction.amount >= rust_decimal::Decimal::ZERO {
        "text-success"
    } else {
        "text-error"
    };

    let target_account_name = account_map
        .get(&transaction.target_account_id)
        .map(|name| name.as_str())
        .unwrap_or("Unknown Account");

    let source_account_name = transaction.source_account_id
        .and_then(|id| account_map.get(&id))
        .map(|name| name.as_str());

    let transaction_id = transaction.id;
    let format_amount = |amount: rust_decimal::Decimal| -> String {
        if amount >= rust_decimal::Decimal::ZERO {
            format!("+{:.2}", amount)
        } else {
            format!("{:.2}", amount)
        }
    };

    // Check if transaction is in the future (pending) or past/today (accounted)
    let today = chrono::Local::now().date_naive();
    let is_pending = transaction.date > today;
    let status_badge = if is_pending {
        html! {
            <div class="flex gap-1 items-center flex-wrap">
                <div class="text-xs font-normal text-warning">{"Pending"}</div>
                {if transaction.is_simulated {
                    html! { <span class="badge badge-xs badge-info">{"simulated"}</span> }
                } else {
                    html! {}
                }}
                {if transaction.scenario_id.is_some() {
                    html! { <span class="badge badge-xs badge-warning">{"scenario"}</span> }
                } else {
                    html! {}
                }}
            </div>
        }
    } else {
        html! {
            <div class="flex gap-1 items-center flex-wrap">
                <div class="text-xs font-normal text-success">{"Accounted"}</div>
                {if transaction.is_simulated {
                    html! { <span class="badge badge-xs badge-info">{"simulated"}</span> }
                } else {
                    html! {}
                }}
                {if transaction.scenario_id.is_some() {
                    html! { <span class="badge badge-xs badge-warning">{"scenario"}</span> }
                } else {
                    html! {}
                }}
            </div>
        }
    };

    html! {
        <tr key={transaction.id} class="cursor-pointer">
            <Link<Route> to={Route::TransactionEdit { id: transaction_id }} classes="contents">
                <td>
                    <div class="font-semibold">{transaction.date.format("%Y-%m-%d").to_string()}</div>
                    {status_badge}
                </td>
                <td>
                    <div class="font-bold">{&transaction.name}</div>
                    {if let Some(desc) = &transaction.description {
                        if !desc.is_empty() {
                            html! {
                                <div class="text-xs font-normal opacity-50">{desc}</div>
                            }
                        } else {
                            html! {}
                        }
                    } else {
                        html! {}
                    }}
                </td>
                <td>
                    {if let Some(source) = source_account_name {
                        html! {
                            <span class="badge badge-sm badge-ghost">{source}</span>
                        }
                    } else {
                        html! { <span class="text-xs text-gray-500">{"—"}</span> }
                    }}
                </td>
                <td>
                    <span class="badge badge-sm badge-ghost">{target_account_name}</span>
                </td>
                <td class={classes!("font-mono", "font-semibold", amount_class)}>
                    {format_amount(transaction.amount)}
                </td>
                <td>
                    {if let Some(category_id) = transaction.category_id {
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
                    <div class="flex gap-1 flex-wrap">
                        { for transaction.tags.iter().map(|tag| {
                            html! {
                                <span class="badge badge-sm badge-outline">{&tag.name}</span>
                            }
                        })}
                    </div>
                </td>
            </Link<Route>>
        </tr>
    }
}

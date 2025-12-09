use yew::prelude::*;
use yew_router::prelude::*;
use std::collections::HashMap;
use crate::api_client::transaction::{get_transactions, TransactionResponse};
use crate::api_client::account::get_accounts;
use crate::common::fetch_hook::use_fetch_with_refetch;
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
    let (fetch_state, refetch) = use_fetch_with_refetch(get_transactions);
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let show_modal = use_state(|| false);
    let sort_column = use_state(|| SortColumn::Date);
    let sort_direction = use_state(|| SortDirection::Descending);

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

    // Get accounts list for the modal
    let accounts_list = match &*accounts_state {
        FetchState::Success(accounts) => accounts.clone(),
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

    html! {
        <>
            <TransactionModal
                show={*show_modal}
                on_close={on_close_modal}
                on_success={on_success}
                accounts={accounts_list}
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
                        if transactions.is_empty() {
                            html! {
                                <div class="text-center py-8">
                                    <p class="text-gray-500">{"No transactions found."}</p>
                                </div>
                            }
                        } else {
                            // Sort transactions
                            let mut sorted_transactions = transactions.clone();
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
                                                {render_sortable_header("Account", SortColumn::Account, current_sort_column, current_sort_direction, on_sort.clone())}
                                                {render_sortable_header("Amount", SortColumn::Amount, current_sort_column, current_sort_direction, on_sort.clone())}
                                                <th>{"Tags"}</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            { for sorted_transactions.iter().map(|transaction| {
                                                render_transaction_row(transaction, &account_map)
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

fn render_transaction_row(transaction: &TransactionResponse, account_map: &HashMap<i32, String>) -> Html {
    let amount_class = if transaction.amount >= rust_decimal::Decimal::ZERO {
        "text-success"
    } else {
        "text-error"
    };

    let account_name = account_map
        .get(&transaction.target_account_id)
        .map(|name| name.as_str())
        .unwrap_or("Unknown Account");

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
            <div class="text-xs font-normal text-warning">{"Pending"}</div>
        }
    } else {
        html! {
            <div class="text-xs font-normal text-success">{"Accounted"}</div>
        }
    };

    html! {
        <tr key={transaction.id} class="hover:bg-primary hover:bg-opacity-10 cursor-pointer transition-colors duration-150">
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
                    <span class="badge badge-sm badge-ghost">{account_name}</span>
                </td>
                <td class={classes!("font-mono", "font-semibold", amount_class)}>
                    {format_amount(transaction.amount)}
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

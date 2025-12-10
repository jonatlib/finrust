use yew::prelude::*;
use crate::api_client::recurring_transaction::get_recurring_transactions;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

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
        get_recurring_transactions(None, None, account_id, None)
    });

    let format_currency = |amount: &str| -> String {
        match amount.parse::<f64>() {
            Ok(val) => format!("${:.2}", val.abs()),
            Err(_) => amount.to_string(),
        }
    };

    let render_content = || -> Html {
        match &*fetch_state {
            FetchState::Success(transactions) if !transactions.is_empty() => {
                html! {
                    <div class="overflow-x-auto bg-base-100 shadow rounded-box">
                        <table class="table table-zebra">
                            <thead>
                                <tr>
                                    <th>{"Name"}</th>
                                    <th>{"Period"}</th>
                                    <th>{"Amount"}</th>
                                    <th>{"Start Date"}</th>
                                    <th>{"End Date"}</th>
                                    <th>{"Tags"}</th>
                                    <th>{"Actions"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for transactions.iter().map(|t| {
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
                                                {&t.name}
                                                if let Some(desc) = &t.description {
                                                    <div class="text-xs font-normal opacity-50">
                                                        {desc}
                                                    </div>
                                                }
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
            {render_content()}
        </div>
    }
}

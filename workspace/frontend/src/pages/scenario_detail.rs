use yew::prelude::*;
use yew_router::prelude::*;
use crate::components::layout::layout::Layout;
use crate::components::transactions::TransactionModal;
use crate::components::recurring::RecurringModal;
use crate::components::scenarios::ScenarioModal;
use crate::router::Route;
use crate::api_client::scenario::{get_scenario, delete_scenario, apply_scenario};
use crate::api_client::account::{get_accounts_with_ignored, AccountResponse};
use crate::api_client::transaction::{get_transactions, TransactionResponse};
use crate::api_client::recurring_transaction::{get_recurring_transactions, RecurringTransactionResponse};
use crate::api_client::timeseries::get_all_accounts_timeseries_with_scenario;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::common::toast::ToastContext;
use crate::hooks::FetchState;
use chrono::{Local, NaiveDate};
use plotly::{Plot, Scatter, Layout as PlotlyLayout};
use plotly::common::Mode;
use web_sys::HtmlElement;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Plotly)]
    fn newPlot(div_id: &str, data: JsValue, layout: JsValue);
}

#[derive(Properties, PartialEq)]
pub struct ScenarioDetailPageProps {
    pub id: i32,
}

#[function_component(ScenarioDetailPage)]
pub fn scenario_detail_page(props: &ScenarioDetailPageProps) -> Html {
    let id = props.id;
    let navigator = use_navigator().unwrap();
    let toast_ctx = use_context::<ToastContext>().expect("ToastContext not found");

    let (scenario_state, scenario_refetch) = use_fetch_with_refetch(move || get_scenario(id));
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));
    let (transactions_state, transactions_refetch) = use_fetch_with_refetch(|| get_transactions(None, None));
    let (recurring_state, recurring_refetch) = use_fetch_with_refetch(|| get_recurring_transactions(None, Some(1000), None, None));

    let show_edit_modal = use_state(|| false);
    let show_apply_modal = use_state(|| false);
    let show_transaction_modal = use_state(|| false);
    let show_recurring_modal = use_state(|| false);
    let edit_transaction = use_state(|| None::<TransactionResponse>);
    let edit_recurring = use_state(|| None::<RecurringTransactionResponse>);
    let is_applying = use_state(|| false);

    let scenario_transactions: Vec<TransactionResponse> = if let FetchState::Success(txs) = &*transactions_state {
        txs.iter().filter(|t| t.scenario_id == Some(id)).cloned().collect()
    } else {
        vec![]
    };

    let scenario_recurring: Vec<RecurringTransactionResponse> = if let FetchState::Success(recurring) = &*recurring_state {
        recurring.iter().filter(|r| r.scenario_id == Some(id)).cloned().collect()
    } else {
        vec![]
    };

    let on_edit_click = {
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| show_edit_modal.set(true))
    };

    let on_close_edit_modal = {
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| show_edit_modal.set(false))
    };

    let on_edit_success = {
        let scenario_refetch = scenario_refetch.clone();
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| {
            scenario_refetch.emit(());
            show_edit_modal.set(false);
        })
    };

    let on_delete_click = {
        let toast_ctx = toast_ctx.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            let toast_ctx = toast_ctx.clone();
            let navigator = navigator.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match delete_scenario(id).await {
                    Ok(_) => {
                        toast_ctx.show_success("Scenario deleted successfully".to_string());
                        navigator.push(&Route::Scenarios);
                    }
                    Err(e) => toast_ctx.show_error(format!("Failed to delete scenario: {}", e)),
                }
            });
        })
    };

    let on_apply_click = {
        let show_apply_modal = show_apply_modal.clone();
        Callback::from(move |_| show_apply_modal.set(true))
    };

    let on_close_apply_modal = {
        let show_apply_modal = show_apply_modal.clone();
        Callback::from(move |_| show_apply_modal.set(false))
    };

    let on_confirm_apply = {
        let is_applying = is_applying.clone();
        let show_apply_modal = show_apply_modal.clone();
        let toast_ctx = toast_ctx.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if *is_applying { return; }
            let is_applying = is_applying.clone();
            let show_apply_modal = show_apply_modal.clone();
            let toast_ctx = toast_ctx.clone();
            let navigator = navigator.clone();
            is_applying.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match apply_scenario(id).await {
                    Ok(message) => {
                        toast_ctx.show_success(format!("Scenario applied: {}", message));
                        show_apply_modal.set(false);
                        is_applying.set(false);
                        navigator.push(&Route::Scenarios);
                    }
                    Err(e) => {
                        toast_ctx.show_error(format!("Failed to apply scenario: {}", e));
                        is_applying.set(false);
                    }
                }
            });
        })
    };

    let on_add_transaction = {
        let show_transaction_modal = show_transaction_modal.clone();
        Callback::from(move |_| show_transaction_modal.set(true))
    };

    let on_close_transaction_modal = {
        let show_transaction_modal = show_transaction_modal.clone();
        let edit_transaction = edit_transaction.clone();
        Callback::from(move |_| {
            show_transaction_modal.set(false);
            edit_transaction.set(None);
        })
    };

    let on_transaction_success = {
        let transactions_refetch = transactions_refetch.clone();
        let show_transaction_modal = show_transaction_modal.clone();
        let edit_transaction = edit_transaction.clone();
        Callback::from(move |_| {
            transactions_refetch.emit(());
            show_transaction_modal.set(false);
            edit_transaction.set(None);
        })
    };

    let on_edit_transaction = {
        let show_transaction_modal = show_transaction_modal.clone();
        let edit_transaction = edit_transaction.clone();
        Callback::from(move |tx: TransactionResponse| {
            edit_transaction.set(Some(tx));
            show_transaction_modal.set(true);
        })
    };

    let on_add_recurring = {
        let show_recurring_modal = show_recurring_modal.clone();
        Callback::from(move |_| show_recurring_modal.set(true))
    };

    let on_close_recurring_modal = {
        let show_recurring_modal = show_recurring_modal.clone();
        let edit_recurring = edit_recurring.clone();
        Callback::from(move |_| {
            show_recurring_modal.set(false);
            edit_recurring.set(None);
        })
    };

    let on_recurring_success = {
        let recurring_refetch = recurring_refetch.clone();
        let show_recurring_modal = show_recurring_modal.clone();
        let edit_recurring = edit_recurring.clone();
        Callback::from(move |_| {
            recurring_refetch.emit(());
            show_recurring_modal.set(false);
            edit_recurring.set(None);
        })
    };

    let on_edit_recurring = {
        let show_recurring_modal = show_recurring_modal.clone();
        let edit_recurring = edit_recurring.clone();
        Callback::from(move |rec: RecurringTransactionResponse| {
            edit_recurring.set(Some(rec));
            show_recurring_modal.set(true);
        })
    };

    let render_scenario_details = || -> Html {
        match &*scenario_state {
            FetchState::Success(scenario) => {
                let status_badge = if scenario.is_active { "badge badge-success" } else { "badge badge-ghost" };
                let status_text = if scenario.is_active { "Active" } else { "Inactive" };

                html! {
                    <>
                        <div class="card bg-base-100 shadow-xl mb-6">
                            <div class="card-body">
                                <div class="flex justify-between items-start">
                                    <div>
                                        <h2 class="card-title text-2xl">{&scenario.name}</h2>
                                        <span class={status_badge}>{status_text}</span>
                                    </div>
                                    <div class="flex gap-2">
                                        <button class="btn btn-sm btn-ghost" onclick={on_edit_click}>
                                            <i class="fas fa-edit"></i>{" Edit"}
                                        </button>
                                        <button class="btn btn-sm btn-error btn-outline" onclick={on_delete_click}>
                                            <i class="fas fa-trash"></i>{" Delete"}
                                        </button>
                                        <button class="btn btn-sm btn-warning" onclick={on_apply_click}>
                                            <i class="fas fa-check"></i>{" Apply Scenario"}
                                        </button>
                                    </div>
                                </div>
                                {if let Some(desc) = &scenario.description {
                                    html! { <p class="text-base-content/70 mt-4">{desc}</p> }
                                } else { html! { <></> }}}
                                <div class="divider"></div>
                                <div class="text-sm text-base-content/60">
                                    {"Created: "}{scenario.created_at.format("%Y-%m-%d %H:%M").to_string()}
                                </div>
                            </div>
                        </div>

                        <div class="card bg-base-100 shadow-xl mb-6">
                            <div class="card-body">
                                <div class="flex justify-between items-center mb-4">
                                    <h3 class="text-xl font-bold">{"Scenario Transactions"}</h3>
                                    <div class="flex gap-2">
                                        <button class="btn btn-primary btn-sm" onclick={on_add_transaction}>
                                            <i class="fas fa-plus"></i>{" Add One-off"}
                                        </button>
                                        <button class="btn btn-primary btn-sm btn-outline" onclick={on_add_recurring}>
                                            <i class="fas fa-sync-alt"></i>{" Add Recurring"}
                                        </button>
                                    </div>
                                </div>
                                {if scenario_transactions.is_empty() && scenario_recurring.is_empty() {
                                    html! {
                                        <div class="text-center py-8 text-base-content/50">
                                            <i class="fas fa-file-invoice text-4xl mb-4 opacity-50"></i>
                                            <p>{"No transactions in this scenario yet."}</p>
                                            <p class="text-sm">{"Add simulated transactions to see their impact."}</p>
                                        </div>
                                    }
                                } else {
                                    html! {
                                        <div class="overflow-x-auto">
                                            <table class="table table-zebra w-full">
                                                <thead>
                                                    <tr>
                                                        <th>{"Date/Period"}</th>
                                                        <th>{"Name"}</th>
                                                        <th>{"Amount"}</th>
                                                        <th>{"Account"}</th>
                                                        <th>{"Type"}</th>
                                                        <th>{"Status"}</th>
                                                        <th>{"Actions"}</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    // Show one-off transactions
                                                    {for scenario_transactions.iter().map(|tx| {
                                                        let account_name = if let FetchState::Success(accounts) = &*accounts_state {
                                                            accounts.iter().find(|a| a.id == tx.target_account_id)
                                                                .map(|a| a.name.clone()).unwrap_or_else(|| "Unknown".to_string())
                                                        } else { "Loading...".to_string() };
                                                        let tx_clone = tx.clone();
                                                        let on_edit = on_edit_transaction.clone();
                                                        html! {
                                                            <tr key={tx.id}>
                                                                <td>{tx.date.format("%Y-%m-%d").to_string()}</td>
                                                                <td>{&tx.name}</td>
                                                                <td class={if tx.amount.is_sign_negative() { "text-error" } else { "text-success" }}>
                                                                    {format!("{:.2}", tx.amount)}
                                                                </td>
                                                                <td>{account_name}</td>
                                                                <td><span class="badge badge-sm">{"One-off"}</span></td>
                                                                <td>
                                                                    {if tx.is_simulated {
                                                                        html! { <span class="badge badge-info">{"Simulated"}</span> }
                                                                    } else {
                                                                        html! { <span class="badge badge-success">{"Real"}</span> }
                                                                    }}
                                                                </td>
                                                                <td>
                                                                    <button
                                                                        class="btn btn-sm btn-ghost"
                                                                        onclick={Callback::from(move |_| on_edit.emit(tx_clone.clone()))}
                                                                    >
                                                                        {"Edit"}
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    })}
                                                    // Show recurring transactions
                                                    {for scenario_recurring.iter().map(|rec| {
                                                        let account_name = if let FetchState::Success(accounts) = &*accounts_state {
                                                            accounts.iter().find(|a| a.id == rec.target_account_id)
                                                                .map(|a| a.name.clone()).unwrap_or_else(|| "Unknown".to_string())
                                                        } else { "Loading...".to_string() };
                                                        let amount = rec.amount.parse::<f64>().unwrap_or(0.0);
                                                        let rec_clone = rec.clone();
                                                        let on_edit = on_edit_recurring.clone();
                                                        html! {
                                                            <tr key={format!("rec-{}", rec.id)}>
                                                                <td>
                                                                    <div>{format!("Starts: {}", rec.start_date)}</div>
                                                                    <div class="text-xs opacity-70">{&rec.period}</div>
                                                                </td>
                                                                <td>{&rec.name}</td>
                                                                <td class={if amount < 0.0 { "text-error" } else { "text-success" }}>
                                                                    {format!("{:.2}", amount)}
                                                                </td>
                                                                <td>{account_name}</td>
                                                                <td><span class="badge badge-sm badge-primary">{"Recurring"}</span></td>
                                                                <td>
                                                                    {if rec.is_simulated {
                                                                        html! { <span class="badge badge-info">{"Simulated"}</span> }
                                                                    } else {
                                                                        html! { <span class="badge badge-success">{"Real"}</span> }
                                                                    }}
                                                                </td>
                                                                <td>
                                                                    <button
                                                                        class="btn btn-sm btn-ghost"
                                                                        onclick={Callback::from(move |_| on_edit.emit(rec_clone.clone()))}
                                                                    >
                                                                        {"Edit"}
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    })}
                                                </tbody>
                                            </table>
                                        </div>
                                    }
                                }}
                            </div>
                        </div>

                        {if let FetchState::Success(accounts) = &*accounts_state {
                            html! {
                                <div class="space-y-6">
                                    <h3 class="text-xl font-bold">{"Financial Forecasts (With This Scenario)"}</h3>
                                    <p class="text-sm text-base-content/60">
                                        {"These charts show how your finances would look if this scenario's transactions were real."}
                                    </p>
                                    <AggregatedCharts accounts={accounts.clone()} scenario_id={id} />
                                </div>
                            }
                        } else { html! { <></> }}}
                    </>
                }
            }
            FetchState::Loading => html! {
                <div class="flex justify-center p-8">
                    <span class="loading loading-spinner loading-lg"></span>
                </div>
            },
            FetchState::Error(e) => html! {
                <div class="alert alert-error">
                    <i class="fas fa-exclamation-circle"></i>
                    <span>{format!("Error loading scenario: {}", e)}</span>
                </div>
            },
            FetchState::NotStarted => html! { <></> }
        }
    };

    let accounts_list = match &*accounts_state {
        FetchState::Success(accounts) => accounts.clone(),
        _ => vec![],
    };

    html! {
        <Layout title="Scenario Detail">
            <div class="container mx-auto p-4">
                {render_scenario_details()}
            </div>

            {if *show_edit_modal {
                if let FetchState::Success(scenario) = &*scenario_state {
                    html! {
                        <ScenarioModal
                            show={*show_edit_modal}
                            on_close={on_close_edit_modal}
                            on_success={on_edit_success}
                            scenario={Some(scenario.clone())}
                        />
                    }
                } else { html! { <></> }}
            } else { html! { <></> }}}

            {if *show_transaction_modal {
                html! {
                    <TransactionModal
                        show={*show_transaction_modal}
                        on_close={on_close_transaction_modal}
                        on_success={on_transaction_success}
                        accounts={accounts_list.clone()}
                        scenarios={vec![]}
                        transaction={(*edit_transaction).clone()}
                        scenario_id={Some(id)}
                    />
                }
            } else { html! { <></> }}}

            {if *show_recurring_modal {
                html! {
                    <RecurringModal
                        show={*show_recurring_modal}
                        on_close={on_close_recurring_modal}
                        on_success={on_recurring_success}
                        accounts={accounts_list}
                        transaction={(*edit_recurring).clone()}
                        scenario_id={Some(id)}
                    />
                }
            } else { html! { <></> }}}

            <dialog class={classes!("modal", (*show_apply_modal).then_some("modal-open"))} id="apply_scenario_modal">
                <div class="modal-box">
                    <h3 class="font-bold text-lg text-warning">
                        <i class="fas fa-exclamation-triangle mr-2"></i>
                        {"Apply Scenario - Confirmation Required"}
                    </h3>
                    <div class="py-4">
                        <div class="alert alert-warning mb-4">
                            <i class="fas fa-exclamation-circle"></i>
                            <span>{"This action cannot be undone!"}</span>
                        </div>
                        <p class="mb-4">
                            {"Applying this scenario will permanently convert all simulated transactions to real transactions."}
                        </p>
                        <p class="font-semibold">{"Are you sure you want to proceed?"}</p>
                    </div>
                    <div class="modal-action">
                        <button type="button" class="btn" onclick={on_close_apply_modal.clone()} disabled={*is_applying}>
                            {"Cancel"}
                        </button>
                        <button type="button" class="btn btn-warning" onclick={on_confirm_apply} disabled={*is_applying}>
                            {if *is_applying {
                                html! { <><span class="loading loading-spinner loading-sm"></span>{" Applying..."}</> }
                            } else {
                                html! { <><i class="fas fa-check mr-2"></i>{"Yes, Apply Scenario"}</> }
                            }}
                        </button>
                    </div>
                </div>
                <form class="modal-backdrop" method="dialog">
                    <button onclick={on_close_apply_modal} disabled={*is_applying}>{"close"}</button>
                </form>
            </dialog>
        </Layout>
    }
}

#[derive(Properties, PartialEq)]
struct AggregatedChartsProps {
    accounts: Vec<AccountResponse>,
    scenario_id: i32,
}

#[function_component(AggregatedCharts)]
fn aggregated_charts(props: &AggregatedChartsProps) -> Html {
    let scenario_id = props.scenario_id;
    let start_date = Local::now().date_naive();
    let end_date = start_date + chrono::Duration::days(13 * 30);

    let (timeseries_state, _) = use_fetch_with_refetch(move || {
        get_all_accounts_timeseries_with_scenario(start_date, end_date, Some(scenario_id))
    });

    html! {
        <div class="space-y-6">
            {match &*timeseries_state {
                FetchState::Loading => html! {
                    <div class="flex justify-center items-center py-8">
                        <span class="loading loading-spinner loading-lg"></span>
                    </div>
                },
                FetchState::Error(error) => html! {
                    <div class="alert alert-error"><span>{error}</span></div>
                },
                FetchState::Success(timeseries) => {
                    if timeseries.data_points.is_empty() {
                        html! {
                            <div class="text-center py-8 text-base-content/50">
                                <i class="fas fa-chart-line text-4xl mb-4 opacity-50"></i>
                                <p>{"No forecast data available."}</p>
                            </div>
                        }
                    } else {
                        html! {
                            <>
                                <NetWorthChart timeseries={timeseries.clone()} scenario_id={scenario_id} />
                                <StackedAreaChart timeseries={timeseries.clone()} accounts={props.accounts.clone()} scenario_id={scenario_id} />
                            </>
                        }
                    }
                },
                FetchState::NotStarted => html! { <></> },
            }}
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct NetWorthChartProps {
    timeseries: crate::api_client::timeseries::AccountStateTimeseries,
    scenario_id: i32,
}

#[function_component(NetWorthChart)]
fn net_worth_chart(props: &NetWorthChartProps) -> Html {
    let container_ref = use_node_ref();
    let timeseries = props.timeseries.clone();
    let div_id = format!("net-worth-chart-{}", props.scenario_id);

    use_effect_with((container_ref.clone(), timeseries.clone(), div_id.clone()), 
        move |(container_ref, timeseries, div_id)| {
        if let Some(element) = container_ref.cast::<HtmlElement>() {
            element.set_id(div_id);

            let mut net_worth_by_date: HashMap<NaiveDate, f64> = HashMap::new();
            for point in &timeseries.data_points {
                let balance = point.balance.to_string().parse::<f64>().unwrap_or(0.0);
                *net_worth_by_date.entry(point.date).or_insert(0.0) += balance;
            }

            let mut dates: Vec<NaiveDate> = net_worth_by_date.keys().cloned().collect();
            dates.sort();

            let dates_str: Vec<String> = dates.iter().map(|d| d.to_string()).collect();
            let balances: Vec<f64> = dates.iter().map(|d| net_worth_by_date[d]).collect();

            let trace = Scatter::new(dates_str, balances)
                .mode(Mode::LinesMarkers)
                .name("Net Worth");

            let layout = PlotlyLayout::new()
                .title("Net Worth Forecast")
                .x_axis(plotly::layout::Axis::new().title("Date"))
                .y_axis(plotly::layout::Axis::new().title("Net Worth"))
                .height(400);

            let mut plot = Plot::new();
            plot.add_trace(trace);
            plot.set_layout(layout);

            let data_js = serde_wasm_bindgen::to_value(&plot.data()).unwrap();
            let layout_js = serde_wasm_bindgen::to_value(&plot.layout()).unwrap();

            newPlot(div_id, data_js, layout_js);
        }
        || ()
    });

    html! {
        <div class="card bg-base-100 shadow">
            <div class="card-body">
                <h4 class="card-title">{"Net Worth Forecast"}</h4>
                <div ref={container_ref} style="width: 100%; height: 400px;"></div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct StackedAreaChartProps {
    timeseries: crate::api_client::timeseries::AccountStateTimeseries,
    accounts: Vec<AccountResponse>,
    scenario_id: i32,
}

#[function_component(StackedAreaChart)]
fn stacked_area_chart(props: &StackedAreaChartProps) -> Html {
    let container_ref = use_node_ref();
    let timeseries = props.timeseries.clone();
    let accounts = props.accounts.clone();
    let div_id = format!("stacked-area-chart-{}", props.scenario_id);

    use_effect_with((container_ref.clone(), timeseries.clone(), accounts.clone(), div_id.clone()),
        move |(container_ref, timeseries, accounts, div_id)| {
        if let Some(element) = container_ref.cast::<HtmlElement>() {
            element.set_id(div_id);

            let mut account_data: HashMap<i32, Vec<(NaiveDate, f64)>> = HashMap::new();
            for point in &timeseries.data_points {
                let balance = point.balance.to_string().parse::<f64>().unwrap_or(0.0);
                account_data.entry(point.account_id).or_insert_with(Vec::new).push((point.date, balance));
            }

            let mut plot = Plot::new();

            for account in accounts.iter() {
                if let Some(data) = account_data.get(&account.id) {
                    let mut sorted_data = data.clone();
                    sorted_data.sort_by_key(|(date, _)| *date);

                    let dates: Vec<String> = sorted_data.iter().map(|(d, _)| d.to_string()).collect();
                    let balances: Vec<f64> = sorted_data.iter().map(|(_, b)| *b).collect();

                    let trace = Scatter::new(dates, balances)
                        .name(&account.name)
                        .mode(Mode::Lines)
                        .stack_group("one");

                    plot.add_trace(trace);
                }
            }

            let layout = PlotlyLayout::new()
                .title("Asset Allocation")
                .x_axis(plotly::layout::Axis::new().title("Date"))
                .y_axis(plotly::layout::Axis::new().title("Balance"))
                .height(500);

            plot.set_layout(layout);

            let data_js = serde_wasm_bindgen::to_value(&plot.data()).unwrap();
            let layout_js = serde_wasm_bindgen::to_value(&plot.layout()).unwrap();

            newPlot(div_id, data_js, layout_js);
        }
        || ()
    });

    html! {
        <div class="card bg-base-100 shadow">
            <div class="card-body">
                <h4 class="card-title">{"Asset Allocation (Stacked)"}</h4>
                <div ref={container_ref} style="width: 100%; height: 500px;"></div>
            </div>
        </div>
    }
}

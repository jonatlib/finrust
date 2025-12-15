use yew::prelude::*;
use yew_router::prelude::*;
use crate::components::layout::layout::Layout;
use crate::router::Route;
use crate::api_client::scenario::{get_scenario, delete_scenario, apply_scenario, Scenario};
use crate::api_client::account::{get_accounts_with_ignored, AccountResponse};
use crate::api_client::timeseries::get_account_timeseries_with_ignored;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::common::toast::ToastContext;
use crate::hooks::FetchState;
use crate::components::scenarios::ScenarioModal;
use chrono::Local;
use plotly::{Plot, Scatter, Layout as PlotlyLayout};
use plotly::common::Mode;
use web_sys::HtmlElement;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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

    // Fetch scenario details
    let (scenario_state, scenario_refetch) = use_fetch_with_refetch(move || {
        get_scenario(id)
    });

    // Fetch all accounts for the chart
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));

    let show_edit_modal = use_state(|| false);
    let show_apply_modal = use_state(|| false);
    let is_applying = use_state(|| false);

    let on_edit_click = {
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| {
            show_edit_modal.set(true);
        })
    };

    let on_close_edit_modal = {
        let show_edit_modal = show_edit_modal.clone();
        Callback::from(move |_| {
            show_edit_modal.set(false);
        })
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
            let id = id;
            let toast_ctx = toast_ctx.clone();
            let navigator = navigator.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match delete_scenario(id).await {
                    Ok(_) => {
                        toast_ctx.show_success("Scenario deleted successfully".to_string());
                        navigator.push(&Route::Scenarios);
                    }
                    Err(e) => {
                        toast_ctx.show_error(format!("Failed to delete scenario: {}", e));
                    }
                }
            });
        })
    };

    let on_apply_click = {
        let show_apply_modal = show_apply_modal.clone();
        Callback::from(move |_| {
            show_apply_modal.set(true);
        })
    };

    let on_close_apply_modal = {
        let show_apply_modal = show_apply_modal.clone();
        Callback::from(move |_| {
            show_apply_modal.set(false);
        })
    };

    let on_confirm_apply = {
        let is_applying = is_applying.clone();
        let show_apply_modal = show_apply_modal.clone();
        let toast_ctx = toast_ctx.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if *is_applying {
                return;
            }

            let id = id;
            let is_applying = is_applying.clone();
            let show_apply_modal = show_apply_modal.clone();
            let toast_ctx = toast_ctx.clone();
            let navigator = navigator.clone();

            is_applying.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match apply_scenario(id).await {
                    Ok(message) => {
                        toast_ctx.show_success(format!("Scenario applied successfully: {}", message));
                        show_apply_modal.set(false);
                        is_applying.set(false);
                        // Navigate back to scenarios list
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

    let render_scenario_details = || -> Html {
        match &*scenario_state {
            FetchState::Success(scenario) => {
                let status_badge = if scenario.is_active {
                    "badge badge-success"
                } else {
                    "badge badge-ghost"
                };

                let status_text = if scenario.is_active { "Active" } else { "Inactive" };

                html! {
                    <>
                        <div class="card bg-base-100 shadow-xl">
                            <div class="card-body">
                                <div class="flex justify-between items-start">
                                    <div>
                                        <h2 class="card-title text-2xl">{&scenario.name}</h2>
                                        <span class={status_badge}>{status_text}</span>
                                    </div>
                                    <div class="flex gap-2">
                                        <button
                                            class="btn btn-sm btn-ghost"
                                            onclick={on_edit_click}
                                        >
                                            <i class="fas fa-edit"></i>
                                            {" Edit"}
                                        </button>
                                        <button
                                            class="btn btn-sm btn-error btn-outline"
                                            onclick={on_delete_click}
                                        >
                                            <i class="fas fa-trash"></i>
                                            {" Delete"}
                                        </button>
                                    </div>
                                </div>

                                {if let Some(desc) = &scenario.description {
                                    html! { <p class="text-base-content/70 mt-4">{desc}</p> }
                                } else {
                                    html! { <></> }
                                }}

                                <div class="divider"></div>

                                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    <div>
                                        <div class="stat bg-base-200 rounded-box">
                                            <div class="stat-title">{"Created At"}</div>
                                            <div class="stat-value text-lg">{scenario.created_at.format("%Y-%m-%d %H:%M").to_string()}</div>
                                        </div>
                                    </div>

                                    <div>
                                        <div class="stat bg-base-200 rounded-box">
                                            <div class="stat-title">{"Status"}</div>
                                            <div class="stat-value text-lg">
                                                <span class={status_badge.to_string()}>{status_text}</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                <div class="divider"></div>

                                <div class="alert alert-warning">
                                    <i class="fas fa-exclamation-triangle"></i>
                                    <div>
                                        <h4 class="font-bold">{"Apply This Scenario"}</h4>
                                        <p class="text-sm">{"Applying this scenario will convert all simulated transactions to real transactions. This action cannot be undone."}</p>
                                    </div>
                                    <button
                                        class="btn btn-warning btn-sm"
                                        onclick={on_apply_click}
                                    >
                                        <i class="fas fa-check"></i>
                                        {" Apply Scenario"}
                                    </button>
                                </div>
                            </div>
                        </div>

                        // Show forecast charts for all accounts with this scenario
                        {if let FetchState::Success(accounts) = &*accounts_state {
                            html! {
                                <div class="mt-6 space-y-6">
                                    <h3 class="text-xl font-bold">{"Account Forecasts (With This Scenario)"}</h3>
                                    <p class="text-sm text-base-content/60">
                                        {"These charts show how your accounts would look if this scenario's transactions were real."}
                                    </p>
                                    {for accounts.iter()
                                        .filter(|account| account.include_in_statistics)
                                        .map(|account| {
                                            html! {
                                                <ScenarioAccountForecast
                                                    key={account.id}
                                                    account={account.clone()}
                                                    scenario_id={id}
                                                />
                                            }
                                        })
                                    }
                                </div>
                            }
                        } else {
                            html! { <></> }
                        }}
                    </>
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
                        <span>{format!("Error loading scenario: {}", e)}</span>
                    </div>
                }
            }
            FetchState::NotStarted => html! { <></> }
        }
    };

    html! {
        <Layout title="Scenario Detail">
            <div class="container mx-auto p-4">
                {render_scenario_details()}
            </div>

            // Edit modal
            {
                if *show_edit_modal {
                    if let FetchState::Success(scenario) = &*scenario_state {
                        html! {
                            <ScenarioModal
                                show={*show_edit_modal}
                                on_close={on_close_edit_modal}
                                on_success={on_edit_success}
                                scenario={Some(scenario.clone())}
                            />
                        }
                    } else {
                        html! { <></> }
                    }
                } else {
                    html! { <></> }
                }
            }

            // Apply confirmation modal
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
                            {"Applying this scenario will permanently convert all simulated transactions to real transactions. "}
                            {"This will affect your actual budget and financial data."}
                        </p>
                        <p class="font-semibold">
                            {"Are you sure you want to proceed?"}
                        </p>
                    </div>
                    <div class="modal-action">
                        <button
                            type="button"
                            class="btn"
                            onclick={on_close_apply_modal.clone()}
                            disabled={*is_applying}
                        >
                            {"Cancel"}
                        </button>
                        <button
                            type="button"
                            class="btn btn-warning"
                            onclick={on_confirm_apply}
                            disabled={*is_applying}
                        >
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

// Component to show forecast for a single account with the scenario applied
#[derive(Properties, PartialEq)]
struct ScenarioAccountForecastProps {
    account: AccountResponse,
    scenario_id: i32,
}

#[function_component(ScenarioAccountForecast)]
fn scenario_account_forecast(props: &ScenarioAccountForecastProps) -> Html {
    let account = &props.account;
    let scenario_id = props.scenario_id;
    let account_id = account.id;

    // Fetch next 13 months of forecast data with scenario
    let start_date = Local::now().date_naive();
    let end_date = start_date + chrono::Duration::days(13 * 30);

    // TODO: Update API client to accept scenario_id parameter
    // For now, this will show the forecast without the scenario
    // The backend timeseries endpoints need to be updated to accept scenario_id query param
    let (fetch_state, _refetch) = use_fetch_with_refetch(move || {
        get_account_timeseries_with_ignored(account_id, start_date, end_date, true)
    });

    html! {
        <div class="card bg-base-100 shadow">
            <div class="card-body">
                <h4 class="card-title text-lg">
                    {&account.name}
                    <span class="text-sm font-normal text-base-content/60">{format!("({})", account.currency_code)}</span>
                </h4>

                {match &*fetch_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-error">
                            <span>{error}</span>
                        </div>
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
                            html! { <ForecastChart timeseries={timeseries.clone()} account_id={account_id} scenario_id={scenario_id} /> }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ForecastChartProps {
    timeseries: crate::api_client::timeseries::AccountStateTimeseries,
    account_id: i32,
    scenario_id: i32,
}

#[function_component(ForecastChart)]
fn forecast_chart(props: &ForecastChartProps) -> Html {
    let container_ref = use_node_ref();
    let timeseries = props.timeseries.clone();
    let div_id = format!("scenario-forecast-{}-{}", props.scenario_id, props.account_id);

    use_effect_with((container_ref.clone(), timeseries.clone(), div_id.clone()), move |(container_ref, timeseries, div_id)| {
        if let Some(element) = container_ref.cast::<HtmlElement>() {
            element.set_id(div_id);

            let points = &timeseries.data_points;
            let dates: Vec<String> = points.iter().map(|p| p.date.to_string()).collect();
            let balances: Vec<f64> = points.iter().map(|p| p.balance.to_string().parse::<f64>().unwrap_or(0.0)).collect();

            let trace = Scatter::new(dates, balances)
                .mode(Mode::LinesMarkers)
                .name("Balance");

            let layout = PlotlyLayout::new()
                .x_axis(plotly::layout::Axis::new().title("Date"))
                .y_axis(plotly::layout::Axis::new().title("Balance"))
                .height(300);

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
        <div ref={container_ref} style="width: 100%; height: 300px;"></div>
    }
}

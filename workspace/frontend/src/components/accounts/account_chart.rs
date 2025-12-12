use yew::prelude::*;
use crate::api_client::timeseries::{get_account_timeseries_with_ignored, AccountStateTimeseries};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use chrono::Local;
use plotly::{Plot, Scatter, Layout};
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
pub struct Props {
    pub account_id: i32,
}

#[function_component(AccountChart)]
pub fn account_chart(props: &Props) -> Html {
    let account_id = props.account_id;

    // Fetch last 13 months of data
    let end_date = Local::now().date_naive();
    let start_date = end_date - chrono::Duration::days(13 * 30);

    let (fetch_state, _refetch) = use_fetch_with_refetch(move || {
        get_account_timeseries_with_ignored(account_id, start_date, end_date, true)
    });

    html! {
        <div class="card bg-base-100 shadow mt-6">
            <div class="card-body">
                <h3 class="card-title text-lg">{"Balance Chart"}</h3>
                <p class="text-sm text-gray-500 mb-4">{"Account balance over the last 13 months"}</p>

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
                                <div class="text-center py-8 text-gray-500">
                                    <i class="fas fa-chart-area text-4xl mb-4 opacity-50"></i>
                                    <p>{"No balance data available."}</p>
                                    <p class="text-sm mt-2">{"Add manual account states and transactions to see the chart."}</p>
                                </div>
                            }
                        } else {
                            html! { <PlotlyChart timeseries={timeseries.clone()} account_id={account_id} /> }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PlotlyChartProps {
    timeseries: AccountStateTimeseries,
    account_id: i32,
}

#[function_component(PlotlyChart)]
fn plotly_chart(props: &PlotlyChartProps) -> Html {
    let container_ref = use_node_ref();
    let timeseries = props.timeseries.clone();
    let account_id = props.account_id;
    let div_id = format!("balance-chart-{}", account_id);

    use_effect_with((container_ref.clone(), timeseries.clone(), div_id.clone()), move |(container_ref, timeseries, div_id)| {
        if let Some(element) = container_ref.cast::<HtmlElement>() {
            // Set the ID on the element
            element.set_id(div_id);

            let points = &timeseries.data_points;

            // Extract dates and balances
            let dates: Vec<String> = points.iter()
                .map(|p| p.date.to_string())
                .collect();

            let balances: Vec<f64> = points.iter()
                .map(|p| p.balance.to_string().parse::<f64>().unwrap_or(0.0))
                .collect();

            // Create the trace
            let trace = Scatter::new(dates, balances)
                .mode(Mode::LinesMarkers)
                .name("Balance")
                .line(plotly::common::Line::new().color("rgb(59, 130, 246)").width(2.0));

            // Create layout
            let layout = Layout::new()
                .title(plotly::common::Title::with_text("Account Balance History"))
                .x_axis(plotly::layout::Axis::new().title(plotly::common::Title::with_text("Date")))
                .y_axis(plotly::layout::Axis::new().title(plotly::common::Title::with_text("Balance")))
                .height(400);

            // Serialize trace to JSON and parse as JS object
            let trace_json = serde_json::to_string(&trace).unwrap();
            let trace_js = js_sys::JSON::parse(&trace_json).unwrap();

            let data_js = js_sys::Array::new();
            data_js.push(&trace_js);

            // Serialize layout to JSON and parse as JS object
            let layout_json = serde_json::to_string(&layout).unwrap();
            let layout_js = js_sys::JSON::parse(&layout_json).unwrap();

            newPlot(div_id, data_js.into(), layout_js);
        }
        || ()
    });

    html! {
        <div ref={container_ref} style="width:100%; height:400px;"></div>
    }
}

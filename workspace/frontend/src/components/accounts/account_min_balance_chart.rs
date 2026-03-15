//! Chart component that displays the minimum balance per month for an account.
//!
//! This visualisation helps the user see whether the account's floor balance
//! is trending upward or downward over time.

use crate::api_client::statistics::{get_monthly_min_balance, MonthlyMinBalanceSeries};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use plotly::Layout;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Plotly)]
    fn newPlot(div_id: &str, data: JsValue, layout: JsValue);
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account_id: i32,
}

/// Shows the minimum account balance per month as a bar + line chart.
#[function_component(AccountMinBalanceChart)]
pub fn account_min_balance_chart(props: &Props) -> Html {
    let account_id = props.account_id;

    let (fetch_state, _refetch) = use_fetch_with_refetch(move || {
        get_monthly_min_balance(account_id, 13)
    });

    html! {
        <div class="card bg-base-100 shadow mt-6">
            <div class="card-body">
                <h3 class="card-title text-lg">{"Monthly Minimum Balance"}</h3>
                <p class="text-sm text-gray-500 mb-4">
                    {"Lowest balance reached each month over the last 13 months (green = positive, red = negative)"}
                </p>

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
                    FetchState::Success(series) => {
                        if series.data_points.is_empty() {
                            html! {
                                <div class="text-center py-8 text-gray-500">
                                    <i class="fas fa-chart-bar text-4xl mb-4 opacity-50"></i>
                                    <p>{"No monthly minimum balance data available."}</p>
                                    <p class="text-sm mt-2">{"Add transactions and manual states to see this chart."}</p>
                                </div>
                            }
                        } else {
                            html! { <MinBalancePlotlyChart series={series.clone()} account_id={account_id} /> }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct MinBalancePlotlyChartProps {
    series: MonthlyMinBalanceSeries,
    account_id: i32,
}

#[function_component(MinBalancePlotlyChart)]
fn min_balance_plotly_chart(props: &MinBalancePlotlyChartProps) -> Html {
    let container_ref = use_node_ref();
    let series = props.series.clone();
    let account_id = props.account_id;
    let div_id = format!("min-balance-chart-{}", account_id);

    use_effect_with(
        (container_ref.clone(), series.clone(), div_id.clone()),
        move |(container_ref, series, div_id)| {
            if let Some(element) = container_ref.cast::<HtmlElement>() {
                element.set_id(div_id);

                let labels: Vec<String> = series
                    .data_points
                    .iter()
                    .map(|p| format!("{}-{:02}", p.year, p.month))
                    .collect();

                let values: Vec<f64> = series
                    .data_points
                    .iter()
                    .map(|p| p.min_balance.to_string().parse::<f64>().unwrap_or(0.0))
                    .collect();

                let colors: Vec<&str> = values
                    .iter()
                    .map(|v| {
                        if *v >= 0.0 {
                            "rgb(34, 197, 94)"
                        } else {
                            "rgb(239, 68, 68)"
                        }
                    })
                    .collect();

                let bar_trace = serde_json::json!({
                    "type": "bar",
                    "x": labels,
                    "y": values,
                    "name": "Min Balance",
                    "marker": { "color": colors }
                });

                let line_trace = serde_json::json!({
                    "type": "scatter",
                    "mode": "lines+markers",
                    "x": labels,
                    "y": values,
                    "name": "Trend",
                    "line": { "color": "rgb(59, 130, 246)", "width": 2 },
                    "marker": { "size": 5 }
                });

                let layout = Layout::new()
                    .title(plotly::common::Title::with_text(
                        "Monthly Minimum Balance",
                    ))
                    .x_axis(
                        plotly::layout::Axis::new()
                            .title(plotly::common::Title::with_text("Month")),
                    )
                    .y_axis(
                        plotly::layout::Axis::new()
                            .title(plotly::common::Title::with_text("Min Balance")),
                    )
                    .height(400);

                let layout_json = serde_json::to_string(&layout).unwrap();
                let layout_js = js_sys::JSON::parse(&layout_json).unwrap();

                let bar_js = js_sys::JSON::parse(
                    &serde_json::to_string(&bar_trace).unwrap(),
                )
                .unwrap();
                let line_js = js_sys::JSON::parse(
                    &serde_json::to_string(&line_trace).unwrap(),
                )
                .unwrap();

                let data_js = js_sys::Array::new();
                data_js.push(&bar_js);
                data_js.push(&line_js);

                newPlot(div_id, data_js.into(), layout_js);
            }
            || ()
        },
    );

    html! {
        <div ref={container_ref} style="width:100%; height:400px;"></div>
    }
}

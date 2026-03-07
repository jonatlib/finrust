use crate::api_client::timeseries::{get_account_timeseries_with_ignored, AccountStateTimeseries};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use chrono::{Datelike, Local};
use plotly::Layout;
use std::collections::BTreeMap;
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

/// Monthly cashflow data point derived from balance timeseries
#[derive(Clone, Debug)]
struct MonthlyCashflow {
    label: String,
    cashflow: f64,
}

/// Computes monthly cashflow from daily balance data points.
///
/// For each month, takes the last balance minus the first balance
/// to determine the net cashflow for that month.
fn compute_monthly_cashflow(timeseries: &AccountStateTimeseries) -> Vec<MonthlyCashflow> {
    if timeseries.data_points.is_empty() {
        return Vec::new();
    }

    // Group data points by (year, month), keeping first and last balance per month
    let mut month_bounds: BTreeMap<(i32, u32), (f64, f64)> = BTreeMap::new();

    for point in &timeseries.data_points {
        let key = (point.date.year(), point.date.month());
        let balance = point.balance.to_string().parse::<f64>().unwrap_or(0.0);

        month_bounds
            .entry(key)
            .and_modify(|(_first, last)| *last = balance)
            .or_insert((balance, balance));
    }

    month_bounds
        .into_iter()
        .map(|((year, month), (first, last))| {
            let label = format!("{}-{:02}", year, month);
            MonthlyCashflow {
                label,
                cashflow: last - first,
            }
        })
        .collect()
}

/// Merges overlapping month between historical and forecast cashflow data.
///
/// When "today" falls mid-month, both the historical and forecast timeseries
/// contain a partial data point for the current month. This function detects
/// that overlap and combines the two partial cashflows into one entry kept
/// in the historical series (since it represents the current month).
fn merge_overlapping_month(
    mut hist: Vec<MonthlyCashflow>,
    mut forecast: Vec<MonthlyCashflow>,
) -> (Vec<MonthlyCashflow>, Vec<MonthlyCashflow>) {
    if let (Some(last_hist), Some(first_forecast)) = (hist.last(), forecast.first()) {
        if last_hist.label == first_forecast.label {
            let combined = last_hist.cashflow + first_forecast.cashflow;
            if let Some(h) = hist.last_mut() {
                h.cashflow = combined;
            }
            forecast.remove(0);
        }
    }
    (hist, forecast)
}

/// Cashflow per month chart showing historical and forecasted monthly net flows
#[function_component(AccountCashflowChart)]
pub fn account_cashflow_chart(props: &Props) -> Html {
    let account_id = props.account_id;

    let end_date = Local::now().date_naive();
    let historical_start = end_date - chrono::Duration::days(13 * 30);
    let forecast_end = end_date + chrono::Duration::days(13 * 30);

    let (hist_state, _hist_refetch) = use_fetch_with_refetch(move || {
        get_account_timeseries_with_ignored(account_id, historical_start, end_date, true)
    });

    let (forecast_state, _forecast_refetch) = use_fetch_with_refetch(move || {
        get_account_timeseries_with_ignored(account_id, end_date, forecast_end, true)
    });

    html! {
        <div class="card bg-base-100 shadow mt-6">
            <div class="card-body">
                <h3 class="card-title text-lg">{"Monthly Cashflow"}</h3>
                <p class="text-sm text-gray-500 mb-4">{"Net cashflow per month (historical and forecast)"}</p>

                {match (&*hist_state, &*forecast_state) {
                    (FetchState::Loading, _) | (_, FetchState::Loading) => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    (FetchState::Error(error), _) | (_, FetchState::Error(error)) => html! {
                        <div class="alert alert-error">
                            <span>{error}</span>
                        </div>
                    },
                    (FetchState::Success(hist_ts), FetchState::Success(forecast_ts)) => {
                        let hist_cf = compute_monthly_cashflow(hist_ts);
                        let forecast_cf = compute_monthly_cashflow(forecast_ts);

                        // Merge: if the current month appears in both historical and forecast,
                        // combine the two partial cashflows into a single data point.
                        let (hist_cf, forecast_cf) = merge_overlapping_month(hist_cf, forecast_cf);

                        if hist_cf.is_empty() && forecast_cf.is_empty() {
                            html! {
                                <div class="text-center py-8 text-gray-500">
                                    <i class="fas fa-chart-bar text-4xl mb-4 opacity-50"></i>
                                    <p>{"No cashflow data available."}</p>
                                </div>
                            }
                        } else {
                            html! {
                                <CashflowPlotlyChart
                                    historical={hist_cf}
                                    forecast={forecast_cf}
                                    account_id={account_id}
                                />
                            }
                        }
                    },
                    _ => html! { <></> },
                }}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct CashflowPlotlyChartProps {
    historical: Vec<MonthlyCashflow>,
    forecast: Vec<MonthlyCashflow>,
    account_id: i32,
}

impl PartialEq for MonthlyCashflow {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label && self.cashflow == other.cashflow
    }
}

#[function_component(CashflowPlotlyChart)]
fn cashflow_plotly_chart(props: &CashflowPlotlyChartProps) -> Html {
    let container_ref = use_node_ref();
    let historical = props.historical.clone();
    let forecast = props.forecast.clone();
    let account_id = props.account_id;
    let div_id = format!("cashflow-chart-{}", account_id);

    use_effect_with(
        (container_ref.clone(), historical.clone(), forecast.clone(), div_id.clone()),
        move |(container_ref, historical, forecast, div_id)| {
            if let Some(element) = container_ref.cast::<HtmlElement>() {
                element.set_id(div_id);

                let hist_labels: Vec<String> = historical.iter().map(|c| c.label.clone()).collect();
                let hist_values: Vec<f64> = historical.iter().map(|c| c.cashflow).collect();
                let hist_colors: Vec<&str> = hist_values.iter()
                    .map(|v| if *v >= 0.0 { "rgb(34, 197, 94)" } else { "rgb(239, 68, 68)" })
                    .collect();

                let forecast_labels: Vec<String> = forecast.iter().map(|c| c.label.clone()).collect();
                let forecast_values: Vec<f64> = forecast.iter().map(|c| c.cashflow).collect();
                let forecast_colors: Vec<&str> = forecast_values.iter()
                    .map(|v| if *v >= 0.0 { "rgba(34, 197, 94, 0.5)" } else { "rgba(239, 68, 68, 0.5)" })
                    .collect();

                // Build historical trace as raw JSON for marker color array support
                let hist_trace_json = serde_json::json!({
                    "type": "bar",
                    "x": hist_labels,
                    "y": hist_values,
                    "name": "Historical",
                    "marker": { "color": hist_colors }
                });

                let forecast_trace_json = serde_json::json!({
                    "type": "bar",
                    "x": forecast_labels,
                    "y": forecast_values,
                    "name": "Forecast",
                    "marker": {
                        "color": forecast_colors,
                        "line": { "color": "rgba(100,100,100,0.5)", "width": 1, "dash": "dash" }
                    }
                });

                let layout = Layout::new()
                    .title(plotly::common::Title::with_text("Monthly Cashflow"))
                    .x_axis(plotly::layout::Axis::new().title(plotly::common::Title::with_text("Month")))
                    .y_axis(plotly::layout::Axis::new().title(plotly::common::Title::with_text("Cashflow")))
                    .height(400);

                let hist_js = js_sys::JSON::parse(
                    &serde_json::to_string(&hist_trace_json).unwrap()
                ).unwrap();
                let forecast_js = js_sys::JSON::parse(
                    &serde_json::to_string(&forecast_trace_json).unwrap()
                ).unwrap();

                let data_js = js_sys::Array::new();
                data_js.push(&hist_js);
                data_js.push(&forecast_js);

                let layout_json = serde_json::to_string(&layout).unwrap();
                let layout_js = js_sys::JSON::parse(&layout_json).unwrap();

                newPlot(div_id, data_js.into(), layout_js);
            }
            || ()
        },
    );

    html! {
        <div ref={container_ref} style="width:100%; height:400px;"></div>
    }
}

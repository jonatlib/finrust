use yew::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::Element;
use crate::mock_data::{generate_net_worth_history, generate_forecast, get_mock_accounts};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Plotly)]
    pub fn newPlot(div_id: &str, data: JsValue, layout: JsValue, config: JsValue);
}

#[function_component(NetWorthChart)]
pub fn net_worth_chart() -> Html {
    let chart_ref = use_node_ref();

    use_effect_with(chart_ref.clone(), move |chart_ref| {
        if let Some(element) = chart_ref.cast::<Element>() {
            let history = generate_net_worth_history();
            let forecast = generate_forecast();
            let accounts = get_mock_accounts();
            let included_accounts: Vec<_> = accounts.iter().filter(|a| a.include_in_overview).collect();

            let mut combined_data = history.clone();
            let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
            combined_data.extend(forecast.iter().filter(|f| f.date > today_str).cloned());

            let dates: Vec<String> = combined_data.iter().map(|d| d.date.clone()).collect();
            let totals: Vec<f64> = combined_data.iter().map(|d| {
                included_accounts.iter().map(|a| d.accounts.get(&a.id).unwrap_or(&0.0)).sum()
            }).collect();

            let trace = serde_json::json!([{
                "x": dates,
                "y": totals,
                "type": "scatter",
                "mode": "lines",
                "fill": "tozeroy",
                "line": {"color": "#22c55e", "shape": "spline"},
                "name": "Net Worth"
            }]);

            let layout = serde_json::json!({
                "margin": {"t": 10, "r": 10, "l": 50, "b": 30},
                "paper_bgcolor": "rgba(0,0,0,0)",
                "plot_bgcolor": "rgba(0,0,0,0)",
                "xaxis": {"showgrid": false},
                "yaxis": {"showgrid": true, "gridcolor": "#eee"},
                "shapes": [{
                    "type": "line",
                    "x0": today_str,
                    "y0": 0,
                    "x1": today_str,
                    "y1": 1,
                    "xref": "x",
                    "yref": "paper",
                    "line": {"color": "#ef4444", "width": 2}
                }]
            });

            let config = serde_json::json!({"responsive": true, "displayModeBar": false});

            let div_id = element.id();
            if !div_id.is_empty() {
                newPlot(
                    &div_id,
                    serde_wasm_bindgen::to_value(&trace).unwrap(),
                    serde_wasm_bindgen::to_value(&layout).unwrap(),
                    serde_wasm_bindgen::to_value(&config).unwrap(),
                );
            }
        }
        || ()
    });

    html! {
        <div ref={chart_ref} id="chart-networth-total" class="chart-container" style="height: 300px;"></div>
    }
}

#[function_component(BalanceBreakdownChart)]
pub fn balance_breakdown_chart() -> Html {
    let chart_ref = use_node_ref();

    use_effect_with(chart_ref.clone(), move |chart_ref| {
        if let Some(element) = chart_ref.cast::<Element>() {
            let history = generate_net_worth_history();
            let forecast = generate_forecast();
            let accounts = get_mock_accounts();
            let included_accounts: Vec<_> = accounts.iter().filter(|a| a.include_in_overview).collect();

            let mut combined_data = history.clone();
            let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
            combined_data.extend(forecast.iter().filter(|f| f.date > today_str).cloned());

            let dates: Vec<String> = combined_data.iter().map(|d| d.date.clone()).collect();

            let traces: Vec<_> = included_accounts.iter().map(|acc| {
                let values: Vec<f64> = combined_data.iter()
                    .map(|d| *d.accounts.get(&acc.id).unwrap_or(&0.0))
                    .collect();

                serde_json::json!({
                    "x": dates.clone(),
                    "y": values,
                    "type": "scatter",
                    "mode": "lines",
                    "stackgroup": "one",
                    "name": acc.name,
                    "fill": "tonexty"
                })
            }).collect();

            let layout = serde_json::json!({
                "margin": {"t": 10, "r": 10, "l": 50, "b": 30},
                "paper_bgcolor": "rgba(0,0,0,0)",
                "plot_bgcolor": "rgba(0,0,0,0)",
                "xaxis": {"showgrid": false},
                "yaxis": {"showgrid": true, "gridcolor": "#eee"},
                "showlegend": true,
                "legend": {"orientation": "h", "y": -0.2},
                "shapes": [{
                    "type": "line",
                    "x0": today_str,
                    "y0": 0,
                    "x1": today_str,
                    "y1": 1,
                    "xref": "x",
                    "yref": "paper",
                    "line": {"color": "#ef4444", "width": 2}
                }]
            });

            let config = serde_json::json!({"responsive": true, "displayModeBar": false});

            let div_id = element.id();
            if !div_id.is_empty() {
                newPlot(
                    &div_id,
                    serde_wasm_bindgen::to_value(&traces).unwrap(),
                    serde_wasm_bindgen::to_value(&layout).unwrap(),
                    serde_wasm_bindgen::to_value(&config).unwrap(),
                );
            }
        }
        || ()
    });

    html! {
        <div ref={chart_ref} id="chart-balance-breakdown" class="chart-container" style="height: 300px;"></div>
    }
}

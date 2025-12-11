use yew::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::Element;
use chrono::{Local, Duration};
use rust_decimal::prelude::*;
use crate::api_client::account::get_accounts;
use crate::api_client::timeseries::get_all_accounts_timeseries;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Plotly)]
    pub fn newPlot(div_id: &str, data: JsValue, layout: JsValue);
}

#[function_component(NetWorthChart)]
pub fn net_worth_chart() -> Html {
    let chart_ref = use_node_ref();
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let (timeseries_state, _) = use_fetch_with_refetch(|| async {
        let today = Local::now().date_naive();
        let start_date = today - Duration::days(13 * 30); // ~13 months history
        let end_date = today + Duration::days(13 * 30); // ~13 months forecast
        log::info!("Fetching timeseries from {} to {}", start_date, end_date);
        get_all_accounts_timeseries(start_date, end_date).await
    });

    use_effect_with((chart_ref.clone(), accounts_state.clone(), timeseries_state.clone()),
        move |(chart_ref, accounts_state, timeseries_state)| {
        if let Some(element) = chart_ref.cast::<Element>() {
            if let (FetchState::Success(accounts), FetchState::Success(timeseries)) =
                (&**accounts_state, &**timeseries_state) {

                log::info!("NetWorthChart: Processing {} accounts and {} data points",
                    accounts.len(), timeseries.data_points.len());

                let included_account_ids: Vec<i32> = accounts
                    .iter()
                    .filter(|a| a.include_in_statistics)
                    .map(|a| a.id)
                    .collect();

                log::info!("NetWorthChart: Included account IDs: {:?}", included_account_ids);

                // Aggregate data across all included accounts by date
                let mut date_totals: std::collections::BTreeMap<String, Decimal> = std::collections::BTreeMap::new();

                for point in &timeseries.data_points {
                    if included_account_ids.contains(&point.account_id) {
                        let date_str = point.date.format("%Y-%m-%d").to_string();
                        *date_totals.entry(date_str).or_insert(Decimal::ZERO) += point.balance;
                    }
                }

                log::info!("NetWorthChart: Aggregated {} date points", date_totals.len());

                let dates: Vec<String> = date_totals.keys().cloned().collect();
                let totals: Vec<f64> = date_totals.values()
                    .map(|d| d.to_f64().unwrap_or(0.0))
                    .collect();

                if dates.is_empty() {
                    log::warn!("NetWorthChart: No data to plot!");
                } else {
                    log::info!("NetWorthChart: Plotting {} points from {} to {}",
                        dates.len(), dates.first().unwrap_or(&"N/A".to_string()),
                        dates.last().unwrap_or(&"N/A".to_string()));
                }

                let today_str = Local::now().format("%Y-%m-%d").to_string();

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

                let div_id = element.id();
                if !div_id.is_empty() {
                    // Convert JSON to JS values using js_sys::JSON::parse (like the working chart)
                    let trace_json = serde_json::to_string(&trace).unwrap();
                    let trace_js = js_sys::JSON::parse(&trace_json).unwrap();

                    let layout_json = serde_json::to_string(&layout).unwrap();
                    let layout_js = js_sys::JSON::parse(&layout_json).unwrap();

                    log::info!("NetWorthChart: Calling newPlot for div: {}", div_id);
                    newPlot(
                        &div_id,
                        trace_js,
                        layout_js,
                    );
                }
            }
        }
        || ()
    });

    let is_loading = matches!(*accounts_state, FetchState::Loading)
        || matches!(*timeseries_state, FetchState::Loading);

    if is_loading {
        return html! {
            <div class="flex justify-center items-center" style="height: 300px;">
                <span class="loading loading-spinner loading-lg"></span>
            </div>
        };
    }

    html! {
        <div ref={chart_ref} id="chart-networth-total" class="chart-container" style="height: 300px;"></div>
    }
}

#[function_component(BalanceBreakdownChart)]
pub fn balance_breakdown_chart() -> Html {
    let chart_ref = use_node_ref();
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);
    let (timeseries_state, _) = use_fetch_with_refetch(|| async {
        let today = Local::now().date_naive();
        let start_date = today - Duration::days(13 * 30); // ~13 months history
        let end_date = today + Duration::days(13 * 30); // ~13 months forecast
        log::info!("Fetching timeseries from {} to {}", start_date, end_date);
        get_all_accounts_timeseries(start_date, end_date).await
    });

    use_effect_with((chart_ref.clone(), accounts_state.clone(), timeseries_state.clone()),
        move |(chart_ref, accounts_state, timeseries_state)| {
        if let Some(element) = chart_ref.cast::<Element>() {
            if let (FetchState::Success(accounts), FetchState::Success(timeseries)) =
                (&**accounts_state, &**timeseries_state) {

                log::info!("BalanceBreakdownChart: Processing {} accounts and {} data points",
                    accounts.len(), timeseries.data_points.len());

                let included_accounts: Vec<_> = accounts
                    .iter()
                    .filter(|a| a.include_in_statistics)
                    .collect();

                log::info!("BalanceBreakdownChart: {} included accounts", included_accounts.len());

                // Get all unique dates
                let mut all_dates: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
                for point in &timeseries.data_points {
                    all_dates.insert(point.date.format("%Y-%m-%d").to_string());
                }

                let dates: Vec<String> = all_dates.iter().cloned().collect();
                log::info!("BalanceBreakdownChart: {} unique dates", dates.len());

                // Create a map of account_id -> (date -> balance)
                let mut account_data: std::collections::HashMap<i32, std::collections::HashMap<String, Decimal>> = std::collections::HashMap::new();
                for point in &timeseries.data_points {
                    account_data.entry(point.account_id)
                        .or_insert_with(std::collections::HashMap::new)
                        .insert(point.date.format("%Y-%m-%d").to_string(), point.balance);
                }

                // Create traces for each account
                let traces: Vec<_> = included_accounts.iter().map(|account| {
                    let values: Vec<f64> = dates.iter().map(|date_str| {
                        account_data.get(&account.id)
                            .and_then(|date_map| date_map.get(date_str))
                            .map(|balance| balance.to_f64().unwrap_or(0.0))
                            .unwrap_or(0.0)
                    }).collect();

                    serde_json::json!({
                        "x": dates.clone(),
                        "y": values,
                        "type": "scatter",
                        "mode": "lines",
                        "stackgroup": "one",
                        "name": account.name,
                        "fill": "tonexty"
                    })
                }).collect();

                let today_str = Local::now().format("%Y-%m-%d").to_string();

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

                let div_id = element.id();
                if !div_id.is_empty() {
                    // Convert JSON to JS values using js_sys::JSON::parse (like the working chart)
                    let traces_json = serde_json::to_string(&traces).unwrap();
                    let traces_js = js_sys::JSON::parse(&traces_json).unwrap();

                    let layout_json = serde_json::to_string(&layout).unwrap();
                    let layout_js = js_sys::JSON::parse(&layout_json).unwrap();

                    log::info!("BalanceBreakdownChart: Calling newPlot for div: {}", div_id);
                    newPlot(
                        &div_id,
                        traces_js,
                        layout_js,
                    );
                }
            }
        }
        || ()
    });

    let is_loading = matches!(*accounts_state, FetchState::Loading)
        || matches!(*timeseries_state, FetchState::Loading);

    if is_loading {
        return html! {
            <div class="flex justify-center items-center" style="height: 300px;">
                <span class="loading loading-spinner loading-lg"></span>
            </div>
        };
    }

    html! {
        <div ref={chart_ref} id="chart-balance-breakdown" class="chart-container" style="height: 300px;"></div>
    }
}

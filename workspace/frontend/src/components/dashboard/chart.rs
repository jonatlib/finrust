use yew::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::Element;
use chrono::{Local, Duration};
use rust_decimal::prelude::*;
use crate::api_client::account::{get_accounts_with_ignored, AccountResponse};
use crate::api_client::timeseries::get_all_accounts_timeseries_with_ignored;
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
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));
    let (timeseries_state, _) = use_fetch_with_refetch(|| async {
        let today = Local::now().date_naive();
        let start_date = today - Duration::days(13 * 30);
        let end_date = today + Duration::days(13 * 30);
        get_all_accounts_timeseries_with_ignored(start_date, end_date, true).await
    });

    use_effect_with((chart_ref.clone(), accounts_state.clone(), timeseries_state.clone()),
        move |(chart_ref, accounts_state, timeseries_state)| {
        if let Some(element) = chart_ref.cast::<Element>() {
            if let (FetchState::Success(accounts), FetchState::Success(timeseries)) =
                (&**accounts_state, &**timeseries_state) {

                let all_ids: std::collections::HashSet<i32> = accounts.iter().map(|a| a.id).collect();
                let liquid_ids: std::collections::HashSet<i32> = accounts.iter()
                    .filter(|a| a.is_liquid)
                    .map(|a| a.id)
                    .collect();

                let mut date_totals: std::collections::BTreeMap<String, Decimal> = std::collections::BTreeMap::new();
                let mut liquid_totals: std::collections::BTreeMap<String, Decimal> = std::collections::BTreeMap::new();

                for point in &timeseries.data_points {
                    if all_ids.contains(&point.account_id) {
                        let date_str = point.date.format("%Y-%m-%d").to_string();
                        *date_totals.entry(date_str.clone()).or_insert(Decimal::ZERO) += point.balance;
                        if liquid_ids.contains(&point.account_id) {
                            *liquid_totals.entry(date_str).or_insert(Decimal::ZERO) += point.balance;
                        }
                    }
                }

                let dates: Vec<String> = date_totals.keys().cloned().collect();
                let totals: Vec<f64> = dates.iter()
                    .map(|d| date_totals.get(d).and_then(|v| v.to_f64()).unwrap_or(0.0))
                    .collect();
                let liquid_vals: Vec<f64> = dates.iter()
                    .map(|d| liquid_totals.get(d).and_then(|v| v.to_f64()).unwrap_or(0.0))
                    .collect();

                let today_str = Local::now().format("%Y-%m-%d").to_string();

                let traces = serde_json::json!([
                    {
                        "x": dates,
                        "y": totals,
                        "type": "scatter",
                        "mode": "lines",
                        "fill": "tozeroy",
                        "line": {"color": "#22c55e", "shape": "spline"},
                        "name": "Total Net Worth"
                    },
                    {
                        "x": dates,
                        "y": liquid_vals,
                        "type": "scatter",
                        "mode": "lines",
                        "fill": "tozeroy",
                        "line": {"color": "#3b82f6", "shape": "spline", "dash": "dot"},
                        "name": "Liquid Net Worth"
                    }
                ]);

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
                    let traces_json = serde_json::to_string(&traces).unwrap();
                    let traces_js = js_sys::JSON::parse(&traces_json).unwrap();
                    let layout_json = serde_json::to_string(&layout).unwrap();
                    let layout_js = js_sys::JSON::parse(&layout_json).unwrap();
                    newPlot(&div_id, traces_js, layout_js);
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

#[derive(Clone, PartialEq)]
pub enum BreakdownFilter {
    All,
    LiquidOnly,
    NonLiquidOnly,
    IncludeIgnored,
}

#[derive(Properties, Clone, PartialEq)]
pub struct BreakdownProps {
    pub filter: BreakdownFilter,
    pub chart_id: AttrValue,
}

fn filter_accounts<'a>(accounts: &'a [AccountResponse], filter: &BreakdownFilter) -> Vec<&'a AccountResponse> {
    accounts.iter().filter(|a| {
        let stats_ok = match filter {
            BreakdownFilter::IncludeIgnored => true,
            _ => a.include_in_statistics,
        };
        let liquid_ok = match filter {
            BreakdownFilter::LiquidOnly => a.is_liquid,
            BreakdownFilter::NonLiquidOnly => !a.is_liquid,
            _ => true,
        };
        stats_ok && liquid_ok
    }).collect()
}

#[function_component(FilteredBreakdownChart)]
pub fn filtered_breakdown_chart(props: &BreakdownProps) -> Html {
    let chart_ref = use_node_ref();
    let (accounts_state, _) = use_fetch_with_refetch(|| get_accounts_with_ignored(true));

    let include_ignored = props.filter == BreakdownFilter::IncludeIgnored;
    let (timeseries_state, _) = use_fetch_with_refetch(move || async move {
        let today = Local::now().date_naive();
        let start_date = today - Duration::days(13 * 30);
        let end_date = today + Duration::days(13 * 30);
        get_all_accounts_timeseries_with_ignored(start_date, end_date, include_ignored).await
    });

    let filter = props.filter.clone();
    let chart_id = props.chart_id.clone();

    use_effect_with((chart_ref.clone(), accounts_state.clone(), timeseries_state.clone(), filter.clone()),
        move |(chart_ref, accounts_state, timeseries_state, filter)| {
        if let Some(element) = chart_ref.cast::<Element>() {
            if let (FetchState::Success(accounts), FetchState::Success(timeseries)) =
                (&**accounts_state, &**timeseries_state) {

                let included_accounts = filter_accounts(accounts, filter);

                let mut all_dates: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
                for point in &timeseries.data_points {
                    all_dates.insert(point.date.format("%Y-%m-%d").to_string());
                }
                let dates: Vec<String> = all_dates.iter().cloned().collect();

                let mut account_data: std::collections::HashMap<i32, std::collections::HashMap<String, Decimal>> = std::collections::HashMap::new();
                for point in &timeseries.data_points {
                    account_data.entry(point.account_id)
                        .or_default()
                        .insert(point.date.format("%Y-%m-%d").to_string(), point.balance);
                }

                let traces: Vec<_> = included_accounts.iter().enumerate().map(|(idx, account)| {
                    let values: Vec<f64> = dates.iter().map(|date_str| {
                        account_data.get(&account.id)
                            .and_then(|dm| dm.get(date_str))
                            .map(|b| b.to_f64().unwrap_or(0.0))
                            .unwrap_or(0.0)
                    }).collect();

                    let color = account.color.clone()
                        .unwrap_or_else(|| crate::colors::color_by_index(idx).to_string());

                    serde_json::json!({
                        "x": dates.clone(),
                        "y": values,
                        "type": "scatter",
                        "mode": "lines",
                        "stackgroup": "one",
                        "name": account.name,
                        "fill": "tonexty",
                        "line": {"color": color}
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
                    let traces_json = serde_json::to_string(&traces).unwrap();
                    let traces_js = js_sys::JSON::parse(&traces_json).unwrap();
                    let layout_json = serde_json::to_string(&layout).unwrap();
                    let layout_js = js_sys::JSON::parse(&layout_json).unwrap();
                    newPlot(&div_id, traces_js, layout_js);
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
        <div ref={chart_ref} id={chart_id} class="chart-container" style="height: 300px;"></div>
    }
}

#[function_component(BalanceBreakdownChart)]
pub fn balance_breakdown_chart() -> Html {
    html! { <FilteredBreakdownChart filter={BreakdownFilter::All} chart_id="chart-balance-breakdown" /> }
}

#[function_component(LiquidBreakdownChart)]
pub fn liquid_breakdown_chart() -> Html {
    html! { <FilteredBreakdownChart filter={BreakdownFilter::LiquidOnly} chart_id="chart-balance-liquid" /> }
}

#[function_component(NonLiquidBreakdownChart)]
pub fn non_liquid_breakdown_chart() -> Html {
    html! { <FilteredBreakdownChart filter={BreakdownFilter::NonLiquidOnly} chart_id="chart-balance-nonliquid" /> }
}

#[function_component(AllAccountsBreakdownChart)]
pub fn all_accounts_breakdown_chart() -> Html {
    html! { <FilteredBreakdownChart filter={BreakdownFilter::IncludeIgnored} chart_id="chart-balance-all" /> }
}

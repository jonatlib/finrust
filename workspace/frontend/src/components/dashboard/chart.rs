use yew::prelude::*;

#[function_component(NetWorthChart)]
pub fn net_worth_chart() -> Html {
    // TODO: Implement Plotly JS interop or use yew-plotly
    html! {
        <div class="chart-container" style="height: 300px; border: 1px dashed #ccc; display: flex; align-items: center; justify-content: center;">
            <span class="text-gray-500">{"[Net Worth Chart Placeholder]"}</span>
        </div>
    }
}

#[function_component(BalanceBreakdownChart)]
pub fn balance_breakdown_chart() -> Html {
    // TODO: Implement Plotly JS interop or use yew-plotly
    html! {
        <div class="chart-container" style="height: 300px; border: 1px dashed #ccc; display: flex; align-items: center; justify-content: center;">
            <span class="text-gray-500">{"[Balance Breakdown Chart Placeholder]"}</span>
        </div>
    }
}

use yew::prelude::*;

#[function_component(CategoryPieChart)]
pub fn category_pie_chart() -> Html {
    html! {
        <div class="chart-container" style="height: 300px; border: 1px dashed #ccc; display: flex; align-items: center; justify-content: center;">
            <span class="text-gray-500">{"[Category Pie Chart]"}</span>
        </div>
    }
}

#[function_component(CashFlowBarChart)]
pub fn cash_flow_bar_chart() -> Html {
    html! {
        <div class="chart-container" style="height: 300px; border: 1px dashed #ccc; display: flex; align-items: center; justify-content: center;">
            <span class="text-gray-500">{"[Cash Flow Bar Chart]"}</span>
        </div>
    }
}

#[function_component(SankeyChart)]
pub fn sankey_chart() -> Html {
    html! {
        <div class="chart-container" style="height: 400px; border: 1px dashed #ccc; display: flex; align-items: center; justify-content: center;">
            <span class="text-gray-500">{"[Sankey Diagram]"}</span>
        </div>
    }
}

#[function_component(AccountBreakdownChart)]
pub fn account_breakdown_chart() -> Html {
    html! {
         <div class="chart-container" style="height: 300px; border: 1px dashed #ccc; display: flex; align-items: center; justify-content: center;">
            <span class="text-gray-500">{"[Account Breakdown Chart]"}</span>
        </div>
    }
}

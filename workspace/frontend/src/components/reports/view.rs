use yew::prelude::*;
use super::charts::{CategoryPieChart, CashFlowBarChart, SankeyChart, AccountBreakdownChart};

#[function_component(Reports)]
pub fn reports() -> Html {
    html! {
        <>
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
                <div class="card bg-base-100 shadow">
                    <div class="card-body">
                        <h3 class="card-title">{"Spending by Category"}</h3>
                        <CategoryPieChart />
                    </div>
                </div>
                <div class="card bg-base-100 shadow">
                    <div class="card-body">
                        <h3 class="card-title">{"Cash Flow (Income vs Expenses)"}</h3>
                        <CashFlowBarChart />
                    </div>
                </div>
            </div>

            <div class="card bg-base-100 shadow mb-6">
                <div class="card-body">
                    <h3 class="card-title">{"Income Flow (Sankey)"}</h3>
                     <SankeyChart />
                </div>
            </div>

            <div class="card bg-base-100 shadow">
                <div class="card-body">
                    <h3 class="card-title">{"Account Breakdown"}</h3>
                    <AccountBreakdownChart />
                </div>
            </div>
        </>
    }
}

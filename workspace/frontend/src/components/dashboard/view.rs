use yew::prelude::*;
use super::stats::Stats;
use super::chart::{NetWorthChart, BalanceBreakdownChart};
use super::activity::RecentActivity;

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    html! {
        <>
            <Stats />
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
                <div class="card bg-base-100 shadow">
                    <div class="card-body">
                        <h2 class="card-title">{"Net Worth Trend (History + Forecast)"}</h2>
                        <NetWorthChart />
                    </div>
                </div>
                <div class="card bg-base-100 shadow">
                    <div class="card-body">
                        <h2 class="card-title">{"Recent Activity"}</h2>
                        <RecentActivity />
                    </div>
                </div>
            </div>
            <div class="card bg-base-100 shadow mt-6">
                <div class="card-body">
                    <h2 class="card-title">{"Account Balances (Breakdown)"}</h2>
                    <BalanceBreakdownChart />
                </div>
            </div>
        </>
    }
}

use super::account_bubbles::AccountTypeBubbles;
use super::activity::RecentActivity;
use super::category_stats::CategoryStats;
use super::chart::{BalanceBreakdownChart, NetWorthChart};
use super::metrics::DashboardMetrics;
use super::stats::Stats;
use yew::prelude::*;

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    html! {
        <>
            <Stats />
            <div class="mt-6">
                <DashboardMetrics />
            </div>
            <div class="card bg-base-100 shadow mt-6">
                <div class="card-body">
                    <h2 class="card-title">{"Account Bubbles"}</h2>
                    <p class="text-sm text-gray-500">{"Grouped by account type with current and month-end balances"}</p>
                    <AccountTypeBubbles />
                </div>
            </div>
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
            <CategoryStats />
        </>
    }
}

use yew::prelude::*;
use super::stats::ForecastStats;
use super::chart::ForecastChart;

#[function_component(Forecast)]
pub fn forecast() -> Html {
    html! {
        <>
            <div class="flex justify-between items-center mb-6">
                <div class="join">
                     <button class="btn join-item btn-active">{"30 Days"}</button>
                     <button class="btn join-item">{"90 Days"}</button>
                     <button class="btn join-item">{"1 Year"}</button>
                </div>
                <button class="btn btn-primary">{"Refresh Projection"}</button>
            </div>
            <ForecastStats />
            <div class="card bg-base-100 shadow">
                <div class="card-body">
                    <h3 class="card-title">{"Projected Balances (Stacked)"}</h3>
                    <ForecastChart />
                </div>
            </div>
        </>
    }
}

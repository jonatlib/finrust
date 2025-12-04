use yew::prelude::*;

#[function_component(ForecastStats)]
pub fn forecast_stats() -> Html {
    html! {
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Lowest Balance"}</div>
                    <div class="stat-value text-error">{"$980.00"}</div>
                    <div class="stat-desc">{"Risk Point (Oct 24)"}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Highest Balance"}</div>
                    <div class="stat-value text-success">{"$28,450.00"}</div>
                    <div class="stat-desc">{"Peak savings (Total)"}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Projected (90d)"}</div>
                    <div class="stat-value text-primary">{"$26,500.00"}</div>
                </div>
            </div>
        </div>
    }
}

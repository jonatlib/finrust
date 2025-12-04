use yew::prelude::*;

#[function_component(ForecastChart)]
pub fn forecast_chart() -> Html {
    // Placeholder
    html! {
        <div id="chart-forecast" style="height: 400px; border: 1px dashed #ccc; display: flex; align-items: center; justify-content: center;">
             <span class="text-gray-500">{"[Forecast Stacked Area Chart]"}</span>
        </div>
    }
}

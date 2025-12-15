use yew::prelude::*;
use yew_router::prelude::*;
use crate::api_client::scenario::Scenario;
use crate::router::Route;

#[derive(Properties, PartialEq)]
pub struct ScenarioCardProps {
    pub scenario: Scenario,
}

#[function_component(ScenarioCard)]
pub fn scenario_card(props: &ScenarioCardProps) -> Html {
    let navigator = use_navigator().unwrap();
    let scenario = &props.scenario;

    let on_click = {
        let scenario_id = scenario.id;
        let navigator = navigator.clone();
        Callback::from(move |_: MouseEvent| {
            navigator.push(&Route::ScenarioDetail { id: scenario_id });
        })
    };

    let badge_class = if scenario.is_active {
        "badge badge-success"
    } else {
        "badge badge-ghost"
    };

    let status_text = if scenario.is_active { "Active" } else { "Inactive" };

    html! {
        <div class="card bg-base-100 shadow-md hover:shadow-lg transition-shadow cursor-pointer" onclick={on_click}>
            <div class="card-body">
                <div class="flex justify-between items-start">
                    <h3 class="card-title text-lg">{&scenario.name}</h3>
                    <span class={badge_class}>{status_text}</span>
                </div>

                {if let Some(description) = &scenario.description {
                    html! {
                        <p class="text-sm text-base-content/70 line-clamp-2">{description}</p>
                    }
                } else {
                    html! {}
                }}

                <div class="text-xs text-base-content/50 mt-2">
                    {"Created: "}{scenario.created_at.format("%Y-%m-%d %H:%M").to_string()}
                </div>
            </div>
        </div>
    }
}

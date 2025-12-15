use yew::prelude::*;
use crate::api_client::scenario::{get_scenarios, delete_scenario, Scenario};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use super::scenario_card::ScenarioCard;
use super::scenario_modal::ScenarioModal;

#[function_component(Scenarios)]
pub fn scenarios() -> Html {
    log::trace!("Scenarios component rendering");
    let (fetch_state, refetch) = use_fetch_with_refetch(|| get_scenarios());
    let show_modal = use_state(|| false);
    let edit_scenario = use_state(|| None::<Scenario>);
    let deleting_id = use_state(|| None::<i32>);

    log::debug!("Scenarios component state: loading={}, success={}, error={}",
        fetch_state.is_loading(), fetch_state.is_success(), fetch_state.is_error());

    let on_open_create_modal = {
        let show_modal = show_modal.clone();
        let edit_scenario = edit_scenario.clone();
        Callback::from(move |_| {
            log::info!("Opening Create Scenario modal");
            edit_scenario.set(None);
            show_modal.set(true);
        })
    };

    let on_close_modal = {
        let show_modal = show_modal.clone();
        let edit_scenario = edit_scenario.clone();
        Callback::from(move |_| {
            log::info!("Closing Scenario modal");
            show_modal.set(false);
            edit_scenario.set(None);
        })
    };

    let on_success = {
        let refetch = refetch.clone();
        Callback::from(move |_| {
            log::info!("Scenario operation successful, refetching scenarios");
            refetch.emit(());
        })
    };

    html! {
        <>
            <ScenarioModal
                show={*show_modal}
                on_close={on_close_modal}
                on_success={on_success.clone()}
                scenario={(*edit_scenario).clone()}
            />

            <div class="flex justify-between items-center mb-4">
                <div>
                    <h2 class="text-2xl font-bold">{"What-If Scenarios"}</h2>
                    <p class="text-sm text-base-content/60 mt-1">
                        {"Create and manage hypothetical financial scenarios"}
                    </p>
                </div>
                <button
                    class="btn btn-primary btn-sm"
                    onclick={on_open_create_modal}
                >
                    <i class="fas fa-plus"></i> {" New Scenario"}
                </button>
            </div>

            {
                match &*fetch_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-error">
                            <span>{error}</span>
                            <button class="btn btn-sm" onclick={move |_| refetch.emit(())}>
                                {"Retry"}
                            </button>
                        </div>
                    },
                    FetchState::Success(scenarios) => {
                        if scenarios.is_empty() {
                            html! {
                                <div class="text-center py-12">
                                    <i class="fas fa-flask text-6xl text-base-content/20 mb-4"></i>
                                    <p class="text-lg text-base-content/70 mb-2">{"No scenarios yet"}</p>
                                    <p class="text-sm text-base-content/50">{"Create your first scenario to explore what-if situations"}</p>
                                </div>
                            }
                        } else {
                            html! {
                                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                    { for scenarios.iter().map(|scenario| {
                                        log::trace!("Rendering scenario card for: {}", scenario.name);
                                        html! { <ScenarioCard key={scenario.id} scenario={scenario.clone()} /> }
                                    })}
                                </div>
                            }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }
            }
        </>
    }
}

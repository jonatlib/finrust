use yew::prelude::*;
use crate::api_client::scenario::{Scenario, CreateScenarioRequest, UpdateScenarioRequest, create_scenario, update_scenario};

#[derive(Properties, PartialEq)]
pub struct ScenarioModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    /// If provided, the modal is in edit mode with this scenario
    pub scenario: Option<Scenario>,
}

#[function_component(ScenarioModal)]
pub fn scenario_modal(props: &ScenarioModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    let is_edit_mode = props.scenario.is_some();
    let title = if is_edit_mode { "Edit Scenario" } else { "New Scenario" };

    let on_submit = {
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let form_ref = form_ref.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let scenario = props.scenario.clone();
        let is_edit = scenario.is_some();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *is_submitting {
                return;
            }

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let name = form_data.get("name").as_string().unwrap_or_default();
                let description = form_data.get("description").as_string();
                let is_active = form_data.get("is_active").as_string().map(|v| v == "on").unwrap_or(false);

                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let on_close = on_close.clone();
                let on_success = on_success.clone();

                is_submitting.set(true);
                error_message.set(None);

                if is_edit {
                    // Edit mode - update scenario
                    let existing_scenario = scenario.clone().unwrap();
                    let scenario_id = existing_scenario.id;
                    let request = UpdateScenarioRequest {
                        name: Some(name.clone()),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                        is_active: Some(is_active),
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Updating scenario ID {}: {}", scenario_id, name);
                        match update_scenario(scenario_id, request).await {
                            Ok(scenario) => {
                                log::info!("Scenario updated successfully: {} (ID: {})", scenario.name, scenario.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to update scenario: {}", e);
                                error_message.set(Some(format!("Failed to update scenario: {}", e)));
                                is_submitting.set(false);
                            }
                        }
                    });
                } else {
                    // Create mode - create new scenario
                    let request = CreateScenarioRequest {
                        name: name.clone(),
                        description: if description.as_ref().map(|d| d.is_empty()).unwrap_or(true) { None } else { description },
                    };

                    wasm_bindgen_futures::spawn_local(async move {
                        log::info!("Creating scenario: {}", name);
                        match create_scenario(request).await {
                            Ok(scenario) => {
                                log::info!("Scenario created successfully: {} (ID: {})", scenario.name, scenario.id);
                                is_submitting.set(false);
                                on_success.emit(());
                                on_close.emit(());
                            }
                            Err(e) => {
                                log::error!("Failed to create scenario: {}", e);
                                error_message.set(Some(format!("Failed to create scenario: {}", e)));
                                is_submitting.set(false);
                            }
                        }
                    });
                }
            }
        })
    };

    let on_close = {
        let on_close = props.on_close.clone();
        let is_submitting = *is_submitting;
        Callback::from(move |_| {
            if !is_submitting {
                on_close.emit(())
            }
        })
    };

    // Get default values from scenario if in edit mode
    let default_name = props.scenario.as_ref().map(|s| s.name.clone()).unwrap_or_default();
    let default_description = props.scenario.as_ref().and_then(|s| s.description.clone()).unwrap_or_default();
    let default_is_active = props.scenario.as_ref().map(|s| s.is_active).unwrap_or(false);

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="scenario_modal">
            <div class="modal-box w-11/12 max-w-2xl">
                <h3 class="font-bold text-lg">{title}</h3>

                {if let Some(error) = (*error_message).as_ref() {
                    html! {
                        <div class="alert alert-error mt-4">
                            <span>{error}</span>
                        </div>
                    }
                } else {
                    html! {}
                }}

                <form ref={form_ref} onsubmit={on_submit} class="py-4 space-y-4">
                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Scenario Name"}</span></label>
                        <input
                            type="text"
                            name="name"
                            class="input input-bordered w-full"
                            placeholder="e.g. House Purchase"
                            value={default_name}
                            required={true}
                            disabled={*is_submitting}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Description (Optional)"}</span></label>
                        <textarea
                            name="description"
                            class="textarea textarea-bordered w-full"
                            rows="4"
                            placeholder="Describe what this scenario simulates..."
                            value={default_description}
                            disabled={*is_submitting}
                        />
                    </div>

                    {if is_edit_mode {
                        html! {
                            <div class="form-control">
                                <label class="label cursor-pointer justify-start gap-2">
                                    <input
                                        type="checkbox"
                                        name="is_active"
                                        class="checkbox checkbox-primary"
                                        checked={default_is_active}
                                        disabled={*is_submitting}
                                    />
                                    <span class="label-text">{"Active"}</span>
                                </label>
                                <label class="label">
                                    <span class="label-text-alt text-base-content/60">
                                        {"Active scenarios are highlighted in the list"}
                                    </span>
                                </label>
                            </div>
                        }
                    } else {
                        html! {}
                    }}

                    <div class="modal-action">
                        <button
                            type="button"
                            class="btn"
                            onclick={on_close.clone()}
                            disabled={*is_submitting}
                        >
                            {"Cancel"}
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled={*is_submitting}
                        >
                            {if *is_submitting {
                                if is_edit_mode {
                                    html! { <><span class="loading loading-spinner loading-sm"></span>{" Updating..."}</> }
                                } else {
                                    html! { <><span class="loading loading-spinner loading-sm"></span>{" Creating..."}</> }
                                }
                            } else {
                                if is_edit_mode {
                                    html! { "Update Scenario" }
                                } else {
                                    html! { "Create Scenario" }
                                }
                            }}
                        </button>
                    </div>
                </form>
            </div>
            <form class="modal-backdrop" method="dialog">
                <button onclick={on_close} disabled={*is_submitting}>{"close"}</button>
            </form>
        </dialog>
    }
}

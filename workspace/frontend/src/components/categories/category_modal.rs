use yew::prelude::*;
use web_sys::HtmlInputElement;
use crate::api_client::category::{create_category, update_category, CreateCategoryRequest, UpdateCategoryRequest, CategoryResponse};
use crate::common::toast::ToastContext;

#[derive(Properties, PartialEq)]
pub struct CategoryModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
    pub category: Option<CategoryResponse>,
    pub categories: Vec<CategoryResponse>, // For parent selection
}

#[function_component(CategoryModal)]
pub fn category_modal(props: &CategoryModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_loading = use_state(|| false);
    let toast_ctx = use_context::<ToastContext>().unwrap();

    let on_submit = {
        let form_ref = form_ref.clone();
        let on_close = props.on_close.clone();
        let on_success = props.on_success.clone();
        let is_loading = is_loading.clone();
        let category = props.category.clone();
        let toast_ctx = toast_ctx.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let form_ref = form_ref.clone();
            let on_close = on_close.clone();
            let on_success = on_success.clone();
            let is_loading = is_loading.clone();
            let category = category.clone();
            let toast_ctx = toast_ctx.clone();

            wasm_bindgen_futures::spawn_local(async move {
                is_loading.set(true);

                // Get form values using FormData
                if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                    let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                    let name = form_data.get("name").as_string().unwrap_or_default();
                    let description = form_data.get("description").as_string();
                    let description = description.filter(|s| !s.is_empty());

                    let parent_id_str = form_data.get("parent_id").as_string().unwrap_or_default();
                    let parent_id = if parent_id_str.is_empty() || parent_id_str == "none" {
                        None
                    } else {
                        parent_id_str.parse::<i32>().ok()
                    };

                    let result = if let Some(cat) = category {
                        // Update existing category
                        log::info!("Updating category ID: {}", cat.id);
                        let request = UpdateCategoryRequest {
                            name: if name.is_empty() { None } else { Some(name) },
                            description,
                            parent_id,
                        };
                        update_category(cat.id, request).await
                    } else {
                        // Create new category
                        log::info!("Creating new category: {}", name);
                        let request = CreateCategoryRequest {
                            name,
                            description,
                            parent_id,
                        };
                        create_category(request).await
                    };

                    is_loading.set(false);

                    match result {
                        Ok(_) => {
                            log::info!("Category saved successfully");
                            toast_ctx.show_success("Category saved successfully".to_string());
                            on_success.emit(());
                            on_close.emit(());
                        }
                        Err(e) => {
                            log::error!("Failed to save category: {}", e);
                            toast_ctx.show_error(format!("Failed to save category: {}", e));
                        }
                    }
                }
            });
        })
    };

    let on_close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    // Determine modal title and button text
    let (title, button_text) = if props.category.is_some() {
        ("Edit Category", "Update Category")
    } else {
        ("Add Category", "Create Category")
    };

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="category_modal">
            <div class="modal-box">
                <h3 class="font-bold text-lg">{title}</h3>
                <form ref={form_ref} onsubmit={on_submit} class="py-4 space-y-4">
                    <div class="form-control">
                        <label class="label">
                            <span class="label-text">{"Name"}</span>
                        </label>
                        <input
                            name="name"
                            type="text"
                            placeholder="e.g. Groceries"
                            class="input input-bordered w-full"
                            required={true}
                            value={props.category.as_ref().map(|c| c.name.clone()).unwrap_or_default()}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label">
                            <span class="label-text">{"Description (optional)"}</span>
                        </label>
                        <input
                            name="description"
                            type="text"
                            placeholder="e.g. Food and household items"
                            class="input input-bordered w-full"
                            value={props.category.as_ref().and_then(|c| c.description.clone()).unwrap_or_default()}
                        />
                    </div>

                    <div class="form-control">
                        <label class="label">
                            <span class="label-text">{"Parent Category (optional)"}</span>
                        </label>
                        <select name="parent_id" class="select select-bordered w-full">
                            <option value="none">{"None (Root Category)"}</option>
                            { for props.categories.iter()
                                .filter(|c| {
                                    // Don't show self as parent option
                                    if let Some(current) = &props.category {
                                        c.id != current.id
                                    } else {
                                        true
                                    }
                                })
                                .map(|c| {
                                    let selected = props.category.as_ref()
                                        .and_then(|cat| cat.parent_id)
                                        .map(|pid| pid == c.id)
                                        .unwrap_or(false);
                                    html! {
                                        <option
                                            value={c.id.to_string()}
                                            selected={selected}
                                        >
                                            {&c.name}
                                        </option>
                                    }
                                })
                            }
                        </select>
                    </div>

                    <div class="modal-action">
                        <button
                            type="button"
                            class="btn"
                            onclick={on_close.clone()}
                            disabled={*is_loading}
                        >
                            {"Cancel"}
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled={*is_loading}
                        >
                            if *is_loading {
                                <span class="loading loading-spinner"></span>
                            }
                            {button_text}
                        </button>
                    </div>
                </form>
            </div>
            <form class="modal-backdrop" method="dialog">
                <button onclick={on_close}>{"close"}</button>
            </form>
        </dialog>
    }
}

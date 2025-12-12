use yew::prelude::*;
use crate::api_client::category::{delete_category, CategoryResponse};
use crate::common::toast::ToastContext;

#[derive(Properties, PartialEq)]
pub struct CategoryCardProps {
    pub category: CategoryResponse,
    pub on_edit: Callback<CategoryResponse>,
    pub on_delete_success: Callback<()>,
    pub parent_name: Option<String>,
}

#[function_component(CategoryCard)]
pub fn category_card(props: &CategoryCardProps) -> Html {
    let is_deleting = use_state(|| false);
    let toast_ctx = use_context::<ToastContext>().unwrap();

    let on_edit = {
        let category = props.category.clone();
        let on_edit = props.on_edit.clone();
        Callback::from(move |_| {
            log::info!("Edit clicked for category: {}", category.name);
            on_edit.emit(category.clone());
        })
    };

    let on_delete = {
        let category = props.category.clone();
        let is_deleting = is_deleting.clone();
        let on_delete_success = props.on_delete_success.clone();
        let toast_ctx = toast_ctx.clone();

        Callback::from(move |_| {
            let category = category.clone();
            let is_deleting = is_deleting.clone();
            let on_delete_success = on_delete_success.clone();
            let toast_ctx = toast_ctx.clone();

            if web_sys::window()
                .and_then(|w| w.confirm_with_message(&format!("Are you sure you want to delete category '{}'?", category.name)).ok())
                .unwrap_or(false)
            {
                log::info!("Delete confirmed for category: {}", category.name);
                wasm_bindgen_futures::spawn_local(async move {
                    is_deleting.set(true);

                    match delete_category(category.id).await {
                        Ok(_) => {
                            log::info!("Category deleted successfully: {}", category.name);
                            toast_ctx.show_success("Category deleted successfully".to_string());
                            on_delete_success.emit(());
                        }
                        Err(e) => {
                            log::error!("Failed to delete category: {}", e);
                            toast_ctx.show_error(format!("Failed to delete category: {}", e));
                            is_deleting.set(false);
                        }
                    }
                });
            }
        })
    };

    html! {
        <div class="card bg-base-100 shadow-md hover:shadow-lg transition-shadow">
            <div class="card-body">
                <div class="flex justify-between items-start">
                    <div class="flex-1">
                        <h3 class="card-title text-lg">{&props.category.name}</h3>
                        if let Some(description) = &props.category.description {
                            <p class="text-sm text-gray-600 mt-1">{description}</p>
                        }
                        if let Some(parent) = &props.parent_name {
                            <div class="badge badge-sm badge-ghost mt-2">
                                <i class="fas fa-folder mr-1"></i>
                                {parent}
                            </div>
                        }
                    </div>
                    <div class="flex gap-2">
                        <button
                            class="btn btn-ghost btn-sm"
                            onclick={on_edit}
                            disabled={*is_deleting}
                        >
                            <i class="fas fa-edit"></i>
                        </button>
                        <button
                            class="btn btn-ghost btn-sm text-error"
                            onclick={on_delete}
                            disabled={*is_deleting}
                        >
                            if *is_deleting {
                                <span class="loading loading-spinner loading-xs"></span>
                            } else {
                                <i class="fas fa-trash"></i>
                            }
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

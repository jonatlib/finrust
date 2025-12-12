use yew::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use crate::api_client::category::CategoryResponse;

#[derive(Properties, PartialEq)]
pub struct TreeItemProps {
    pub category: CategoryResponse,
    pub children_map: Rc<HashMap<i32, Vec<CategoryResponse>>>,
    pub on_edit: Callback<CategoryResponse>,
    pub on_delete_success: Callback<()>,
    pub level: usize,
}

#[function_component(TreeItem)]
pub fn tree_item(props: &TreeItemProps) -> Html {
    let is_expanded = use_state(|| true);
    let is_deleting = use_state(|| false);

    let toggle_expanded = {
        let is_expanded = is_expanded.clone();
        Callback::from(move |_| {
            is_expanded.set(!*is_expanded);
        })
    };

    let on_edit = {
        let category = props.category.clone();
        let on_edit = props.on_edit.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            log::info!("Edit clicked for category: {}", category.name);
            on_edit.emit(category.clone());
        })
    };

    let on_delete = {
        let category = props.category.clone();
        let is_deleting = is_deleting.clone();
        let on_delete_success = props.on_delete_success.clone();

        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            let category = category.clone();
            let is_deleting = is_deleting.clone();
            let on_delete_success = on_delete_success.clone();

            if web_sys::window()
                .and_then(|w| w.confirm_with_message(&format!("Are you sure you want to delete category '{}'? This will also delete all its subcategories.", category.name)).ok())
                .unwrap_or(false)
            {
                log::info!("Delete confirmed for category: {}", category.name);
                wasm_bindgen_futures::spawn_local(async move {
                    use crate::api_client::category::delete_category;
                    use crate::common::toast::ToastContext;

                    is_deleting.set(true);

                    match delete_category(category.id).await {
                        Ok(_) => {
                            log::info!("Category deleted successfully: {}", category.name);
                            on_delete_success.emit(());
                        }
                        Err(e) => {
                            log::error!("Failed to delete category: {}", e);
                            is_deleting.set(false);
                        }
                    }
                });
            }
        })
    };

    let children = props.children_map.get(&props.category.id).cloned().unwrap_or_default();
    let has_children = !children.is_empty();
    let indent_style = format!("margin-left: {}rem", props.level as f32 * 1.5);

    html! {
        <div class="category-tree-item">
            <div class="flex items-center py-2 px-4 hover:bg-base-200 rounded-lg transition-colors" style={indent_style}>
                // Expand/Collapse button
                <div class="w-6 flex-shrink-0">
                    if has_children {
                        <button
                            class="btn btn-ghost btn-xs"
                            onclick={toggle_expanded}
                        >
                            if *is_expanded {
                                <i class="fas fa-chevron-down"></i>
                            } else {
                                <i class="fas fa-chevron-right"></i>
                            }
                        </button>
                    }
                </div>

                // Category icon
                <div class="flex-shrink-0 w-8 h-8 flex items-center justify-center bg-primary/10 rounded-lg mr-3">
                    <i class="fas fa-folder text-primary"></i>
                </div>

                // Category info
                <div class="flex-1">
                    <div class="font-medium">{&props.category.name}</div>
                    if let Some(description) = &props.category.description {
                        <div class="text-sm text-gray-500">{description}</div>
                    }
                </div>

                // Children count badge
                if has_children {
                    <div class="badge badge-sm badge-ghost mr-2">
                        {format!("{} {}", children.len(), if children.len() == 1 { "child" } else { "children" })}
                    </div>
                }

                // Action buttons
                <div class="flex gap-1 flex-shrink-0">
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

            // Render children recursively if expanded
            if *is_expanded && has_children {
                <div class="category-tree-children">
                    { for children.iter().map(|child| {
                        html! {
                            <TreeItem
                                key={child.id}
                                category={child.clone()}
                                children_map={props.children_map.clone()}
                                on_edit={props.on_edit.clone()}
                                on_delete_success={props.on_delete_success.clone()}
                                level={props.level + 1}
                            />
                        }
                    })}
                </div>
            }
        </div>
    }
}

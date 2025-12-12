use yew::prelude::*;
use std::collections::HashMap;
use crate::api_client::category::{get_categories, CategoryResponse};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use super::category_card::CategoryCard;
use super::category_modal::CategoryModal;

#[function_component(Categories)]
pub fn categories() -> Html {
    log::trace!("Categories component rendering");
    let (fetch_state, refetch) = use_fetch_with_refetch(get_categories);
    let show_modal = use_state(|| false);
    let selected_category = use_state(|| None::<CategoryResponse>);

    log::debug!("Categories component state: loading={}, success={}, error={}",
        fetch_state.is_loading(), fetch_state.is_success(), fetch_state.is_error());

    // Build parent name lookup
    let parent_lookup: Option<HashMap<i32, String>> = match &*fetch_state {
        FetchState::Success(categories) => {
            let mut lookup = HashMap::new();
            for category in categories {
                lookup.insert(category.id, category.name.clone());
            }
            Some(lookup)
        },
        _ => None,
    };

    // Group categories into root and children
    let (root_categories, child_categories): (Vec<CategoryResponse>, Vec<CategoryResponse>) = match &*fetch_state {
        FetchState::Success(categories) => {
            let mut roots = Vec::new();
            let mut children = Vec::new();
            for category in categories {
                if category.parent_id.is_none() {
                    roots.push(category.clone());
                } else {
                    children.push(category.clone());
                }
            }
            (roots, children)
        },
        _ => (Vec::new(), Vec::new()),
    };

    let on_open_modal = {
        let show_modal = show_modal.clone();
        let selected_category = selected_category.clone();
        Callback::from(move |_| {
            log::info!("Opening Add Category modal");
            selected_category.set(None);
            show_modal.set(true);
        })
    };

    let on_edit_category = {
        let show_modal = show_modal.clone();
        let selected_category = selected_category.clone();
        Callback::from(move |category: CategoryResponse| {
            log::info!("Opening Edit Category modal for: {}", category.name);
            selected_category.set(Some(category));
            show_modal.set(true);
        })
    };

    let on_close_modal = {
        let show_modal = show_modal.clone();
        let selected_category = selected_category.clone();
        Callback::from(move |_| {
            log::info!("Closing Category modal");
            show_modal.set(false);
            selected_category.set(None);
        })
    };

    let on_success = {
        let refetch = refetch.clone();
        let show_modal = show_modal.clone();
        let selected_category = selected_category.clone();
        Callback::from(move |_| {
            log::info!("Category operation successful, refetching categories");
            refetch.emit(());
            show_modal.set(false);
            selected_category.set(None);
        })
    };

    let on_delete_success = {
        let refetch = refetch.clone();
        Callback::from(move |_| {
            log::info!("Category deleted, refetching categories");
            refetch.emit(());
        })
    };

    // Get all categories for the modal
    let all_categories = match &*fetch_state {
        FetchState::Success(categories) => categories.clone(),
        _ => Vec::new(),
    };

    html! {
        <>
            <CategoryModal
                show={*show_modal}
                on_close={on_close_modal}
                on_success={on_success}
                category={(*selected_category).clone()}
                categories={all_categories}
            />

            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Categories"}</h2>
                <button
                    class="btn btn-primary btn-sm"
                    onclick={on_open_modal}
                >
                    <i class="fas fa-plus"></i> {" Add Category"}
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
                    FetchState::Success(categories) => {
                        if categories.is_empty() {
                            html! {
                                <div class="text-center py-8">
                                    <p class="text-gray-500">{"No categories found. Create your first category to get started!"}</p>
                                </div>
                            }
                        } else {
                            html! {
                                <div class="space-y-6">
                                    // Root Categories
                                    if !root_categories.is_empty() {
                                        <div>
                                            <h3 class="text-xl font-semibold mb-3">{"Root Categories"}</h3>
                                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                                { for root_categories.iter().map(|category| {
                                                    log::trace!("Rendering root category card for: {}", category.name);
                                                    html! {
                                                        <CategoryCard
                                                            key={category.id}
                                                            category={category.clone()}
                                                            on_edit={on_edit_category.clone()}
                                                            on_delete_success={on_delete_success.clone()}
                                                            parent_name={None::<String>}
                                                        />
                                                    }
                                                })}
                                            </div>
                                        </div>
                                    }

                                    // Child Categories
                                    if !child_categories.is_empty() {
                                        <div>
                                            <h3 class="text-xl font-semibold mb-3">{"Subcategories"}</h3>
                                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                                { for child_categories.iter().map(|category| {
                                                    log::trace!("Rendering child category card for: {}", category.name);
                                                    let parent_name = category.parent_id
                                                        .and_then(|pid| parent_lookup.as_ref()
                                                            .and_then(|lookup| lookup.get(&pid).cloned()));
                                                    html! {
                                                        <CategoryCard
                                                            key={category.id}
                                                            category={category.clone()}
                                                            on_edit={on_edit_category.clone()}
                                                            on_delete_success={on_delete_success.clone()}
                                                            parent_name={parent_name}
                                                        />
                                                    }
                                                })}
                                            </div>
                                        </div>
                                    }
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

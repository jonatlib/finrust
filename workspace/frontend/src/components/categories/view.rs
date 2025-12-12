use yew::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use crate::api_client::category::{get_categories, get_category_stats, CategoryResponse, CategoryStatistics};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use super::tree_item::TreeItem;
use super::category_modal::CategoryModal;
use chrono::{Datelike, NaiveDate};

#[function_component(Categories)]
pub fn categories() -> Html {
    log::trace!("Categories component rendering");
    let (fetch_state, refetch) = use_fetch_with_refetch(get_categories);
    let show_modal = use_state(|| false);
    let selected_category = use_state(|| None::<CategoryResponse>);

    // Get current year start and end dates for stats
    let now = chrono::Local::now().naive_local();
    let start_date = NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap().to_string();
    let end_date = NaiveDate::from_ymd_opt(now.year(), 12, 31).unwrap().to_string();

    let (stats_state, _stats_refetch) = use_fetch_with_refetch(move || {
        let start_date = start_date.clone();
        let end_date = end_date.clone();
        async move {
            get_category_stats(&start_date, &end_date).await
        }
    });

    log::debug!("Categories component state: loading={}, success={}, error={}",
        fetch_state.is_loading(), fetch_state.is_success(), fetch_state.is_error());

    // Build tree structure: map parent_id -> list of children
    let (root_categories, children_map): (Vec<CategoryResponse>, HashMap<i32, Vec<CategoryResponse>>) = match &*fetch_state {
        FetchState::Success(categories) => {
            let mut roots = Vec::new();
            let mut children_map: HashMap<i32, Vec<CategoryResponse>> = HashMap::new();

            for category in categories {
                if category.parent_id.is_none() {
                    roots.push(category.clone());
                } else if let Some(parent_id) = category.parent_id {
                    children_map.entry(parent_id)
                        .or_insert_with(Vec::new)
                        .push(category.clone());
                }
            }

            (roots, children_map)
        },
        _ => (Vec::new(), HashMap::new()),
    };

    // Build stats map: category_id -> CategoryStatistics
    let stats_map: HashMap<i32, CategoryStatistics> = match &*stats_state {
        FetchState::Success(stats) => {
            stats.iter().map(|stat| (stat.category_id, stat.clone())).collect()
        },
        _ => HashMap::new(),
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
                            let children_map_rc = Rc::new(children_map);
                            let stats_map_rc = Rc::new(stats_map);

                            html! {
                                <div class="card bg-base-100 shadow">
                                    <div class="card-body">
                                        <h3 class="text-lg font-semibold mb-4">{"Category Tree"}</h3>
                                        <div class="category-tree space-y-1">
                                            { for root_categories.iter().map(|category| {
                                                log::trace!("Rendering tree for root category: {}", category.name);
                                                html! {
                                                    <TreeItem
                                                        key={category.id}
                                                        category={category.clone()}
                                                        children_map={children_map_rc.clone()}
                                                        stats_map={stats_map_rc.clone()}
                                                        on_edit={on_edit_category.clone()}
                                                        on_delete_success={on_delete_success.clone()}
                                                        level={0}
                                                    />
                                                }
                                            })}
                                        </div>
                                    </div>
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

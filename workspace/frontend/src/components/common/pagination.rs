use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PaginationProps {
    pub current_page: u64,
    pub total_items: u64,
    pub items_per_page: u64,
    pub on_page_change: Callback<u64>,
}

#[function_component(Pagination)]
pub fn pagination(props: &PaginationProps) -> Html {
    let total_pages = (props.total_items as f64 / props.items_per_page as f64).ceil() as u64;

    if total_pages <= 1 {
        return html! {};
    }

    let current = props.current_page;

    // Calculate visible page numbers
    let mut pages = Vec::new();
    let max_visible = 5;

    if total_pages <= max_visible {
        // Show all pages if we have few pages
        for i in 1..=total_pages {
            pages.push(i);
        }
    } else {
        // Show first, last, current and neighbors
        pages.push(1);

        let start = current.saturating_sub(1).max(2);
        let end = (current + 1).min(total_pages - 1);

        if start > 2 {
            pages.push(0); // 0 represents ellipsis
        }

        for i in start..=end {
            pages.push(i);
        }

        if end < total_pages - 1 {
            pages.push(0); // 0 represents ellipsis
        }

        pages.push(total_pages);
    }

    let on_previous = {
        let on_page_change = props.on_page_change.clone();
        let current = current;
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if current > 1 {
                on_page_change.emit(current - 1);
            }
        })
    };

    let on_next = {
        let on_page_change = props.on_page_change.clone();
        let current = current;
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if current < total_pages {
                on_page_change.emit(current + 1);
            }
        })
    };

    html! {
        <div class="flex justify-center items-center gap-2 my-4">
            <button
                class="btn btn-sm"
                disabled={current <= 1}
                onclick={on_previous}
            >
                <i class="fas fa-chevron-left"></i>
            </button>

            {for pages.iter().map(|&page| {
                if page == 0 {
                    // Ellipsis
                    html! {
                        <span class="px-2">{"..."}</span>
                    }
                } else {
                    let on_click = {
                        let on_page_change = props.on_page_change.clone();
                        Callback::from(move |e: MouseEvent| {
                            e.prevent_default();
                            on_page_change.emit(page);
                        })
                    };

                    html! {
                        <button
                            class={classes!(
                                "btn",
                                "btn-sm",
                                if page == current { "btn-primary" } else { "" }
                            )}
                            onclick={on_click}
                        >
                            {page}
                        </button>
                    }
                }
            })}

            <button
                class="btn btn-sm"
                disabled={current >= total_pages}
                onclick={on_next}
            >
                <i class="fas fa-chevron-right"></i>
            </button>

            <div class="ml-4 text-sm text-base-content/70">
                {format!("Page {} of {} ({} items)", current, total_pages, props.total_items)}
            </div>
        </div>
    }
}

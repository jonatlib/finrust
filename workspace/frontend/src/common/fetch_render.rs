use yew::prelude::*;
use crate::hooks::FetchState;
use super::loading::LoadingSpinner;
use super::error::ErrorDisplay;

#[derive(Properties)]
pub struct FetchRenderProps<T: Clone + PartialEq + 'static> {
    pub state: FetchState<T>,
    pub render: Callback<T, Html>,
    #[prop_or_default]
    pub on_retry: Option<Callback<()>>,
    #[prop_or_default]
    pub loading_text: Option<String>,
    #[prop_or_default]
    pub empty_message: Option<String>,
}

impl<T: Clone + PartialEq + 'static> PartialEq for FetchRenderProps<T> {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.loading_text == other.loading_text
            && self.empty_message == other.empty_message
    }
}

/// Component that handles rendering based on FetchState
/// - Loading: shows loading spinner
/// - Error: shows error display with optional retry
/// - Success: calls render callback with data
#[function_component(FetchRender)]
pub fn fetch_render<T>(props: &FetchRenderProps<T>) -> Html
where
    T: Clone + PartialEq + 'static,
{
    match &props.state {
        FetchState::NotStarted => html! {},
        FetchState::Loading => {
            if let Some(text) = &props.loading_text {
                html! {
                    <div class="flex flex-col justify-center items-center py-12 gap-4">
                        <span class="loading loading-spinner loading-lg"></span>
                        <p class="text-sm text-gray-500">{text}</p>
                    </div>
                }
            } else {
                html! { <LoadingSpinner /> }
            }
        }
        FetchState::Error(err) => {
            html! {
                <ErrorDisplay
                    message={err.clone()}
                    on_retry={props.on_retry.clone()}
                />
            }
        }
        FetchState::Success(data) => props.render.emit(data.clone()),
    }
}

/// Helper for rendering lists with empty state handling
#[derive(Properties)]
pub struct FetchRenderListProps<T: Clone + PartialEq + 'static> {
    pub state: FetchState<Vec<T>>,
    pub render_item: Callback<T, Html>,
    #[prop_or_default]
    pub on_retry: Option<Callback<()>>,
    #[prop_or_default]
    pub empty_message: Option<String>,
    #[prop_or_default]
    pub container_class: Option<String>,
}

impl<T: Clone + PartialEq + 'static> PartialEq for FetchRenderListProps<T> {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.empty_message == other.empty_message
            && self.container_class == other.container_class
    }
}

#[function_component(FetchRenderList)]
pub fn fetch_render_list<T>(props: &FetchRenderListProps<T>) -> Html
where
    T: Clone + PartialEq + 'static,
{
    let render = {
        let render_item = props.render_item.clone();
        let empty_message = props.empty_message.clone();
        let container_class = props.container_class.clone();

        Callback::from(move |items: Vec<T>| {
            if items.is_empty() {
                html! {
                    <div class="alert alert-info">
                        <i class="fas fa-info-circle"></i>
                        <span>{empty_message.clone().unwrap_or_else(|| "No items found.".to_string())}</span>
                    </div>
                }
            } else {
                html! {
                    <div class={container_class.clone().unwrap_or_else(|| "grid grid-cols-1 gap-4".to_string())}>
                        { for items.iter().map(|item| render_item.emit(item.clone())) }
                    </div>
                }
            }
        })
    };

    html! {
        <FetchRender<Vec<T>>
            state={props.state.clone()}
            render={render}
            on_retry={props.on_retry.clone()}
        />
    }
}

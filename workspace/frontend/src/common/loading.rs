use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LoadingProps {
    #[prop_or_default]
    pub size: LoadingSize,
    #[prop_or_default]
    pub text: Option<String>,
}

#[derive(Clone, PartialEq, Default)]
pub enum LoadingSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl LoadingSize {
    fn class(&self) -> &'static str {
        match self {
            LoadingSize::Small => "loading-sm",
            LoadingSize::Medium => "loading-md",
            LoadingSize::Large => "loading-lg",
        }
    }
}

#[function_component(Loading)]
pub fn loading(props: &LoadingProps) -> Html {
    html! {
        <div class="flex flex-col justify-center items-center py-12 gap-4">
            <span class={classes!("loading", "loading-spinner", props.size.class())}></span>
            {if let Some(text) = &props.text {
                html! { <p class="text-sm text-gray-500">{text}</p> }
            } else {
                html! {}
            }}
        </div>
    }
}

/// Centered loading spinner without text
#[function_component(LoadingSpinner)]
pub fn loading_spinner() -> Html {
    html! {
        <div class="flex justify-center items-center py-12">
            <span class="loading loading-spinner loading-lg"></span>
        </div>
    }
}

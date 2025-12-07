use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod mock_data;
mod pages;
mod router;
pub mod api_client;
pub mod hooks;
pub mod common;
pub mod settings;

use common::toast::ToastProvider;
use router::{Route, switch};

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <ToastProvider>
            <BrowserRouter>
                <Switch<Route> render={switch} />
            </BrowserRouter>
        </ToastProvider>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run_app() {
    // Initialize settings first
    settings::init_settings();

    // Initialize logger with settings
    let settings = settings::get_settings();
    wasm_logger::init(wasm_logger::Config::new(settings.log_level));

    log::info!("=== FinRust Frontend Application Starting ===");
    log::info!("Application settings: {:?}", settings);
    log::debug!("API base URL: {}", settings.api_base_url());
    log::debug!("Debug mode: {}", settings.debug_mode);

    log::trace!("Initializing Yew renderer");
    yew::Renderer::<App>::new().render();
    log::info!("Application initialized successfully");
}

use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod mock_data;
pub mod api_client;
pub mod hooks;
pub mod common;
pub mod settings;

use common::toast::ToastProvider;
use components::accounts::Accounts;
use components::budgets::Budgets;
use components::dashboard::Dashboard;
use components::forecast::Forecast;
use components::layout::layout::Layout;
use components::recurring::Recurring;
use components::reports::Reports;
use components::settings::Settings;
use components::transactions::Transactions;

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/dashboard")]
    Dashboard,
    #[at("/accounts")]
    Accounts,
    #[at("/transactions")]
    Transactions,
    #[at("/recurring")]
    Recurring,
    #[at("/budgets")]
    Budgets,
    #[at("/forecast")]
    Forecast,
    #[at("/reports")]
    Reports,
    #[at("/settings")]
    Settings,
    #[at("/about")]
    About,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    log::debug!("Routing to: {:?}", routes);
    match routes {
        Route::Home | Route::Dashboard => {
            log::trace!("Rendering Dashboard page");
            html! { <Layout title="Dashboard"><Dashboard /></Layout> }
        }
        Route::Accounts => {
            log::trace!("Rendering Accounts page");
            // For Accounts page, we need to create a wrapper that provides refresh functionality
            html! { <AccountsPage /> }
        }
        Route::Transactions => {
            log::trace!("Rendering Transactions page");
            html! { <Layout title="Transactions"><Transactions /></Layout> }
        }
        Route::Recurring => {
            log::trace!("Rendering Recurring page");
            html! { <Layout title="Recurring"><Recurring /></Layout> }
        }
        Route::Budgets => {
            log::trace!("Rendering Budgets page");
            html! { <Layout title="Budgets"><Budgets /></Layout> }
        }
        Route::Forecast => {
            log::trace!("Rendering Forecast page");
            html! { <Layout title="Forecast"><Forecast /></Layout> }
        }
        Route::Reports => {
            log::trace!("Rendering Reports page");
            html! { <Layout title="Reports"><Reports /></Layout> }
        }
        Route::Settings => {
            log::trace!("Rendering Settings page");
            html! { <Layout title="Settings"><Settings /></Layout> }
        }
        Route::About => {
            log::trace!("Rendering About page");
            html! { <Layout title="About"><div>{"About Page"}</div></Layout> }
        }
        Route::NotFound => {
            log::warn!("404 - Route not found");
            html! { <Layout title="404"><h1>{"404 Not Found"}</h1></Layout> }
        }
    }
}

#[function_component(AccountsPage)]
fn accounts_page() -> Html {
    let refresh_trigger = use_state(|| 0);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            log::debug!("Accounts page refresh triggered");
            refresh_trigger.set(*refresh_trigger + 1);
        })
    };

    html! {
        <Layout title="Accounts" on_refresh={Some(on_refresh)}>
            <Accounts key={*refresh_trigger} />
        </Layout>
    }
}

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

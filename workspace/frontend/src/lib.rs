use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod mock_data;
pub mod api_client;
pub mod hooks;
pub mod common;

use components::layout::layout::Layout;
use common::toast::ToastProvider;
use components::dashboard::Dashboard;
use components::accounts::Accounts;
use components::transactions::Transactions;
use components::recurring::Recurring;
use components::budgets::Budgets;
use components::forecast::Forecast;
use components::reports::Reports;
use components::settings::Settings;

#[derive(Clone, Routable, PartialEq)]
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
    match routes {
        Route::Home | Route::Dashboard => html! { <Layout title="Dashboard"><Dashboard /></Layout> },
        Route::Accounts => html! { <Layout title="Accounts"><Accounts /></Layout> },
        Route::Transactions => html! { <Layout title="Transactions"><Transactions /></Layout> },
        Route::Recurring => html! { <Layout title="Recurring"><Recurring /></Layout> },
        Route::Budgets => html! { <Layout title="Budgets"><Budgets /></Layout> },
        Route::Forecast => html! { <Layout title="Forecast"><Forecast /></Layout> },
        Route::Reports => html! { <Layout title="Reports"><Reports /></Layout> },
        Route::Settings => html! { <Layout title="Settings"><Settings /></Layout> },
        Route::About => html! { <Layout title="About"><div>{"About Page"}</div></Layout> },
        Route::NotFound => html! { <Layout title="404"><h1>{"404 Not Found"}</h1></Layout> },
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
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}

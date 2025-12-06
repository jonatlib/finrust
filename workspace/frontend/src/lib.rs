use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod mock_data;
pub mod api_client;
pub mod hooks;
pub mod common;

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
        Route::Accounts => {
            // For Accounts page, we need to create a wrapper that provides refresh functionality
            html! { <AccountsPage /> }
        }
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

#[function_component(AccountsPage)]
fn accounts_page() -> Html {
    let refresh_trigger = use_state(|| 0);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
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
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}

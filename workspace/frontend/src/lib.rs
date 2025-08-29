use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod pages;

use components::navbar::Navbar;
use pages::{home::Home, about::About, admin::Admin};

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/about")]
    About,
    #[at("/admin")]
    Admin,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { <Home /> },
        Route::About => html! { <About /> },
        Route::Admin => html! { <Admin /> },
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <div class="min-h-screen bg-base-100">
                <Navbar />
                <main class="container mx-auto px-4 py-8">
                    <Switch<Route> render={switch} />
                </main>
            </div>
        </BrowserRouter>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
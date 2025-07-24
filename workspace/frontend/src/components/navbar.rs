use yew::prelude::*;
use yew_router::prelude::*;
use crate::Route;

#[function_component(Navbar)]
pub fn navbar() -> Html {
    html! {
        <div class="navbar bg-primary text-primary-content">
            <div class="navbar-start">
                <div class="dropdown">
                    <div tabindex="0" role="button" class="btn btn-ghost lg:hidden">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h8m-8 6h16" />
                        </svg>
                    </div>
                    <ul tabindex="0" class="menu menu-sm dropdown-content mt-3 z-[1] p-2 shadow bg-base-100 rounded-box w-52">
                        <li><Link<Route> to={Route::Home} classes="text-base-content">{"Home"}</Link<Route>></li>
                        <li><Link<Route> to={Route::About} classes="text-base-content">{"About"}</Link<Route>></li>
                    </ul>
                </div>
                <Link<Route> to={Route::Home} classes="btn btn-ghost text-xl">{"FinRust"}</Link<Route>>
            </div>
            <div class="navbar-center hidden lg:flex">
                <ul class="menu menu-horizontal px-1">
                    <li><Link<Route> to={Route::Home} classes="btn btn-ghost">{"Home"}</Link<Route>></li>
                    <li><Link<Route> to={Route::About} classes="btn btn-ghost">{"About"}</Link<Route>></li>
                </ul>
            </div>
            <div class="navbar-end">
                <button class="btn btn-ghost btn-circle">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                </button>
            </div>
        </div>
    }
}
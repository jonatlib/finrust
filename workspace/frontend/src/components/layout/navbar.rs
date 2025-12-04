use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub title: String,
}

#[function_component(Navbar)]
pub fn navbar(props: &Props) -> Html {
    html! {
        <div class="navbar bg-base-100 shadow-sm z-40 sticky top-0">
            <div class="flex-none lg:hidden">
                <label aria-label="open sidebar" class="btn btn-square btn-ghost" for="my-drawer">
                    <i class="fas fa-bars text-xl"></i>
                </label>
            </div>
            <div class="flex-1 px-4">
                <h1 class="text-xl font-bold" id="page-title">{ &props.title }</h1>
            </div>
            <div class="flex-none gap-2">
                <select class="select select-sm select-bordered w-full max-w-xs hidden md:block" id="global-currency-select">
                    <option value="USD">{"USD ($)"}</option>
                    <option value="EUR">{"EUR (€)"}</option>
                    <option value="GBP">{"GBP (£)"}</option>
                </select>

                <label class="swap swap-rotate btn btn-ghost btn-circle">
                    <input id="theme-toggle" type="checkbox"/>
                    <i class="swap-on fill-current fas fa-sun text-xl"></i>
                    <i class="swap-off fill-current fas fa-moon text-xl"></i>
                </label>
            </div>
        </div>
    }
}

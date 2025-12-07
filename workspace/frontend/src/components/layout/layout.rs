use yew::prelude::*;
use super::navbar::Navbar;
use super::sidebar::Sidebar;
use super::breadcrumb::Breadcrumb;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub children: Children,
    pub title: String,
    #[prop_or_default]
    pub on_refresh: Option<Callback<()>>,
}

#[function_component(Layout)]
pub fn layout(props: &Props) -> Html {
    html! {
        <div class="drawer lg:drawer-open">
            <input id="my-drawer" type="checkbox" class="drawer-toggle" />
            <div class="drawer-content flex flex-col min-h-screen bg-base-200 transition-all duration-300">
                <Navbar title={props.title.clone()} on_refresh={props.on_refresh.clone()} />
                <Breadcrumb />
                <main class="flex-1 p-6 overflow-y-auto">
                    { for props.children.iter() }
                </main>
            </div>
            <Sidebar />
        </div>
    }
}

use yew::prelude::*;
use web_sys::window;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "
export function get_system_theme() {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
        return 'dark';
    }
    return 'light';
}
")]
extern "C" {
    fn get_system_theme() -> String;
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub title: String,
    #[prop_or_default]
    pub on_refresh: Option<Callback<()>>,
}

#[function_component(Navbar)]
pub fn navbar(props: &Props) -> Html {
    let theme = use_state(|| {
        // Check localStorage for saved theme, otherwise check system preference
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(saved_theme)) = storage.get_item("theme") {
                    return saved_theme;
                }
            }
        }
        // Use system preference as default
        get_system_theme()
    });

    // Set initial theme on mount
    use_effect_with((), {
        let theme = theme.clone();
        move |_| {
            if let Some(window) = window() {
                if let Some(document) = window.document() {
                    if let Some(html) = document.document_element() {
                        let _ = html.set_attribute("data-theme", &*theme);
                    }
                }
            }
            || ()
        }
    });

    let on_theme_toggle = {
        let theme = theme.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                let new_theme = if input.checked() { "dark" } else { "light" };
                theme.set(new_theme.to_string());

                // Update HTML attribute
                if let Some(window) = window() {
                    if let Some(document) = window.document() {
                        if let Some(html) = document.document_element() {
                            let _ = html.set_attribute("data-theme", new_theme);
                        }
                    }
                    // Save to localStorage
                    if let Ok(Some(storage)) = window.local_storage() {
                        let _ = storage.set_item("theme", new_theme);
                    }
                }
            }
        })
    };

    let on_refresh_click = {
        let on_refresh = props.on_refresh.clone();
        Callback::from(move |_| {
            if let Some(callback) = &on_refresh {
                callback.emit(());
            }
        })
    };

    let is_dark = *theme == "dark";
    let has_refresh = props.on_refresh.is_some();

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
                if has_refresh {
                    <button
                        class="btn btn-ghost btn-circle"
                        onclick={on_refresh_click}
                        title="Refresh data"
                    >
                        <i class="fas fa-sync-alt text-xl"></i>
                    </button>
                }
                <label class="swap swap-rotate btn btn-ghost btn-circle">
                    <input
                        id="theme-toggle"
                        type="checkbox"
                        checked={is_dark}
                        onchange={on_theme_toggle}
                    />
                    <i class="swap-on fill-current fas fa-sun text-xl"></i>
                    <i class="swap-off fill-current fas fa-moon text-xl"></i>
                </label>
            </div>
        </div>
    }
}

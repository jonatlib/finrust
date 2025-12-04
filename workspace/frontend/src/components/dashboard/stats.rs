use yew::prelude::*;

#[function_component(Stats)]
pub fn stats() -> Html {
    html! {
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Net Worth"}</div>
                    <div class="stat-value text-primary">{"$1,250,000.00"}</div>
                    <div class="stat-desc">{"Included accounts only"}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Income (30d)"}</div>
                    <div class="stat-value text-success">{"$12,450.00"}</div>
                </div>
            </div>
            <div class="stats shadow bg-base-100">
                <div class="stat">
                    <div class="stat-title">{"Expenses (30d)"}</div>
                    <div class="stat-value text-error">{"$4,320.00"}</div>
                </div>
            </div>
        </div>
    }
}

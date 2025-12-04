use yew::prelude::*;

#[function_component(Settings)]
pub fn settings() -> Html {
    html! {
        <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
            <div class="card bg-base-100 shadow">
                <div class="card-body">
                    <h2 class="card-title">{"Connection Settings"}</h2>
                    <div class="form-control w-full mt-4">
                        <label class="label"><span class="label-text">{"API Base URL"}</span></label>
                        <input type="text" id="api-url-input" placeholder="Empty = Demo Mode" class="input input-bordered w-full" />
                    </div>
                    <div class="card-actions justify-end mt-4">
                        <button class="btn btn-primary">{"Save & Reload"}</button>
                    </div>
                </div>
            </div>
        </div>
    }
}

use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::dashboard::Dashboard;
use crate::components::layout::layout::Layout;

#[function_component(DashboardPage)]
pub fn dashboard_page() -> Html {
    let refresh_trigger = use_state(|| 0u32);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            let refresh_trigger = refresh_trigger.clone();
            spawn_local(async move {
                let _ = crate::api_client::post::<serde_json::Value, ()>(
                    "/api/v1/cache/flush",
                    &(),
                )
                .await;
                log::debug!("Cache flushed, refreshing dashboard");
                refresh_trigger.set(*refresh_trigger + 1);
            });
        })
    };

    html! {
        <Layout title="Dashboard" on_refresh={Some(on_refresh)}>
            <Dashboard key={*refresh_trigger} />
        </Layout>
    }
}

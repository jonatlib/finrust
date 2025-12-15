use yew::prelude::*;

use crate::components::scenarios::Scenarios;
use crate::components::layout::layout::Layout;

#[function_component(ScenariosPage)]
pub fn scenarios_page() -> Html {
    let refresh_trigger = use_state(|| 0);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            log::debug!("Scenarios page refresh triggered");
            refresh_trigger.set(*refresh_trigger + 1);
        })
    };

    html! {
        <Layout title="What-If Scenarios" on_refresh={Some(on_refresh)}>
            <Scenarios key={*refresh_trigger} />
        </Layout>
    }
}

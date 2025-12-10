use yew::prelude::*;

use crate::components::instances::Instances;
use crate::components::layout::layout::Layout;

#[function_component(InstancesPage)]
pub fn instances_page() -> Html {
    let refresh_trigger = use_state(|| 0);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            log::debug!("Instances page refresh triggered");
            refresh_trigger.set(*refresh_trigger + 1);
        })
    };

    html! {
        <Layout title="Transaction Instances" on_refresh={Some(on_refresh)}>
            <Instances key={*refresh_trigger} />
        </Layout>
    }
}

use yew::prelude::*;

use crate::components::manual_states::ManualStates;
use crate::components::layout::layout::Layout;

#[function_component(ManualStatesPage)]
pub fn manual_states_page() -> Html {
    let refresh_trigger = use_state(|| 0);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            log::debug!("Manual states page refresh triggered");
            refresh_trigger.set(*refresh_trigger + 1);
        })
    };

    html! {
        <Layout title="Account Balances" on_refresh={Some(on_refresh)}>
            <ManualStates key={*refresh_trigger} />
        </Layout>
    }
}

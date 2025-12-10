use yew::prelude::*;

use crate::components::recurring::Recurring;
use crate::components::layout::layout::Layout;

#[function_component(RecurringPage)]
pub fn recurring_page() -> Html {
    let refresh_trigger = use_state(|| 0);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            log::debug!("Recurring page refresh triggered");
            refresh_trigger.set(*refresh_trigger + 1);
        })
    };

    html! {
        <Layout title="Recurring Transactions" on_refresh={Some(on_refresh)}>
            <Recurring key={*refresh_trigger} />
        </Layout>
    }
}

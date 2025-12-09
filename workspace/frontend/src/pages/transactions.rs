use yew::prelude::*;

use crate::components::layout::layout::Layout;
use crate::components::transactions::Transactions;

#[function_component(TransactionsPage)]
pub fn transactions_page() -> Html {
    let refresh_trigger = use_state(|| 0);

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            log::debug!("Transactions page refresh triggered");
            refresh_trigger.set(*refresh_trigger + 1);
        })
    };

    html! {
        <Layout title="Transactions" on_refresh={Some(on_refresh)}>
            <Transactions key={*refresh_trigger} />
        </Layout>
    }
}

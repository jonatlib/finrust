use yew::prelude::*;

use crate::components::layout::layout::Layout;
use crate::components::transactions::TransactionEdit;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub transaction_id: i32,
}

#[function_component(TransactionEditPage)]
pub fn transaction_edit_page(props: &Props) -> Html {
    html! {
        <Layout title="Edit Transaction" on_refresh={Option::<Callback<()>>::None}>
            <TransactionEdit transaction_id={props.transaction_id} />
        </Layout>
    }
}

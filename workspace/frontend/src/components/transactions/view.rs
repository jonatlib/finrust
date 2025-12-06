use yew::prelude::*;
use super::table::TransactionTable;
use crate::components::modals::{TransactionModal, TransactionFormData};
use crate::mock_data::{get_mock_categories, get_mock_accounts};

#[function_component(Transactions)]
pub fn transactions() -> Html {
    let show_modal = use_state(|| false);
    let categories = get_mock_categories();
    let accounts = get_mock_accounts();

    let on_add_click = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| show_modal.set(true))
    };

    let on_modal_close = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| show_modal.set(false))
    };

    let on_modal_submit = {
        let show_modal = show_modal.clone();
        Callback::from(move |data: TransactionFormData| {
            web_sys::console::log_1(&format!("Transaction submitted: {:?}", data.description).into());
            show_modal.set(false);
        })
    };

    html! {
        <>
            <div class="flex justify-between items-center mb-4">
               <div class="join">
                   <input class="input input-bordered join-item" placeholder="Search transactions..." />
                   <button class="btn join-item">{"Search"}</button>
               </div>
               <button class="btn btn-primary" onclick={on_add_click}>
                   <i class="fas fa-plus"></i> {" Add Transaction"}
               </button>
            </div>
            <TransactionTable />
            <TransactionModal
                show={*show_modal}
                categories={categories}
                accounts={accounts}
                on_close={on_modal_close}
                on_submit={on_modal_submit}
            />
        </>
    }
}

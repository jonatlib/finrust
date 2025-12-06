use yew::prelude::*;
use super::list::RecurringList;
use crate::components::modals::{RecurringModal, RecurringFormData};
use crate::mock_data::{get_mock_categories, get_mock_accounts};

#[function_component(Recurring)]
pub fn recurring() -> Html {
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
        Callback::from(move |data: RecurringFormData| {
            web_sys::console::log_1(&format!("Recurring rule submitted: {:?}", data.name).into());
            show_modal.set(false);
        })
    };

    html! {
        <>
            <div class="flex justify-end mb-4">
                <button class="btn btn-primary" onclick={on_add_click}>
                    <i class="fas fa-plus"></i> {" Add Recurring Item"}
                </button>
            </div>
            <RecurringList />
            <RecurringModal
                show={*show_modal}
                categories={categories}
                accounts={accounts}
                rule={None}
                on_close={on_modal_close}
                on_submit={on_modal_submit}
            />
        </>
    }
}

use yew::prelude::*;
use super::list::RecurringList;

#[function_component(Recurring)]
pub fn recurring() -> Html {
    html! {
        <>
            <div class="flex justify-end mb-4">
                <button class="btn btn-primary">
                    <i class="fas fa-plus"></i> {"Add Recurring Item"}
                </button>
            </div>
            <RecurringList />
        </>
    }
}

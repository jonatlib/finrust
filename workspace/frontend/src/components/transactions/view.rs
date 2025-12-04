use yew::prelude::*;
use super::table::TransactionTable;

#[function_component(Transactions)]
pub fn transactions() -> Html {
    html! {
        <>
            <div class="flex justify-between items-center mb-4">
               <div class="join">
                   <input class="input input-bordered join-item" placeholder="Search transactions..." />
                   <button class="btn join-item">{"Search"}</button>
               </div>
               <button class="btn btn-primary">
                   <i class="fas fa-plus"></i> {"Add Transaction"}
               </button>
            </div>
            <TransactionTable />
        </>
    }
}

use yew::prelude::*;
use super::progress::BudgetProgress;

#[function_component(Budgets)]
pub fn budgets() -> Html {
    html! {
         <div class="card bg-base-100 shadow">
            <div class="card-body">
                <h2 class="card-title mb-4">{"Monthly Budgets"}</h2>
                <BudgetProgress category="Groceries" spent={350.0} limit={500.0} />
                <BudgetProgress category="Entertainment" spent={120.0} limit={150.0} />
                <BudgetProgress category="Dining Out" spent={180.0} limit={200.0} />
                <BudgetProgress category="Transport" spent={45.0} limit={100.0} />
                
                <div class="card-actions justify-end mt-6">
                     <button class="btn btn-primary">{"Edit Budgets"}</button>
                </div>
            </div>
         </div>
    }
}

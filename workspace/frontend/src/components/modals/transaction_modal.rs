use yew::prelude::*;
use crate::mock_data::{Category, AccountBalance};

#[derive(Properties, PartialEq)]
pub struct TransactionModalProps {
    pub show: bool,
    pub categories: Vec<Category>,
    pub accounts: Vec<AccountBalance>,
    pub on_close: Callback<()>,
    pub on_submit: Callback<TransactionFormData>,
}

#[derive(Clone, PartialEq)]
pub struct TransactionFormData {
    pub date: String,
    pub txn_type: String,
    pub description: String,
    pub amount: f64,
    pub category_id: String,
    pub account_id: i32,
}

#[function_component(TransactionModal)]
pub fn transaction_modal(props: &TransactionModalProps) -> Html {
    let form_ref = use_node_ref();

    let on_submit = {
        let on_submit = props.on_submit.clone();
        let form_ref = form_ref.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let date = form_data.get("date").as_string().unwrap_or_default();
                let txn_type = form_data.get("type").as_string().unwrap_or("expense".to_string());
                let description = form_data.get("description").as_string().unwrap_or_default();
                let amount_str = form_data.get("amount").as_string().unwrap_or("0".to_string());
                let category_id = form_data.get("category_id").as_string().unwrap_or_default();
                let account_id_str = form_data.get("account_id").as_string().unwrap_or("1".to_string());

                let mut amount = amount_str.parse::<f64>().unwrap_or(0.0);
                if txn_type == "expense" {
                    amount = -amount.abs();
                }

                let account_id = account_id_str.parse::<i32>().unwrap_or(1);

                on_submit.emit(TransactionFormData {
                    date,
                    txn_type,
                    description,
                    amount,
                    category_id,
                    account_id,
                });
            }
        })
    };

    let on_close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="transaction_modal">
            <div class="modal-box w-11/12 max-w-2xl">
                <h3 class="font-bold text-lg">{"Add Transaction"}</h3>
                <form ref={form_ref} onsubmit={on_submit} class="py-4 space-y-4">
                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Date"}</span></label>
                            <input type="date" name="date" class="input input-bordered w-full" value={today} required={true} />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Type"}</span></label>
                            <select name="type" class="select select-bordered w-full">
                                <option value="expense">{"Expense"}</option>
                                <option value="income">{"Income"}</option>
                            </select>
                        </div>
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Description"}</span></label>
                        <input type="text" name="description" class="input input-bordered w-full" placeholder="e.g. Grocery Store" required={true} />
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Amount"}</span></label>
                            <input type="number" step="0.01" name="amount" class="input input-bordered w-full" placeholder="0.00" required={true} />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Category"}</span></label>
                            <select name="category_id" class="select select-bordered w-full">
                                { for props.categories.iter().map(|c| html! {
                                    <option value={c.id.clone()}>{&c.name}</option>
                                })}
                            </select>
                        </div>
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Account"}</span></label>
                        <select name="account_id" class="select select-bordered w-full">
                            { for props.accounts.iter().map(|a| html! {
                                <option value={a.id.to_string()}>{&a.name}</option>
                            })}
                        </select>
                    </div>

                    <div class="modal-action">
                        <button type="button" class="btn" onclick={on_close.clone()}>{"Cancel"}</button>
                        <button type="submit" class="btn btn-primary">{"Save Transaction"}</button>
                    </div>
                </form>
            </div>
            <form class="modal-backdrop" method="dialog">
                <button onclick={on_close}>{"close"}</button>
            </form>
        </dialog>
    }
}

use yew::prelude::*;
use crate::mock_data::{Category, AccountBalance, RecurringRule};

#[derive(Properties, PartialEq)]
pub struct RecurringModalProps {
    pub show: bool,
    pub categories: Vec<Category>,
    pub accounts: Vec<AccountBalance>,
    pub rule: Option<RecurringRule>,
    pub on_close: Callback<()>,
    pub on_submit: Callback<RecurringFormData>,
}

#[derive(Clone, PartialEq)]
pub struct RecurringFormData {
    pub name: String,
    pub amount: f64,
    pub frequency: String,
    pub next_date: String,
    pub end_date: Option<String>,
    pub category_id: String,
    pub account_id: i32,
}

#[function_component(RecurringModal)]
pub fn recurring_modal(props: &RecurringModalProps) -> Html {
    let form_ref = use_node_ref();
    let is_edit = props.rule.is_some();

    let on_submit = {
        let on_submit = props.on_submit.clone();
        let form_ref = form_ref.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if let Some(form) = form_ref.cast::<web_sys::HtmlFormElement>() {
                let form_data = web_sys::FormData::new_with_form(&form).unwrap();

                let name = form_data.get("name").as_string().unwrap_or_default();
                let amount = form_data.get("amount").as_string().unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
                let frequency = form_data.get("frequency").as_string().unwrap_or("Monthly".to_string());
                let next_date = form_data.get("next_date").as_string().unwrap_or_default();
                let end_date = form_data.get("end_date").as_string().filter(|s| !s.is_empty());
                let category_id = form_data.get("category_id").as_string().unwrap_or_default();
                let account_id = form_data.get("account_id").as_string().unwrap_or("1".to_string()).parse::<i32>().unwrap_or(1);

                on_submit.emit(RecurringFormData {
                    name,
                    amount,
                    frequency,
                    next_date,
                    end_date,
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
    let (name, amount, frequency, next_date, end_date, category_id, account_id) = if let Some(rule) = &props.rule {
        (rule.name.clone(), rule.amount.to_string(), rule.frequency.clone(),
         rule.next_date.clone(), rule.end_date.clone().unwrap_or_default(),
         rule.category_id.clone(), rule.account_id)
    } else {
        (String::new(), String::new(), "Monthly".to_string(), today.clone(), String::new(),
         props.categories.first().map(|c| c.id.clone()).unwrap_or_default(),
         props.accounts.first().map(|a| a.id).unwrap_or(1))
    };

    html! {
        <dialog class={classes!("modal", props.show.then_some("modal-open"))} id="recurring_modal">
            <div class="modal-box w-11/12 max-w-2xl">
                <h3 class="font-bold text-lg">{if is_edit { "Edit Recurring Rule" } else { "New Recurring Rule" }}</h3>
                <form ref={form_ref} onsubmit={on_submit} class="py-4 space-y-4">
                    <div class="form-control">
                        <label class="label"><span class="label-text">{"Rule Name"}</span></label>
                        <input type="text" name="name" class="input input-bordered w-full" placeholder="e.g. Rent" value={name} required={true} />
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Amount"}</span></label>
                            <input type="number" step="0.01" name="amount" class="input input-bordered w-full" placeholder="-100.00" value={amount} required={true} />
                            <label class="label"><span class="label-text-alt">{"Negative for expenses"}</span></label>
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Frequency"}</span></label>
                            <select name="frequency" class="select select-bordered w-full">
                                { for ["Monthly", "Weekly", "Bi-Weekly", "Yearly"].iter().map(|f| html! {
                                    <option value={*f} selected={&frequency == f}>{f}</option>
                                })}
                            </select>
                        </div>
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Next Due Date"}</span></label>
                            <input type="date" name="next_date" class="input input-bordered w-full" value={next_date} required={true} />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"End Date (Optional)"}</span></label>
                            <input type="date" name="end_date" class="input input-bordered w-full" value={end_date} />
                            <label class="label"><span class="label-text-alt">{"Leave empty if indefinite"}</span></label>
                        </div>
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Category"}</span></label>
                            <select name="category_id" class="select select-bordered w-full">
                                { for props.categories.iter().map(|c| html! {
                                    <option value={c.id.clone()} selected={&category_id == &c.id}>{&c.name}</option>
                                })}
                            </select>
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text">{"Account"}</span></label>
                            <select name="account_id" class="select select-bordered w-full">
                                { for props.accounts.iter().map(|a| html! {
                                    <option value={a.id.to_string()} selected={account_id == a.id}>{&a.name}</option>
                                })}
                            </select>
                        </div>
                    </div>

                    <div class="modal-action">
                        <button type="button" class="btn" onclick={on_close.clone()}>{"Cancel"}</button>
                        <button type="submit" class="btn btn-primary">{if is_edit { "Update Rule" } else { "Save Rule" }}</button>
                    </div>
                </form>
            </div>
            <form class="modal-backdrop" method="dialog">
                <button onclick={on_close}>{"close"}</button>
            </form>
        </dialog>
    }
}

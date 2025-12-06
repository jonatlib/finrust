use yew::prelude::*;
use crate::mock_data::{get_mock_recurring, get_mock_categories};
use std::collections::HashMap;

#[function_component(RecurringList)]
pub fn recurring_list() -> Html {
    let recurring = get_mock_recurring();
    let categories = get_mock_categories();

    let cat_map: HashMap<String, String> = categories.iter()
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();

    let format_currency = |amount: f64| -> String {
        format!("${:.2}", amount.abs())
    };

    html! {
        <div class="overflow-x-auto bg-base-100 shadow rounded-box">
            <table class="table table-zebra">
                <thead>
                    <tr>
                        <th>{"Rule Name"}</th>
                        <th>{"Frequency"}</th>
                        <th>{"Category"}</th>
                        <th>{"Next Due"}</th>
                        <th>{"Amount"}</th>
                        <th>{"Actions"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for recurring.iter().map(|r| {
                        let category_name = cat_map.get(&r.category_id).cloned().unwrap_or_else(|| "Uncategorized".to_string());
                        let amount_class = if r.amount >= 0.0 { "text-success" } else { "text-error" };
                        let is_overdue = r.next_date < chrono::Local::now().format("%Y-%m-%d").to_string();

                        html! {
                            <tr>
                                <td class="font-bold">
                                    {&r.name}
                                    <div class="text-xs font-normal opacity-50">
                                        {if r.active { "Active" } else { "Paused" }}
                                    </div>
                                </td>
                                <td>{&r.frequency}</td>
                                <td><span class="badge badge-sm badge-ghost">{category_name}</span></td>
                                <td class={if is_overdue { "text-error font-bold" } else { "" }}>
                                    {&r.next_date}
                                </td>
                                <td class={classes!("font-mono", amount_class)}>
                                    {if r.amount >= 0.0 { format!("+{}", format_currency(r.amount)) } else { format!("-{}", format_currency(r.amount)) }}
                                </td>
                                <td>
                                    <div class="flex gap-2">
                                        <button class="btn btn-sm btn-ghost btn-square" title="Edit">
                                            <i class="fas fa-edit"></i>
                                        </button>
                                        <button class="btn btn-sm btn-success btn-outline gap-2">
                                            <i class="fas fa-check"></i> {"Pay"}
                                        </button>
                                    </div>
                                </td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </div>
    }
}

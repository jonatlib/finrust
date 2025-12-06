use yew::prelude::*;
use crate::mock_data::{get_mock_transactions, get_mock_categories, get_mock_accounts};
use std::collections::HashMap;

#[function_component(TransactionTable)]
pub fn transaction_table() -> Html {
    let transactions = get_mock_transactions();
    let categories = get_mock_categories();
    let accounts = get_mock_accounts();

    let cat_map: HashMap<String, String> = categories.iter()
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();

    let acc_map: HashMap<i32, String> = accounts.iter()
        .map(|a| (a.id, a.name.clone()))
        .collect();

    let format_currency = |amount: f64| -> String {
        format!("${:.2}", amount.abs())
    };

    html! {
        <div class="overflow-x-auto bg-base-100 shadow rounded-box">
            <table class="table table-zebra">
                <thead>
                    <tr>
                        <th>{"Date"}</th>
                        <th>{"Description"}</th>
                        <th>{"Category"}</th>
                        <th>{"Account"}</th>
                        <th>{"Amount"}</th>
                        <th>{"Status"}</th>
                        <th>{"Actions"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for transactions.iter().take(20).map(|t| {
                        let category_name = cat_map.get(&t.category_id).cloned().unwrap_or_else(|| "Uncategorized".to_string());
                        let account_name = acc_map.get(&t.account_id).cloned().unwrap_or_else(|| "Unknown".to_string());
                        let amount_class = if t.amount >= 0.0 { "text-success" } else { "text-error" };
                        let status_badge = if t.status == "cleared" { "badge-success" } else { "badge-warning" };

                        html! {
                            <tr class="hover">
                                <td class="whitespace-nowrap">{&t.date}</td>
                                <td class="font-medium">{&t.description}</td>
                                <td><span class="badge badge-sm badge-ghost">{category_name}</span></td>
                                <td>{account_name}</td>
                                <td class={classes!("font-mono", "text-right", "font-bold", amount_class)}>
                                    {if t.amount >= 0.0 { format!("+{}", format_currency(t.amount)) } else { format!("-{}", format_currency(t.amount)) }}
                                </td>
                                <td><span class={classes!("badge", "badge-xs", status_badge)}></span> {" "}{&t.status}</td>
                                <td>
                                    <button class="btn btn-ghost btn-xs"><i class="fas fa-edit"></i></button>
                                </td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </div>
    }
}

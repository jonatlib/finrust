use yew::prelude::*;
use crate::mock_data::{get_mock_budgets, get_mock_categories, get_mock_transactions};
use std::collections::HashMap;
use chrono::{Datelike, Local};

#[function_component(Budgets)]
pub fn budgets() -> Html {
    let budgets = get_mock_budgets();
    let categories = get_mock_categories();
    let transactions = get_mock_transactions();

    let cat_map: HashMap<String, (String, String)> = categories.iter()
        .map(|c| (c.id.clone(), (c.name.clone(), c.color.clone())))
        .collect();

    let current_month = Local::now().month();

    // Calculate actual spending per category for current month
    let mut spending: HashMap<String, f64> = HashMap::new();
    for txn in transactions.iter() {
        if txn.amount < 0.0 {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&txn.date, "%Y-%m-%d") {
                if date.month() == current_month {
                    *spending.entry(txn.category_id.clone()).or_insert(0.0) += txn.amount.abs();
                }
            }
        }
    }

    html! {
        <>
            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Monthly Budget"}</h2>
                <button class="btn btn-primary btn-sm"><i class="fas fa-plus"></i> {" Edit Allocations"}</button>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                { for budgets.iter().map(|b| {
                    let (cat_name, cat_color) = cat_map.get(&b.category_id).cloned().unwrap_or_else(|| ("Unknown".to_string(), "#ccc".to_string()));
                    let actual = *spending.get(&b.category_id).unwrap_or(&0.0);
                    let percent = ((actual / b.amount) * 100.0).min(100.0);
                    let color_class = if percent > 90.0 { "progress-error" } else if percent > 75.0 { "progress-warning" } else { "progress-primary" };
                    let remaining = b.amount - actual;

                    html! {
                        <div class="card bg-base-100 shadow">
                            <div class="card-body p-5">
                                <div class="flex justify-between mb-2">
                                    <span class="font-bold" style={format!("color: {}", cat_color)}>{cat_name}</span>
                                    <span class="text-sm">{format!("${:.2} / ${:.2}", actual, b.amount)}</span>
                                </div>
                                <progress class={classes!("progress", "w-full", color_class)} value={percent.to_string()} max="100"></progress>
                                <div class="text-right text-xs mt-1 opacity-70">{format!("${:.2} remaining", remaining)}</div>
                            </div>
                        </div>
                    }
                })}
            </div>
        </>
    }
}

use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BudgetProps {
    pub category: String,
    pub spent: f64,
    pub limit: f64,
}

#[function_component(BudgetProgress)]
pub fn budget_progress(props: &BudgetProps) -> Html {
    let percentage = if props.limit > 0.0 {
        (props.spent / props.limit * 100.0).min(100.0)
    } else {
        0.0
    };
    
    let color_class = if percentage > 90.0 {
        "progress-error"
    } else if percentage > 75.0 {
        "progress-warning"
    } else {
        "progress-primary"
    };

    html! {
        <div class="mb-4">
            <div class="flex justify-between mb-1">
                <span class="text-sm font-medium">{&props.category}</span>
                <span class="text-sm text-gray-500">{format!("${:.2} / ${:.2}", props.spent, props.limit)}</span>
            </div>
            <progress class={classes!("progress", "w-full", color_class)} value={percentage.to_string()} max="100"></progress>
        </div>
    }
}

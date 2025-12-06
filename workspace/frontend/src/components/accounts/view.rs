use yew::prelude::*;
use crate::mock_data::get_mock_accounts;

#[function_component(Accounts)]
pub fn accounts() -> Html {
    let accounts = get_mock_accounts();

    let format_currency = |amount: f64| -> String {
        format!("${:.2}", amount)
    };

    html! {
        <>
            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Accounts"}</h2>
                <button class="btn btn-primary btn-sm"><i class="fas fa-plus"></i> {" Add Account"}</button>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                { for accounts.iter().map(|acc| {
                    let border_class = if acc.include_in_overview { "border-primary" } else { "border-base-300" };
                    let balance_class = if acc.current_balance < 0.0 { "text-error" } else { "" };

                    html! {
                        <div class={classes!("card", "bg-base-100", "shadow", "border-l-4", border_class)}>
                            <div class="card-body">
                                <div class="flex justify-between items-start">
                                    <div>
                                        <h3 class="card-title text-base">{&acc.name}</h3>
                                        <p class="text-sm opacity-70">{&acc.institution}{" • "}{&acc.account_type}</p>
                                    </div>
                                    {if acc.include_in_overview {
                                        html! { <div class="badge badge-primary badge-outline badge-sm" title="Included in Overview"><i class="fas fa-eye"></i></div> }
                                    } else {
                                        html! { <div class="badge badge-ghost badge-sm" title="Excluded from Overview"><i class="fas fa-eye-slash"></i></div> }
                                    }}
                                </div>
                                <div class={classes!("text-2xl", "font-bold", "mt-4", "font-mono", balance_class)}>
                                    {format_currency(acc.current_balance)}
                                </div>
                            </div>
                        </div>
                    }
                })}
            </div>
        </>
    }
}

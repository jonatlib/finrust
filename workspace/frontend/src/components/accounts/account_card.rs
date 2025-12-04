use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub name: String,
    pub balance: String,
    pub currency: String,
    pub kind: String,
    pub apy: String,
}

#[function_component(AccountCard)]
pub fn account_card(props: &Props) -> Html {
    html! {
        <div class="card bg-base-100 shadow hover:shadow-md transition-shadow cursor-pointer">
            <div class="card-body">
                <div class="flex justify-between items-start">
                    <div>
                        <h3 class="card-title text-base">{&props.name}</h3>
                        <span class="badge badge-ghost badge-sm mt-1">{&props.kind}</span>
                    </div>
                    <div class="text-right">
                        <div class={classes!("text-2xl", "font-bold", if props.balance.starts_with('-') { "text-error" } else { "text-success" })}>
                            {&props.balance}
                        </div>
                        <div class="text-xs text-gray-500">{&props.currency}</div>
                    </div>
                </div>
                {
                    if !props.apy.is_empty() {
                        html! {
                            <div class="mt-4 flex justify-between items-center">
                                <div class="text-xs text-gray-500">{"APY"}</div>
                                <div class="badge badge-secondary badge-outline badge-sm">{&props.apy}</div>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
                <div class="card-actions justify-end mt-4">
                   <button class="btn btn-sm btn-ghost">{"Edit"}</button>
                   <button class="btn btn-sm btn-ghost">{"View"}</button>
                </div>
            </div>
        </div>
    }
}

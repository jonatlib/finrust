use yew::prelude::*;
use crate::api_client::account::AccountResponse;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account: AccountResponse,
}

#[function_component(AccountCard)]
pub fn account_card(props: &Props) -> Html {
    let account = &props.account;

    html! {
        <div class="card bg-base-100 shadow hover:shadow-md transition-shadow cursor-pointer">
            <div class="card-body">
                <div class="flex justify-between items-start">
                    <div>
                        <h3 class="card-title text-base">{&account.name}</h3>
                        {if let Some(desc) = &account.description {
                            html! { <p class="text-xs text-gray-500 mt-1">{desc}</p> }
                        } else {
                            html! {}
                        }}
                    </div>
                    {if account.include_in_statistics {
                        html! { <div class="badge badge-primary badge-outline badge-sm" title="Included in Statistics"><i class="fas fa-chart-line"></i></div> }
                    } else {
                        html! { <div class="badge badge-ghost badge-sm" title="Excluded from Statistics"><i class="fas fa-eye-slash"></i></div> }
                    }}
                </div>
                <div class="mt-4">
                    <div class="text-xs text-gray-500">{"Currency"}</div>
                    <div class="badge badge-secondary badge-outline badge-sm mt-1">{&account.currency_code}</div>
                </div>
                {if let Some(ledger) = &account.ledger_name {
                    html! {
                        <div class="mt-2">
                            <div class="text-xs text-gray-500">{"Ledger"}</div>
                            <div class="text-sm mt-1">{ledger}</div>
                        </div>
                    }
                } else {
                    html! {}
                }}
                <div class="card-actions justify-end mt-4">
                   <button class="btn btn-sm btn-ghost">{"Edit"}</button>
                   <button class="btn btn-sm btn-ghost">{"View"}</button>
                </div>
            </div>
        </div>
    }
}

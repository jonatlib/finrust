use yew::prelude::*;
use crate::api_client::manual_account_state::{get_account_manual_states, ManualAccountStateResponse};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub account_id: i32,
}

#[function_component(ManualStatesAccountView)]
pub fn manual_states_account_view(props: &Props) -> Html {
    let account_id = props.account_id;
    let (fetch_state, _refetch) = use_fetch_with_refetch(move || get_account_manual_states(account_id));

    html! {
        <div class="card bg-base-100 shadow mt-6">
            <div class="card-body">
                <h3 class="card-title text-lg">{"Manual Account States"}</h3>
                <p class="text-sm text-gray-500 mb-4">
                    {"Initial balances recorded for this account. These are used to calculate statistics."}
                </p>

                {match &*fetch_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-error">
                            <span>{error}</span>
                        </div>
                    },
                    FetchState::Success(states) => {
                        if states.is_empty() {
                            html! {
                                <div class="text-center py-8 text-gray-500">
                                    <i class="fas fa-balance-scale text-4xl mb-4 opacity-50"></i>
                                    <p>{"No manual account states recorded yet."}</p>
                                    <p class="text-sm mt-2">{"Go to Account Balances page to add initial balances."}</p>
                                </div>
                            }
                        } else {
                            html! {
                                <div class="overflow-x-auto">
                                    <table class="table table-zebra w-full">
                                        <thead>
                                            <tr>
                                                <th>{"Date"}</th>
                                                <th class="text-right">{"Amount"}</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {states.iter().map(|state| {
                                                html! {
                                                    <tr key={state.id}>
                                                        <td>{state.date.to_string()}</td>
                                                        <td class="text-right font-mono">{format_amount(state.amount)}</td>
                                                    </tr>
                                                }
                                            }).collect::<Html>()}
                                        </tbody>
                                    </table>
                                </div>
                            }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }}
            </div>
        </div>
    }
}

fn format_amount(amount: rust_decimal::Decimal) -> String {
    format!("{:.2}", amount)
}

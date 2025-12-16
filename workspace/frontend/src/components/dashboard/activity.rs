use yew::prelude::*;
use rust_decimal::prelude::*;
use crate::api_client::transaction::get_transactions;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

#[function_component(RecentActivity)]
pub fn recent_activity() -> Html {
    let (transactions_state, _) = use_fetch_with_refetch(|| get_transactions(None, None));

    let format_currency = |amount: Decimal| -> String {
        if amount >= Decimal::ZERO {
            format!("+${:.2}", amount)
        } else {
            format!("-${:.2}", amount.abs())
        }
    };

    match &*transactions_state {
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
        FetchState::Success(transactions) => {
            // Sort transactions by date (most recent first) and take the last 10
            let mut sorted_transactions = transactions.clone();
            sorted_transactions.sort_by(|a, b| b.date.cmp(&a.date));
            let recent_transactions: Vec<_> = sorted_transactions.iter().take(10).collect();

            if recent_transactions.is_empty() {
                html! {
                    <div class="text-center py-8">
                        <p class="text-gray-500">{"No recent transactions"}</p>
                    </div>
                }
            } else {
                html! {
                    <div class="overflow-x-auto">
                        <table class="table table-sm">
                            <thead>
                                <tr>
                                    <th>{"Date"}</th>
                                    <th>{"Description"}</th>
                                    <th class="text-right">{"Amount"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    for recent_transactions.iter().map(|transaction| {
                                        let amount_class = if transaction.amount >= Decimal::ZERO {
                                            "text-success"
                                        } else {
                                            "text-error"
                                        };

                                        html! {
                                            <tr key={transaction.id}>
                                                <td>{transaction.date.format("%Y-%m-%d").to_string()}</td>
                                                <td>{&transaction.name}</td>
                                                <td class={classes!("text-right", amount_class)}>
                                                    {format_currency(transaction.amount)}
                                                </td>
                                            </tr>
                                        }
                                    })
                                }
                            </tbody>
                        </table>
                    </div>
                }
            }
        },
        FetchState::NotStarted => html! { <></> },
    }
}

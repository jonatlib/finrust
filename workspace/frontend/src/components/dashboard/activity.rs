use yew::prelude::*;
use rust_decimal::prelude::*;
use chrono::Local;
use crate::api_client::transaction::{get_transactions, TransactionFilters};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::formatting::{fmt_amount, use_currency};
use crate::hooks::FetchState;

#[function_component(RecentActivity)]
pub fn recent_activity() -> Html {
    let (transactions_state, _) = use_fetch_with_refetch(|| {
        let filters = TransactionFilters::default();
        async move { get_transactions(None, None, &filters).await }
    });
    let currency = use_currency();

    let format_currency = {
        let currency = currency.clone();
        move |amount: Decimal| -> String {
            if amount >= Decimal::ZERO {
                format!("+{} {}", fmt_amount(amount), currency)
            } else {
                format!("-{} {}", fmt_amount(amount.abs()), currency)
            }
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
            let today = Local::now().date_naive();
            let mut sorted_transactions: Vec<_> = transactions.iter()
                .filter(|t| !t.is_simulated && t.scenario_id.is_none() && t.date <= today)
                .cloned()
                .collect();
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

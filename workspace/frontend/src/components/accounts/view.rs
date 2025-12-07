use yew::prelude::*;
use crate::api_client::account::{get_accounts, AccountResponse, AccountKind};
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;
use super::account_card::AccountCard;
use super::account_modal::AccountModal;
use std::collections::BTreeMap;

#[function_component(Accounts)]
pub fn accounts() -> Html {
    log::trace!("Accounts component rendering");
    let (fetch_state, refetch) = use_fetch_with_refetch(get_accounts);
    let show_modal = use_state(|| false);

    log::debug!("Accounts component state: loading={}, success={}, error={}",
        fetch_state.is_loading(), fetch_state.is_success(), fetch_state.is_error());

    // Group accounts by kind
    let grouped_accounts: Option<BTreeMap<AccountKind, Vec<AccountResponse>>> = match &*fetch_state {
        FetchState::Success(accounts) => {
            let mut groups: BTreeMap<AccountKind, Vec<AccountResponse>> = BTreeMap::new();
            for account in accounts {
                groups.entry(account.account_kind)
                    .or_insert_with(Vec::new)
                    .push(account.clone());
            }
            Some(groups)
        },
        _ => None,
    };

    let on_open_modal = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| {
            log::info!("Opening Add Account modal");
            show_modal.set(true);
        })
    };

    let on_close_modal = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| {
            log::info!("Closing Add Account modal");
            show_modal.set(false);
        })
    };

    let on_success = {
        let refetch = refetch.clone();
        Callback::from(move |_| {
            log::info!("Account created successfully, refetching accounts");
            refetch.emit(());
        })
    };

    html! {
        <>
            <AccountModal
                show={*show_modal}
                on_close={on_close_modal}
                on_success={on_success}
            />

            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Accounts"}</h2>
                <button
                    class="btn btn-primary btn-sm"
                    onclick={on_open_modal}
                >
                    <i class="fas fa-plus"></i> {" Add Account"}
                </button>
            </div>

            {
                match &*fetch_state {
                    FetchState::Loading => html! {
                        <div class="flex justify-center items-center py-8">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    },
                    FetchState::Error(error) => html! {
                        <div class="alert alert-error">
                            <span>{error}</span>
                            <button class="btn btn-sm" onclick={move |_| refetch.emit(())}>
                                {"Retry"}
                            </button>
                        </div>
                    },
                    FetchState::Success(accounts) => {
                        if accounts.is_empty() {
                            html! {
                                <div class="text-center py-8">
                                    <p class="text-gray-500">{"No accounts found. Create your first account to get started!"}</p>
                                </div>
                            }
                        } else if let Some(groups) = grouped_accounts {
                            html! {
                                <div class="space-y-6">
                                    {
                                        // Render groups in the specified order
                                        [AccountKind::RealAccount, AccountKind::Savings, AccountKind::Investment, AccountKind::Debt, AccountKind::Other]
                                            .iter()
                                            .filter_map(|kind| {
                                                groups.get(kind).map(|accounts| {
                                                    html! {
                                                        <div key={kind.display_name()}>
                                                            <h3 class="text-xl font-semibold mb-3">{kind.display_name()}</h3>
                                                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                                                { for accounts.iter().map(|account| {
                                                                    log::trace!("Rendering account card for: {}", account.name);
                                                                    html! { <AccountCard key={account.id} account={account.clone()} /> }
                                                                })}
                                                            </div>
                                                        </div>
                                                    }
                                                })
                                            })
                                            .collect::<Html>()
                                    }
                                </div>
                            }
                        } else {
                            html! { <></> }
                        }
                    },
                    FetchState::NotStarted => html! { <></> },
                }
            }
        </>
    }
}

use yew::prelude::*;
use crate::api_client::account::{get_accounts, AccountResponse};
use super::account_card::AccountCard;

#[function_component(Accounts)]
pub fn accounts() -> Html {
    let accounts = use_state(|| None::<Vec<AccountResponse>>);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);

    {
        let accounts = accounts.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                loading.set(true);
                match get_accounts().await {
                    Ok(data) => {
                        accounts.set(Some(data));
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    html! {
        <>
            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Accounts"}</h2>
                <button class="btn btn-primary btn-sm"><i class="fas fa-plus"></i> {" Add Account"}</button>
            </div>

            {if *loading {
                html! {
                    <div class="flex justify-center items-center py-12">
                        <span class="loading loading-spinner loading-lg"></span>
                    </div>
                }
            } else if let Some(err) = (*error).as_ref() {
                html! {
                    <div class="alert alert-error">
                        <i class="fas fa-exclamation-circle"></i>
                        <span>{format!("Failed to load accounts: {}", err)}</span>
                    </div>
                }
            } else if let Some(accounts_data) = (*accounts).as_ref() {
                if accounts_data.is_empty() {
                    html! {
                        <div class="alert alert-info">
                            <i class="fas fa-info-circle"></i>
                            <span>{"No accounts found. Create your first account to get started!"}</span>
                        </div>
                    }
                } else {
                    html! {
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                            { for accounts_data.iter().map(|account| {
                                html! { <AccountCard account={account.clone()} /> }
                            })}
                        </div>
                    }
                }
            } else {
                html! {}
            }}
        </>
    }
}

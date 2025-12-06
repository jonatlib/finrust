use yew::prelude::*;
use crate::api_client::account::{get_accounts, AccountResponse};
use crate::common::fetch_render::FetchRenderList;
use crate::common::fetch_hook::use_fetch_with_refetch;
use super::account_card::AccountCard;

#[function_component(Accounts)]
pub fn accounts() -> Html {
    let (fetch_state, refetch) = use_fetch_with_refetch(get_accounts);

    let render_item = Callback::from(|account: AccountResponse| {
        html! { <AccountCard account={account} /> }
    });

    html! {
        <>
            <div class="flex justify-between items-center mb-4">
                <h2 class="text-2xl font-bold">{"Accounts"}</h2>
                <button class="btn btn-primary btn-sm"><i class="fas fa-plus"></i> {" Add Account"}</button>
            </div>

            <FetchRenderList<AccountResponse>
                state={(*fetch_state).clone()}
                render_item={render_item}
                on_retry={Some(refetch)}
                empty_message={"No accounts found. Create your first account to get started!".to_string()}
                container_class={"grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4".to_string()}
            />
        </>
    }
}

use yew::prelude::*;
use super::account_card::AccountCard;

#[function_component(Accounts)]
pub fn accounts() -> Html {
    html! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <AccountCard 
                name="Main Checking" 
                balance="$5,430.20" 
                currency="USD" 
                kind="Bank" 
                apy=""
            />
            <AccountCard 
                name="High Yield Savings" 
                balance="$25,000.00" 
                currency="USD" 
                kind="Savings" 
                apy="4.50%"
            />
            <AccountCard 
                name="Credit Card" 
                balance="-$1,250.00" 
                currency="USD" 
                kind="Credit" 
                apy=""
            />
        </div>
    }
}

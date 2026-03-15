use rust_decimal::Decimal;
use yew::prelude::*;

use crate::api_client::account::get_accounts;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

pub fn fmt_amount(amount: Decimal) -> String {
    format!("{:.1}", amount)
}

pub fn fmt_amount_opt(value: Option<Decimal>) -> String {
    match value {
        Some(d) => fmt_amount(d),
        None => "N/A".to_string(),
    }
}

pub fn fmt_amount_f64(amount: f64) -> String {
    format!("{:.1}", amount)
}

pub fn fmt_amount_str(amount: &str) -> String {
    match amount.parse::<f64>() {
        Ok(val) => format!("{:.1}", val.abs()),
        Err(_) => amount.to_string(),
    }
}

/// Yew context providing the primary currency code (from the first account).
#[derive(Clone, PartialEq)]
pub struct CurrencyContext {
    pub code: AttrValue,
}

impl Default for CurrencyContext {
    fn default() -> Self {
        Self {
            code: AttrValue::from(""),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct CurrencyProviderProps {
    pub children: Children,
}

/// Wraps children in a `ContextProvider<CurrencyContext>` populated from the
/// first account's `currency_code`.
#[function_component(CurrencyProvider)]
pub fn currency_provider(props: &CurrencyProviderProps) -> Html {
    let (accounts_state, _) = use_fetch_with_refetch(get_accounts);

    let ctx = match &*accounts_state {
        FetchState::Success(accounts) => {
            let code = accounts
                .first()
                .map(|a| a.currency_code.clone())
                .unwrap_or_default();
            CurrencyContext {
                code: AttrValue::from(code),
            }
        }
        _ => CurrencyContext::default(),
    };

    html! {
        <ContextProvider<CurrencyContext> context={ctx}>
            { for props.children.iter() }
        </ContextProvider<CurrencyContext>>
    }
}

/// Convenience hook: returns the primary currency code (empty string while loading).
#[hook]
pub fn use_currency() -> AttrValue {
    use_context::<CurrencyContext>()
        .map(|c| c.code)
        .unwrap_or_else(|| AttrValue::from(""))
}

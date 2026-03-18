use rust_decimal::Decimal;
use yew::prelude::*;

use crate::api_client::account::get_accounts;
use crate::common::fetch_hook::use_fetch_with_refetch;
use crate::hooks::FetchState;

/// Inserts space as a thousands separator into the integer part of a
/// pre-formatted number string (e.g. `"1234567.8"` → `"1 234 567.8"`).
fn insert_thousands_sep(formatted: &str) -> String {
    let (int_part, dec_part) = match formatted.find('.') {
        Some(pos) => (&formatted[..pos], &formatted[pos..]),
        None => (formatted, ""),
    };
    let negative = int_part.starts_with('-');
    let digits = if negative { &int_part[1..] } else { int_part };

    let separated: String = digits
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if i > 0 && i % 3 == 0 {
                acc.push(' ');
            }
            acc.push(c);
            acc
        })
        .chars()
        .rev()
        .collect();

    if negative {
        format!("-{}{}", separated, dec_part)
    } else {
        format!("{}{}", separated, dec_part)
    }
}

/// Formats a `Decimal` with one decimal place and thousands separators.
pub fn fmt_amount(amount: Decimal) -> String {
    insert_thousands_sep(&format!("{:.1}", amount))
}

/// Formats an optional `Decimal`, returning `"N/A"` for `None`.
pub fn fmt_amount_opt(value: Option<Decimal>) -> String {
    match value {
        Some(d) => fmt_amount(d),
        None => "N/A".to_string(),
    }
}

/// Formats an `f64` with one decimal place and thousands separators.
pub fn fmt_amount_f64(amount: f64) -> String {
    insert_thousands_sep(&format!("{:.1}", amount))
}

/// Parses a string as `f64`, takes absolute value, formats with thousands separators.
pub fn fmt_amount_str(amount: &str) -> String {
    match amount.parse::<f64>() {
        Ok(val) => insert_thousands_sep(&format!("{:.1}", val.abs())),
        Err(_) => amount.to_string(),
    }
}

/// Formats an `f64` with zero decimal places and thousands separators.
pub fn fmt_amount_f64_int(amount: f64) -> String {
    insert_thousands_sep(&format!("{:.0}", amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_separator_basic() {
        assert_eq!(insert_thousands_sep("1234567.8"), "1 234 567.8");
        assert_eq!(insert_thousands_sep("123.4"), "123.4");
        assert_eq!(insert_thousands_sep("1234.5"), "1 234.5");
        assert_eq!(insert_thousands_sep("-9876543.2"), "-9 876 543.2");
        assert_eq!(insert_thousands_sep("0.0"), "0.0");
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

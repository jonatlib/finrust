use yew::prelude::*;
use yew_router::prelude::*;
use crate::Route;
use crate::api_client::account::get_account;
use crate::hooks::FetchState;
use crate::common::fetch_hook::use_fetch_with_refetch;

#[derive(Clone, PartialEq)]
struct BreadcrumbItem {
    label: String,
    route: Route,
}

#[function_component(Breadcrumb)]
pub fn breadcrumb() -> Html {
    let location = use_location();

    let breadcrumb_items = if let Some(route) = location.as_ref().and_then(|loc| {
        Route::recognize(loc.path())
    }) {
        match route {
            Route::Home | Route::Dashboard => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard }
            ],
            Route::Accounts => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Accounts".to_string(), route: Route::Accounts }
            ],
            Route::AccountEdit { id } => {
                return html! { <BreadcrumbWithAccount account_id={id} /> };
            },
            Route::Transactions => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Transactions".to_string(), route: Route::Transactions }
            ],
            Route::TransactionEdit { id } => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Transactions".to_string(), route: Route::Transactions },
                BreadcrumbItem { label: format!("Transaction #{}", id), route: Route::TransactionEdit { id } }
            ],
            Route::ManualStates => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Account Balances".to_string(), route: Route::ManualStates }
            ],
            Route::Recurring => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Recurring".to_string(), route: Route::Recurring }
            ],
            Route::Budgets => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Budgets".to_string(), route: Route::Budgets }
            ],
            Route::Forecast => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Forecast".to_string(), route: Route::Forecast }
            ],
            Route::Reports => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Reports".to_string(), route: Route::Reports }
            ],
            Route::Settings => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "Settings".to_string(), route: Route::Settings }
            ],
            Route::About => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "About".to_string(), route: Route::About }
            ],
            Route::NotFound => vec![
                BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
                BreadcrumbItem { label: "404".to_string(), route: Route::NotFound }
            ],
        }
    } else {
        vec![BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard }]
    };

    html! {
        <div class="breadcrumbs text-sm px-6 py-2 bg-base-100">
            <ul>
                {for breadcrumb_items.iter().enumerate().map(|(idx, item)| {
                    let is_last = idx == breadcrumb_items.len() - 1;
                    html! {
                        <li>
                            if is_last {
                                <span class="text-primary font-semibold">{&item.label}</span>
                            } else {
                                <Link<Route> to={item.route.clone()} classes="hover:text-primary">
                                    {&item.label}
                                </Link<Route>>
                            }
                        </li>
                    }
                })}
            </ul>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct BreadcrumbWithAccountProps {
    account_id: i32,
}

#[function_component(BreadcrumbWithAccount)]
fn breadcrumb_with_account(props: &BreadcrumbWithAccountProps) -> Html {
    let account_id = props.account_id;
    let (fetch_state, _refetch) = use_fetch_with_refetch(move || get_account(account_id));

    let account_label = match &*fetch_state {
        FetchState::Success(account) => account.name.clone(),
        FetchState::Loading => "Loading...".to_string(),
        FetchState::Error(_) => format!("Account {}", account_id),
        FetchState::NotStarted => format!("Account {}", account_id),
    };

    let breadcrumb_items = vec![
        BreadcrumbItem { label: "Home".to_string(), route: Route::Dashboard },
        BreadcrumbItem { label: "Accounts".to_string(), route: Route::Accounts },
        BreadcrumbItem { label: account_label, route: Route::AccountEdit { id: account_id } },
    ];

    html! {
        <div class="breadcrumbs text-sm px-6 py-2 bg-base-100">
            <ul>
                {for breadcrumb_items.iter().enumerate().map(|(idx, item)| {
                    let is_last = idx == breadcrumb_items.len() - 1;
                    html! {
                        <li>
                            if is_last {
                                <span class="text-primary font-semibold">{&item.label}</span>
                            } else {
                                <Link<Route> to={item.route.clone()} classes="hover:text-primary">
                                    {&item.label}
                                </Link<Route>>
                            }
                        </li>
                    }
                })}
            </ul>
        </div>
    }
}

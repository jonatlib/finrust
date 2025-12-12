use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::accounts::AccountEdit;
use crate::components::budgets::Budgets;
use crate::components::dashboard::Dashboard;
use crate::components::forecast::Forecast;
use crate::components::layout::layout::Layout;
use crate::components::reports::Reports;
use crate::components::settings::Settings;
use crate::pages::accounts::AccountsPage;
use crate::pages::transactions::TransactionsPage;
use crate::pages::transaction_edit::TransactionEditPage;
use crate::pages::manual_states::ManualStatesPage;
use crate::pages::recurring::RecurringPage;
use crate::pages::recurring_detail::RecurringDetailPage;
use crate::pages::instances::InstancesPage;
use crate::pages::categories::CategoriesPage;

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/dashboard")]
    Dashboard,
    #[at("/accounts")]
    Accounts,
    #[at("/accounts/:id")]
    AccountEdit { id: i32 },
    #[at("/transactions")]
    Transactions,
    #[at("/transactions/:id")]
    TransactionEdit { id: i32 },
    #[at("/manual-states")]
    ManualStates,
    #[at("/recurring")]
    Recurring,
    #[at("/recurring/:id")]
    RecurringDetail { id: i32 },
    #[at("/instances")]
    Instances,
    #[at("/categories")]
    Categories,
    #[at("/budgets")]
    Budgets,
    #[at("/forecast")]
    Forecast,
    #[at("/reports")]
    Reports,
    #[at("/settings")]
    Settings,
    #[at("/about")]
    About,
    #[not_found]
    #[at("/404")]
    NotFound,
}

pub fn switch(routes: Route) -> Html {
    log::debug!("Routing to: {:?}", routes);
    match routes {
        Route::Home | Route::Dashboard => {
            log::trace!("Rendering Dashboard page");
            html! { <Layout title="Dashboard"><Dashboard /></Layout> }
        }
        Route::Accounts => {
            log::trace!("Rendering Accounts page");
            html! { <AccountsPage /> }
        }
        Route::AccountEdit { id } => {
            log::trace!("Rendering Account Edit page for ID: {}", id);
            html! { <Layout title="Edit Account"><AccountEdit account_id={id} /></Layout> }
        }
        Route::Transactions => {
            log::trace!("Rendering Transactions page");
            html! { <TransactionsPage /> }
        }
        Route::TransactionEdit { id } => {
            log::trace!("Rendering Transaction Edit page for ID: {}", id);
            html! { <TransactionEditPage transaction_id={id} /> }
        }
        Route::ManualStates => {
            log::trace!("Rendering Manual Account States page");
            html! { <ManualStatesPage /> }
        }
        Route::Recurring => {
            log::trace!("Rendering Recurring page");
            html! { <RecurringPage /> }
        }
        Route::RecurringDetail { id } => {
            log::trace!("Rendering Recurring Detail page for ID: {}", id);
            html! { <RecurringDetailPage id={id} /> }
        }
        Route::Instances => {
            log::trace!("Rendering Instances page");
            html! { <InstancesPage /> }
        }
        Route::Categories => {
            log::trace!("Rendering Categories page");
            html! { <CategoriesPage /> }
        }
        Route::Budgets => {
            log::trace!("Rendering Budgets page");
            html! { <Layout title="Budgets"><Budgets /></Layout> }
        }
        Route::Forecast => {
            log::trace!("Rendering Forecast page");
            html! { <Layout title="Forecast"><Forecast /></Layout> }
        }
        Route::Reports => {
            log::trace!("Rendering Reports page");
            html! { <Layout title="Reports"><Reports /></Layout> }
        }
        Route::Settings => {
            log::trace!("Rendering Settings page");
            html! { <Layout title="Settings"><Settings /></Layout> }
        }
        Route::About => {
            log::trace!("Rendering About page");
            html! { <Layout title="About"><div>{"About Page"}</div></Layout> }
        }
        Route::NotFound => {
            log::warn!("404 - Route not found");
            html! { <Layout title="404"><h1>{"404 Not Found"}</h1></Layout> }
        }
    }
}

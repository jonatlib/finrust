use yew::prelude::*;
use yew_router::prelude::*;
use crate::Route;

#[function_component(Sidebar)]
pub fn sidebar() -> Html {
    html! {
        <div class="drawer-side z-50">
            <label aria-label="close sidebar" class="drawer-overlay" for="my-drawer"></label>
            <ul class="menu p-4 w-80 min-h-full bg-base-100 text-base-content border-r border-base-300">
                <li class="mb-4">
                    <div class="flex items-center gap-3 px-2">
                        <div class="w-10 h-10 rounded-lg bg-primary flex items-center justify-center text-primary-content font-bold text-2xl">
                            <i class="fas fa-wallet"></i>
                        </div>
                        <span class="text-2xl font-bold tracking-tight">{"FinRust"}</span>
                    </div>
                </li>

                <li><Link<Route> to={Route::Dashboard} classes="nav-link"><i class="fas fa-home w-5"></i> {"Dashboard"}</Link<Route>></li>
                <li><Link<Route> to={Route::Accounts} classes="nav-link"><i class="fas fa-university w-5"></i> {"Accounts"}</Link<Route>></li>
                <li><Link<Route> to={Route::Transactions} classes="nav-link"><i class="fas fa-exchange-alt w-5"></i> {"Transactions"}</Link<Route>></li>
                <li><Link<Route> to={Route::ManualStates} classes="nav-link"><i class="fas fa-balance-scale w-5"></i> {"Account Balances"}</Link<Route>></li>
                <li><Link<Route> to={Route::Recurring} classes="nav-link"><i class="fas fa-calendar-check w-5"></i> {"Recurring"}</Link<Route>></li>
                <li><Link<Route> to={Route::Instances} classes="nav-link"><i class="fas fa-list-check w-5"></i> {"Instances"}</Link<Route>></li>
                <li><Link<Route> to={Route::Budgets} classes="nav-link"><i class="fas fa-chart-pie w-5"></i> {"Budgets"}</Link<Route>></li>
                <li><Link<Route> to={Route::Forecast} classes="nav-link"><i class="fas fa-chart-area w-5"></i> {"Forecast"}</Link<Route>></li>
                <li><Link<Route> to={Route::Reports} classes="nav-link"><i class="fas fa-chart-line w-5"></i> {"Reports"}</Link<Route>></li>

                <div class="divider"></div>

                <li><Link<Route> to={Route::Settings} classes="nav-link"><i class="fas fa-cog w-5"></i> {"Settings"}</Link<Route>></li>
                <li><a class="nav-link" href="https://github.com/generic/finrust" target="_blank"><i
                        class="fab fa-github w-5"></i> {"GitHub"}</a></li>
            </ul>
        </div>
    }
}

use crate::handlers::{
    accounts::{create_account, delete_account, get_account, get_accounts, update_account},
    health::health_check,
    manual_account_states::{
        create_manual_account_state, delete_manual_account_state, get_manual_account_state,
        get_manual_account_states, update_manual_account_state,
    },
    recurring_income::{
        create_recurring_income, delete_recurring_income, get_recurring_income,
        get_recurring_incomes, update_recurring_income,
    },
    statistics::{get_account_statistics, get_all_accounts_statistics},
    tags::{
        create_tag, delete_tag, get_tag, get_tags, update_tag,
        get_tag_children, link_tag_to_parent, unlink_tag_from_parent,
    },
    timeseries::{get_account_timeseries, get_all_accounts_timeseries},
    transactions::{
        create_transaction, delete_transaction, get_account_transactions, get_transaction,
        get_transactions, update_transaction, create_recurring_instance,
        create_recurring_transaction, get_recurring_transactions, get_recurring_transaction,
        update_recurring_transaction, delete_recurring_transaction,
        create_imported_transaction, get_imported_transactions, get_account_imported_transactions,
        get_imported_transaction, update_imported_transaction, delete_imported_transaction,
        reconcile_imported_transaction, clear_imported_transaction_reconciliation,
    },
    users::{create_user, delete_user, get_user, get_users, update_user},
};
use crate::schemas::{ApiDoc, AppState};
use axum::{
    routing::{delete, get, post, put},
    Router,
};
#[cfg(not(test))]
use axum_prometheus::PrometheusMetricLayer;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Create application router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    // Create Prometheus metrics layer only in non-test environments
    #[cfg(not(test))]
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let mut router = Router::new()
        // Health check
        .route("/health", get(health_check));

    // Only add Prometheus metrics endpoint in non-test environments
    #[cfg(not(test))]
    {
        router = router
            // Prometheus metrics endpoint
            .route("/metrics", get(move || async move { metric_handle.render() }));
    }

    router
        // Account CRUD routes
        .route("/api/v1/accounts", post(create_account))
        .route("/api/v1/accounts", get(get_accounts))
        .route("/api/v1/accounts/:account_id", get(get_account))
        .route("/api/v1/accounts/:account_id", put(update_account))
        .route("/api/v1/accounts/:account_id", delete(delete_account))
        // Manual account states routes
        .route("/api/v1/accounts/:account_id/manual-states", post(create_manual_account_state))
        .route("/api/v1/accounts/:account_id/manual-states", get(get_manual_account_states))
        .route("/api/v1/accounts/:account_id/manual-states/:state_id", get(get_manual_account_state))
        .route("/api/v1/accounts/:account_id/manual-states/:state_id", put(update_manual_account_state))
        .route("/api/v1/accounts/:account_id/manual-states/:state_id", delete(delete_manual_account_state))
        // User CRUD routes
        .route("/api/v1/users", post(create_user))
        .route("/api/v1/users", get(get_users))
        .route("/api/v1/users/:user_id", get(get_user))
        .route("/api/v1/users/:user_id", put(update_user))
        .route("/api/v1/users/:user_id", delete(delete_user))
        // Tag CRUD routes
        .route("/api/v1/tags", post(create_tag))
        .route("/api/v1/tags", get(get_tags))
        .route("/api/v1/tags/:tag_id", get(get_tag))
        .route("/api/v1/tags/:tag_id", put(update_tag))
        .route("/api/v1/tags/:tag_id", delete(delete_tag))
        // Tag tree structure routes
        .route("/api/v1/tags/:tag_id/children", get(get_tag_children))
        .route("/api/v1/tags/:tag_id/parent/:parent_id", put(link_tag_to_parent))
        .route("/api/v1/tags/:tag_id/parent", delete(unlink_tag_from_parent))
        // Transaction CRUD routes
        .route("/api/v1/transactions", post(create_transaction))
        .route("/api/v1/transactions", get(get_transactions))
        .route("/api/v1/transactions/:transaction_id", get(get_transaction))
        .route("/api/v1/transactions/:transaction_id", put(update_transaction))
        .route("/api/v1/transactions/:transaction_id", delete(delete_transaction))
        .route("/api/v1/accounts/:account_id/transactions", get(get_account_transactions))
        // Recurring transaction routes
        .route("/api/v1/recurring-transactions", post(create_recurring_transaction))
        .route("/api/v1/recurring-transactions", get(get_recurring_transactions))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id", get(get_recurring_transaction))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id", put(update_recurring_transaction))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id", delete(delete_recurring_transaction))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id/instances", post(create_recurring_instance))
        // Imported transaction routes
        .route("/api/v1/imported-transactions", post(create_imported_transaction))
        .route("/api/v1/imported-transactions", get(get_imported_transactions))
        .route("/api/v1/imported-transactions/:transaction_id", get(get_imported_transaction))
        .route("/api/v1/imported-transactions/:transaction_id", put(update_imported_transaction))
        .route("/api/v1/imported-transactions/:transaction_id", delete(delete_imported_transaction))
        .route("/api/v1/accounts/:account_id/imported-transactions", get(get_account_imported_transactions))
        .route("/api/v1/imported-transactions/:transaction_id/reconcile", post(reconcile_imported_transaction))
        .route("/api/v1/imported-transactions/:transaction_id/reconcile", delete(clear_imported_transaction_reconciliation))
        // Recurring income routes
        .route("/api/v1/recurring-incomes", post(create_recurring_income))
        .route("/api/v1/recurring-incomes", get(get_recurring_incomes))
        .route("/api/v1/recurring-incomes/:recurring_income_id", get(get_recurring_income))
        .route("/api/v1/recurring-incomes/:recurring_income_id", put(update_recurring_income))
        .route("/api/v1/recurring-incomes/:recurring_income_id", delete(delete_recurring_income))
        // API v1 routes (existing statistics and timeseries)
        .route(
            "/api/v1/accounts/:account_id/statistics",
            get(get_account_statistics),
        )
        .route(
            "/api/v1/accounts/:account_id/timeseries",
            get(get_account_timeseries),
        )
        .route(
            "/api/v1/accounts/statistics",
            get(get_all_accounts_statistics),
        )
        .route(
            "/api/v1/accounts/timeseries",
            get(get_all_accounts_timeseries),
        )
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Add middleware
        .layer({
            #[cfg(not(test))]
            {
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(prometheus_layer)
                    .layer(CompressionLayer::new())
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    .layer(CorsLayer::permissive())
            }
            #[cfg(test)]
            {
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    .layer(CorsLayer::permissive())
            }
        })
        .with_state(state)
}

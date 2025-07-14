use crate::handlers::{
    accounts::{create_account, delete_account, get_account, get_accounts, update_account},
    health::health_check,
    statistics::{get_account_statistics, get_all_accounts_statistics},
    timeseries::{get_account_timeseries, get_all_accounts_timeseries},
    transactions::{
        create_transaction, delete_transaction, get_account_transactions, get_transaction,
        get_transactions, update_transaction, create_recurring_instance,
    },
    users::{create_user, delete_user, get_user, get_users, update_user},
};
use crate::schemas::{ApiDoc, AppState};
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Create application router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Account CRUD routes
        .route("/api/v1/accounts", post(create_account))
        .route("/api/v1/accounts", get(get_accounts))
        .route("/api/v1/accounts/:account_id", get(get_account))
        .route("/api/v1/accounts/:account_id", put(update_account))
        .route("/api/v1/accounts/:account_id", delete(delete_account))
        // User CRUD routes
        .route("/api/v1/users", post(create_user))
        .route("/api/v1/users", get(get_users))
        .route("/api/v1/users/:user_id", get(get_user))
        .route("/api/v1/users/:user_id", put(update_user))
        .route("/api/v1/users/:user_id", delete(delete_user))
        // Transaction CRUD routes
        .route("/api/v1/transactions", post(create_transaction))
        .route("/api/v1/transactions", get(get_transactions))
        .route("/api/v1/transactions/:transaction_id", get(get_transaction))
        .route("/api/v1/transactions/:transaction_id", put(update_transaction))
        .route("/api/v1/transactions/:transaction_id", delete(delete_transaction))
        .route("/api/v1/accounts/:account_id/transactions", get(get_account_transactions))
        // Recurring transaction routes
        .route("/api/v1/recurring-transactions/:recurring_transaction_id/instances", post(create_recurring_instance))
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
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

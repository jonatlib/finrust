use axum::{routing::get, Router};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
    timeout::TimeoutLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::schemas::{AppState, ApiDoc};
use crate::handlers::{
    health::health_check,
    statistics::{get_account_statistics, get_all_accounts_statistics},
    timeseries::{get_account_timeseries, get_all_accounts_timeseries},
};

/// Create application router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))

        // API v1 routes
        .route("/api/v1/accounts/:account_id/statistics", get(get_account_statistics))
        .route("/api/v1/accounts/:account_id/timeseries", get(get_account_timeseries))
        .route("/api/v1/accounts/statistics", get(get_all_accounts_statistics))
        .route("/api/v1/accounts/timeseries", get(get_all_accounts_timeseries))

        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))

        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

use crate::handlers::{
    accounts::{create_account, delete_account, get_account, get_accounts, update_account},
    cache::flush_cache,
    categories::{
        create_category, delete_category, get_categories, get_category, get_category_children,
        get_category_stats, update_category,
    },
    health::health_check,
    manual_account_states::{
        create_manual_account_state, delete_manual_account_state, get_all_manual_account_states,
        get_manual_account_state, get_manual_account_states, update_manual_account_state,
    },
    metrics::{get_account_metrics, get_dashboard_metrics},
    recurring_income::{
        create_recurring_income, delete_recurring_income, get_recurring_income,
        get_recurring_incomes, update_recurring_income,
    },
    scenarios::{
        apply_scenario, create_scenario, delete_scenario, get_scenario, get_scenarios,
        update_scenario,
    },
    statistics::{get_account_statistics, get_all_accounts_statistics, get_monthly_min_balance},
    tags::{
        create_tag, delete_tag, get_tag, get_tag_children, get_tags,
        link_tag_to_parent, unlink_tag_from_parent, update_tag,
    },
    timeseries::{get_account_timeseries, get_all_accounts_timeseries},
    transactions::{
        bulk_create_instances, clear_imported_transaction_reconciliation, create_imported_transaction, create_recurring_instance,
        create_recurring_transaction, create_transaction, delete_imported_transaction,
        delete_recurring_instance, delete_recurring_transaction, delete_transaction,
        get_account_imported_transactions, get_account_transactions, get_imported_transaction,
        get_imported_transactions,
        get_missing_instances, get_recurring_instance,
        get_recurring_instances, get_recurring_transaction,
        get_recurring_transactions, get_transaction, get_transactions,
        reconcile_imported_transaction, update_imported_transaction, update_recurring_instance,
        update_recurring_transaction, update_transaction,
    },
    users::{create_user, delete_user, get_user, get_users, update_user},
};
use crate::middleware::invalidate_cache_on_mutation;
use crate::schemas::{ApiDoc, AppState};
use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Create application router with all routes and middleware.
///
/// When `enable_metrics` is true, a Prometheus metrics layer and `/metrics` endpoint
/// are added. Pass `false` in test environments to avoid global recorder conflicts.
pub fn create_router(state: AppState) -> Router {
    create_router_inner(state, true)
}

/// Create application router without the Prometheus metrics layer.
/// Useful for integration tests where multiple routers are created.
pub fn create_test_router(state: AppState) -> Router {
    create_router_inner(state, false)
}

fn create_router_inner(state: AppState, enable_metrics: bool) -> Router {
    let mut router = Router::new()
        .route("/health", get(health_check));

    if enable_metrics {
        let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
        router = router
            .route("/metrics", get(move || async move { metric_handle.render() }));

        return build_routes(router)
            .layer(axum_middleware::from_fn_with_state(
                state.clone(),
                invalidate_cache_on_mutation,
            ))
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(prometheus_layer)
                    .layer(CompressionLayer::new())
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    .layer(CorsLayer::permissive()),
            )
            .with_state(state);
    }

    build_routes(router)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            invalidate_cache_on_mutation,
        ))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

fn build_routes(router: Router<AppState>) -> Router<AppState> {
    router
        // Cache management
        .route("/api/v1/cache/flush", post(flush_cache))
        // Account CRUD routes
        .route("/api/v1/accounts", post(create_account))
        .route("/api/v1/accounts", get(get_accounts))
        .route("/api/v1/accounts/:account_id", get(get_account))
        .route("/api/v1/accounts/:account_id", put(update_account))
        .route("/api/v1/accounts/:account_id", delete(delete_account))
        // Manual account states routes
        .route("/api/v1/manual-account-states", get(get_all_manual_account_states))
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
        // Category CRUD routes
        .route("/api/v1/categories", post(create_category))
        .route("/api/v1/categories", get(get_categories))
        .route("/api/v1/categories/:id", get(get_category))
        .route("/api/v1/categories/:id", put(update_category))
        .route("/api/v1/categories/:id", delete(delete_category))
        // Category tree structure and stats routes
        .route("/api/v1/categories/:id/children", get(get_category_children))
        .route("/api/v1/categories/stats", get(get_category_stats))
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
        .route("/api/v1/recurring-transactions/missing-instances", get(get_missing_instances))
        .route("/api/v1/recurring-transactions/bulk-create-instances", post(bulk_create_instances))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id", get(get_recurring_transaction))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id", put(update_recurring_transaction))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id", delete(delete_recurring_transaction))
        .route("/api/v1/recurring-transactions/:recurring_transaction_id/instances", post(create_recurring_instance))
        // Recurring instance routes
        .route("/api/v1/recurring-instances", get(get_recurring_instances))
        .route("/api/v1/recurring-instances/:instance_id", get(get_recurring_instance))
        .route("/api/v1/recurring-instances/:instance_id", put(update_recurring_instance))
        .route("/api/v1/recurring-instances/:instance_id", delete(delete_recurring_instance))
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
        // Scenario routes (what-if analysis)
        .route("/api/v1/scenarios", post(create_scenario))
        .route("/api/v1/scenarios", get(get_scenarios))
        .route("/api/v1/scenarios/:scenario_id", get(get_scenario))
        .route("/api/v1/scenarios/:scenario_id", put(update_scenario))
        .route("/api/v1/scenarios/:scenario_id", delete(delete_scenario))
        .route("/api/v1/scenarios/:scenario_id/apply", post(apply_scenario))
        // Metrics routes
        .route("/api/v1/metrics/dashboard", get(get_dashboard_metrics))
        .route("/api/v1/accounts/:account_id/metrics", get(get_account_metrics))
        // API v1 routes (existing statistics and timeseries)
        .route(
            "/api/v1/accounts/:account_id/statistics",
            get(get_account_statistics),
        )
        .route(
            "/api/v1/accounts/:account_id/monthly-min-balance",
            get(get_monthly_min_balance),
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
}

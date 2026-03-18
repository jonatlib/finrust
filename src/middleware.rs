use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use tracing::debug;

use crate::schemas::AppState;

/// Middleware that invalidates all cached data when a mutating request
/// (POST, PUT, DELETE, PATCH) is received.
pub async fn invalidate_cache_on_mutation(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let response = next.run(request).await;

    if response.status().is_success()
        && (method == axum::http::Method::POST
            || method == axum::http::Method::PUT
            || method == axum::http::Method::DELETE
            || method == axum::http::Method::PATCH)
    {
        debug!("Invalidating cache after successful {} request", method);
        state.cache.invalidate_all();
    }

    response
}

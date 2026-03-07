use crate::api_client;
use common::metrics::{AccountMetricsDto, DashboardMetricsDto};

/// Fetches the full financial dashboard with cross-account and per-account metrics.
pub async fn get_dashboard_metrics() -> Result<DashboardMetricsDto, String> {
    log::trace!("Fetching dashboard metrics");
    let result = api_client::get::<DashboardMetricsDto>("/metrics/dashboard").await;

    if let Err(ref e) = result {
        log::error!("Failed to fetch dashboard metrics: {}", e);
    } else {
        log::info!("Successfully fetched dashboard metrics");
    }

    result
}

/// Fetches detailed metrics for a specific account.
pub async fn get_account_metrics(account_id: i32) -> Result<AccountMetricsDto, String> {
    log::trace!("Fetching metrics for account ID: {}", account_id);
    let url = format!("/accounts/{}/metrics", account_id);
    let result = api_client::get::<AccountMetricsDto>(&url).await;

    if let Err(ref e) = result {
        log::error!("Failed to fetch metrics for account {}: {}", account_id, e);
    } else {
        log::info!("Successfully fetched metrics for account ID: {}", account_id);
    }

    result
}

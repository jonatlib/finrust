pub mod account;

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use crate::settings;

// API_BASE is now retrieved from settings
fn api_base() -> String {
    settings::get_settings().api_base_url()
}

/// API Response wrapper
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub message: String,
    pub success: bool,
}

/// Error Response
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub success: bool,
}

/// Common GET request handler
pub async fn get<T>(endpoint: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let url = format!("{}{}", api_base(), endpoint);
    log::debug!("GET request to: {}", url);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Request failed: {}", e);
            log::error!("GET {} - {}", endpoint, error_msg);
            error_msg
        })?;

    if !response.ok() {
        let error_msg = format!("HTTP error: {}", response.status());
        log::error!("GET {} - {}", endpoint, error_msg);
        return Err(error_msg);
    }

    log::trace!("GET {} - Response received, parsing JSON", endpoint);
    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to parse response: {}", e);
            log::error!("GET {} - {}", endpoint, error_msg);
            error_msg
        })?;

    log::info!("GET {} - Success", endpoint);
    Ok(api_response.data)
}

/// Common POST request handler
pub async fn post<T, B>(endpoint: &str, body: &B) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
    B: Serialize,
{
    let url = format!("{}{}", api_base(), endpoint);
    log::debug!("POST request to: {}", url);

    let response = Request::post(&url)
        .json(body)
        .map_err(|e| {
            let error_msg = format!("Failed to serialize request: {}", e);
            log::error!("POST {} - {}", endpoint, error_msg);
            error_msg
        })?
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Request failed: {}", e);
            log::error!("POST {} - {}", endpoint, error_msg);
            error_msg
        })?;

    if !response.ok() {
        log::warn!("POST {} - Non-OK response: {}", endpoint, response.status());
        let error_response: Result<ErrorResponse, _> = response.json().await;
        return Err(match error_response {
            Ok(err) => {
                log::error!("POST {} - API error: {}", endpoint, err.error);
                format!("Error: {}", err.error)
            }
            Err(_) => {
                let error_msg = format!("HTTP error: {}", response.status());
                log::error!("POST {} - {}", endpoint, error_msg);
                error_msg
            }
        });
    }

    log::trace!("POST {} - Response received, parsing JSON", endpoint);
    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to parse response: {}", e);
            log::error!("POST {} - {}", endpoint, error_msg);
            error_msg
        })?;

    log::info!("POST {} - Success", endpoint);
    Ok(api_response.data)
}

/// Common PUT request handler
pub async fn put<T, B>(endpoint: &str, body: &B) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
    B: Serialize,
{
    let url = format!("{}{}", api_base(), endpoint);
    log::debug!("PUT request to: {}", url);

    let response = Request::put(&url)
        .json(body)
        .map_err(|e| {
            let error_msg = format!("Failed to serialize request: {}", e);
            log::error!("PUT {} - {}", endpoint, error_msg);
            error_msg
        })?
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Request failed: {}", e);
            log::error!("PUT {} - {}", endpoint, error_msg);
            error_msg
        })?;

    if !response.ok() {
        log::warn!("PUT {} - Non-OK response: {}", endpoint, response.status());
        let error_response: Result<ErrorResponse, _> = response.json().await;
        return Err(match error_response {
            Ok(err) => {
                log::error!("PUT {} - API error: {}", endpoint, err.error);
                format!("Error: {}", err.error)
            }
            Err(_) => {
                let error_msg = format!("HTTP error: {}", response.status());
                log::error!("PUT {} - {}", endpoint, error_msg);
                error_msg
            }
        });
    }

    log::trace!("PUT {} - Response received, parsing JSON", endpoint);
    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to parse response: {}", e);
            log::error!("PUT {} - {}", endpoint, error_msg);
            error_msg
        })?;

    log::info!("PUT {} - Success", endpoint);
    Ok(api_response.data)
}

/// Common DELETE request handler
pub async fn delete<T>(endpoint: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let url = format!("{}{}", api_base(), endpoint);
    log::debug!("DELETE request to: {}", url);

    let response = Request::delete(&url)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Request failed: {}", e);
            log::error!("DELETE {} - {}", endpoint, error_msg);
            error_msg
        })?;

    if !response.ok() {
        log::warn!("DELETE {} - Non-OK response: {}", endpoint, response.status());
        let error_response: Result<ErrorResponse, _> = response.json().await;
        return Err(match error_response {
            Ok(err) => {
                log::error!("DELETE {} - API error: {}", endpoint, err.error);
                format!("Error: {}", err.error)
            }
            Err(_) => {
                let error_msg = format!("HTTP error: {}", response.status());
                log::error!("DELETE {} - {}", endpoint, error_msg);
                error_msg
            }
        });
    }

    log::trace!("DELETE {} - Response received, parsing JSON", endpoint);
    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to parse response: {}", e);
            log::error!("DELETE {} - {}", endpoint, error_msg);
            error_msg
        })?;

    log::info!("DELETE {} - Success", endpoint);
    Ok(api_response.data)
}

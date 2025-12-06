pub mod account;

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

const API_BASE: &str = "/api/v1";

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
    let url = format!("{}{}", API_BASE, endpoint);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(api_response.data)
}

/// Common POST request handler
pub async fn post<T, B>(endpoint: &str, body: &B) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
    B: Serialize,
{
    let url = format!("{}{}", API_BASE, endpoint);

    let response = Request::post(&url)
        .json(body)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        let error_response: Result<ErrorResponse, _> = response.json().await;
        return Err(match error_response {
            Ok(err) => format!("Error: {}", err.error),
            Err(_) => format!("HTTP error: {}", response.status()),
        });
    }

    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(api_response.data)
}

/// Common PUT request handler
pub async fn put<T, B>(endpoint: &str, body: &B) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
    B: Serialize,
{
    let url = format!("{}{}", API_BASE, endpoint);

    let response = Request::put(&url)
        .json(body)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        let error_response: Result<ErrorResponse, _> = response.json().await;
        return Err(match error_response {
            Ok(err) => format!("Error: {}", err.error),
            Err(_) => format!("HTTP error: {}", response.status()),
        });
    }

    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(api_response.data)
}

/// Common DELETE request handler
pub async fn delete<T>(endpoint: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let url = format!("{}{}", API_BASE, endpoint);

    let response = Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        let error_response: Result<ErrorResponse, _> = response.json().await;
        return Err(match error_response {
            Ok(err) => format!("Error: {}", err.error),
            Err(_) => format!("HTTP error: {}", response.status()),
        });
    }

    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(api_response.data)
}

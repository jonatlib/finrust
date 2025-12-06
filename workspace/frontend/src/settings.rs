use log::Level;
use wasm_bindgen::JsValue;
use web_sys::window;

/// Global application settings
#[derive(Debug, Clone)]
pub struct AppSettings {
    /// Backend API host (e.g., "localhost" or "api.example.com")
    pub api_host: String,

    /// Backend API port (e.g., 3000)
    pub api_port: u16,

    /// API path prefix (e.g., "/api/v1")
    pub api_path: String,

    /// Use HTTPS for API requests
    pub api_use_https: bool,

    /// Default log level for the application
    pub log_level: Level,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u32,

    /// Enable debug mode
    pub debug_mode: bool,

    /// API retry attempts on failure
    pub api_retry_attempts: u32,

    /// Toast notification duration in milliseconds
    pub toast_duration_ms: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_host: "localhost".to_string(),
            api_port: 3000,
            api_path: "/api/v1".to_string(),
            api_use_https: false,
            log_level: Level::Info,
            request_timeout_ms: 30000,
            debug_mode: false,
            api_retry_attempts: 3,
            toast_duration_ms: 5000,
        }
    }
}

impl AppSettings {
    /// Create settings from environment/window location
    pub fn from_environment() -> Self {
        log::trace!("Loading settings from environment");
        let mut settings = Self::default();

        // Detect if running in development mode
        if let Some(window) = window() {
            if let Ok(hostname) = window.location().hostname() {
                log::debug!("Detected hostname: {}", hostname);
                settings.debug_mode = hostname == "localhost" || hostname == "127.0.0.1";

                // In development, use more verbose logging
                if settings.debug_mode {
                    log::debug!("Running in development mode, setting log level to Debug");
                    settings.log_level = Level::Debug;
                }

                // Try to read from localStorage for custom settings
                if let Ok(Some(storage)) = window.local_storage() {
                    log::trace!("Reading settings from localStorage");

                    // Read API host
                    if let Ok(Some(api_host)) = storage.get_item("finrust_api_host") {
                        log::debug!("Loaded API host from storage: {}", api_host);
                        settings.api_host = api_host;
                    }

                    // Read API port
                    if let Ok(Some(api_port)) = storage.get_item("finrust_api_port") {
                        if let Ok(port_val) = api_port.parse::<u16>() {
                            log::debug!("Loaded API port from storage: {}", port_val);
                            settings.api_port = port_val;
                        } else {
                            log::warn!("Failed to parse API port from storage: {}", api_port);
                        }
                    }

                    // Read API path
                    if let Ok(Some(api_path)) = storage.get_item("finrust_api_path") {
                        log::debug!("Loaded API path from storage: {}", api_path);
                        settings.api_path = api_path;
                    }

                    // Read API HTTPS flag
                    if let Ok(Some(use_https)) = storage.get_item("finrust_api_use_https") {
                        settings.api_use_https = use_https.to_lowercase() == "true";
                        log::debug!("Loaded API HTTPS flag from storage: {}", settings.api_use_https);
                    }

                    // Read log level
                    if let Ok(Some(log_level)) = storage.get_item("finrust_log_level") {
                        settings.log_level = match log_level.to_lowercase().as_str() {
                            "error" => Level::Error,
                            "warn" => Level::Warn,
                            "info" => Level::Info,
                            "debug" => Level::Debug,
                            "trace" => Level::Trace,
                            _ => {
                                log::warn!("Unknown log level in storage: {}, using default", log_level);
                                settings.log_level
                            }
                        };
                        log::debug!("Loaded log level from storage: {:?}", settings.log_level);
                    }

                    // Read timeout
                    if let Ok(Some(timeout)) = storage.get_item("finrust_request_timeout_ms") {
                        if let Ok(timeout_val) = timeout.parse::<u32>() {
                            log::debug!("Loaded request timeout from storage: {}ms", timeout_val);
                            settings.request_timeout_ms = timeout_val;
                        } else {
                            log::warn!("Failed to parse request timeout from storage: {}", timeout);
                        }
                    }

                    // Read retry attempts
                    if let Ok(Some(retries)) = storage.get_item("finrust_api_retry_attempts") {
                        if let Ok(retry_val) = retries.parse::<u32>() {
                            log::debug!("Loaded retry attempts from storage: {}", retry_val);
                            settings.api_retry_attempts = retry_val;
                        } else {
                            log::warn!("Failed to parse retry attempts from storage: {}", retries);
                        }
                    }
                } else {
                    log::warn!("localStorage not available, using default settings");
                }
            } else {
                log::error!("Failed to get hostname from window location");
            }
        } else {
            log::error!("Window object not available, using default settings");
        }

        log::info!("Settings initialized: api={}:{}, debug_mode={}, log_level={:?}",
                   settings.api_host, settings.api_port, settings.debug_mode, settings.log_level);
        settings
    }

    /// Save settings to localStorage
    pub fn save_to_storage(&self) -> Result<(), JsValue> {
        log::debug!("Saving settings to localStorage");
        if let Some(window) = window() {
            if let Some(storage) = window.local_storage()? {
                storage.set_item("finrust_api_host", &self.api_host)?;
                storage.set_item("finrust_api_port", &self.api_port.to_string())?;
                storage.set_item("finrust_api_path", &self.api_path)?;
                storage.set_item("finrust_api_use_https", &self.api_use_https.to_string())?;
                storage.set_item("finrust_log_level", &format!("{:?}", self.log_level).to_lowercase())?;
                storage.set_item("finrust_request_timeout_ms", &self.request_timeout_ms.to_string())?;
                storage.set_item("finrust_api_retry_attempts", &self.api_retry_attempts.to_string())?;
                log::info!("Settings saved successfully to localStorage");
            } else {
                log::error!("localStorage not available, cannot save settings");
            }
        } else {
            log::error!("Window object not available, cannot save settings");
        }
        Ok(())
    }

    /// Get the base API URL (protocol + host + port)
    pub fn api_base_url(&self) -> String {
        let protocol = if self.api_use_https { "https" } else { "http" };
        format!("{}://{}:{}{}", protocol, self.api_host, self.api_port, self.api_path)
    }

    /// Get the full API URL for an endpoint
    pub fn api_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.api_base_url(), endpoint)
    }
}

// Global settings instance using thread_local
use std::cell::RefCell;

thread_local! {
    static SETTINGS: RefCell<AppSettings> = RefCell::new(AppSettings::from_environment());
}

/// Get a copy of the current settings
pub fn get_settings() -> AppSettings {
    SETTINGS.with(|s| s.borrow().clone())
}

/// Update the global settings
pub fn update_settings<F>(f: F)
where
    F: FnOnce(&mut AppSettings),
{
    log::trace!("Updating global settings");
    SETTINGS.with(|s| {
        let mut settings = s.borrow_mut();
        f(&mut settings);
    });
    log::debug!("Global settings updated");
}

/// Initialize settings (call this at app startup)
pub fn init_settings() {
    log::trace!("Initializing global settings");
    SETTINGS.with(|s| {
        *s.borrow_mut() = AppSettings::from_environment();
    });
    log::debug!("Global settings initialized successfully");
}

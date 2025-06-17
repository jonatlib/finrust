use thiserror::Error;
use tracing::{error, instrument};

/// Error types for the compute module
#[derive(Error, Debug)]
pub enum ComputeError {
    /// Error from the database operations
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    /// Error from Polars DataFrame operations
    #[error("DataFrame error: {0}")]
    DataFrame(String),

    /// Error from Polars Series operations
    #[error("Series error: {0}")]
    Series(String),

    /// Error from balance computation
    #[error("Balance computation error: {0}")]
    BalanceComputation(String),

    /// Error from forecast computation
    #[error("Forecast computation error: {0}")]
    ForecastComputation(String),

    /// Error from date operations
    #[error("Date error: {0}")]
    Date(String),

    /// Error from decimal operations
    #[error("Decimal error: {0}")]
    Decimal(String),

    /// Error from account operations
    #[error("Account error: {0}")]
    Account(String),

    /// Error from transaction operations
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Runtime error for unexpected situations
    #[error("Runtime error: {0}")]
    Runtime(String),
}

// Implement From<polars::error::PolarsError> for ComputeError
impl From<polars::error::PolarsError> for ComputeError {
    fn from(error: polars::error::PolarsError) -> Self {
        let compute_error = match error {
            polars::error::PolarsError::NoData(_) => {
                let err = ComputeError::DataFrame(format!("No data: {}", error));
                error!(?err, "DataFrame error: No data");
                err
            }
            polars::error::PolarsError::ShapeMismatch(_) => {
                let err = ComputeError::DataFrame(format!("Shape mismatch: {}", error));
                error!(?err, "DataFrame error: Shape mismatch");
                err
            }
            polars::error::PolarsError::SchemaMismatch(_) => {
                let err = ComputeError::DataFrame(format!("Schema mismatch: {}", error));
                error!(?err, "DataFrame error: Schema mismatch");
                err
            }
            polars::error::PolarsError::ComputeError(_) => {
                let err = ComputeError::DataFrame(format!("Compute error: {}", error));
                error!(?err, "DataFrame error: Compute error");
                err
            }
            polars::error::PolarsError::OutOfBounds(_) => {
                let err = ComputeError::DataFrame(format!("Out of bounds: {}", error));
                error!(?err, "DataFrame error: Out of bounds");
                err
            }
            _ => {
                let err = ComputeError::Series(format!("Series error: {}", error));
                error!(?err, "Series error");
                err
            }
        };
        compute_error
    }
}

/// Type alias for Result with ComputeError
pub type Result<T> = std::result::Result<T, ComputeError>;

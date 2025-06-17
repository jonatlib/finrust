use thiserror::Error;

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
        match error {
            polars::error::PolarsError::NoData(_) => ComputeError::DataFrame(format!("No data: {}", error)),
            polars::error::PolarsError::ShapeMismatch(_) => ComputeError::DataFrame(format!("Shape mismatch: {}", error)),
            polars::error::PolarsError::SchemaMismatch(_) => ComputeError::DataFrame(format!("Schema mismatch: {}", error)),
            polars::error::PolarsError::ComputeError(_) => ComputeError::DataFrame(format!("Compute error: {}", error)),
            polars::error::PolarsError::OutOfBounds(_) => ComputeError::DataFrame(format!("Out of bounds: {}", error)),
            _ => ComputeError::Series(format!("Series error: {}", error)),
        }
    }
}

/// Type alias for Result with ComputeError
pub type Result<T> = std::result::Result<T, ComputeError>;

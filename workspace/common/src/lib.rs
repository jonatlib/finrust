//! Common module for transport layer structures
//! 
//! This module provides polars-free wrapper structures for data generated
//! by the compute module. It serves as a transport layer between different
//! components (e.g., API, compute module) without requiring polars dependencies.
//! 
//! The module includes:
//! - Statistics wrappers for account statistics
//! - Timeseries wrappers for account state data over time
//! - Converter utilities for bridging with compute module

pub mod statistics;
pub mod timeseries;
pub mod converters;

// Re-export main types for convenience
pub use statistics::{AccountStatistics, AccountStatisticsCollection, TimePeriod};
pub use timeseries::{AccountStatePoint, AccountStateTimeseries, DateRange};
pub use converters::{
    compute_stats_to_common_stats, 
    compute_stats_vec_to_collection,
    create_account_state_point,
    create_account_state_points,
    create_year_period,
    create_month_period,
    create_date_range_period,
    timeseries_to_raw_data,
    statistics_to_raw_data,
    DataFrameConverter,
    dataframe_to_timeseries,
};

pub mod entities;

// Re-export tracing for use in this crate
pub use tracing;

// Initialize tracing if not already initialized
#[cfg(not(test))]
pub fn init_tracing() {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;

    // Initialize the tracing subscriber with a default configuration
    // This will log to stdout with a default format
    // The log level can be controlled via the RUST_LOG environment variable
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .init();
}

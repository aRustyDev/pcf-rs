use anyhow::Result;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use uuid::Uuid;

use crate::config::LoggingConfig;

/// Generate a unique trace ID for request correlation
#[deprecated(
    since = "0.1.0",
    note = "Use `observability::logging::current_trace_id()` instead. This function will be removed in a future version."
)]
pub fn generate_trace_id() -> String {
    Uuid::new_v4().to_string()
}

/// Set up tracing subscriber based on configuration
/// 
/// Supports two formats:
/// - "json": Structured JSON output for production
/// - "pretty": Human-readable format for development
/// 
/// The subscriber is configured with:
/// - Environment-based log level filtering
/// - Format selection based on config
/// - Async, non-blocking logging
#[deprecated(
    since = "0.1.0",
    note = "Use `observability::logging::init_logging()` instead. This function will be removed in a future version."
)]
pub fn setup_tracing(config: &LoggingConfig) -> Result<()> {
    // Create environment filter from log level
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    match config.format.as_str() {
        "json" => {
            // JSON format for production
            let json_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(json_layer)
                .try_init()?;
        }
        "pretty" => {
            // Pretty format for development
            let pretty_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(pretty_layer)
                .try_init()?;
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported log format: {}. Use 'json' or 'pretty'",
                config.format
            ));
        }
    }

    Ok(())
}
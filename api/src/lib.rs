pub mod config;
pub mod error;
pub mod health;
pub mod logging;
pub mod server;
pub mod services;

pub use config::*;
pub use error::*;
pub use logging::*;
pub use server::*;

use anyhow::Result;
use std::panic;

/// Main server entry point for library usage
pub async fn run_server() -> Result<()> {
    // Initialize logging FIRST
    let config = config::LoggingConfig {
        level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        format: match std::env::var("ENVIRONMENT").as_deref() {
            Ok("development") => "pretty",
            _ => "json",
        }.to_string(),
    };
    
    // Initialize logging if not already done (for tests)
    let _ = logging::setup_tracing(&config);
    
    // Set up panic handler (so it can use logging)
    panic::set_hook(Box::new(|panic_info| {
        ::tracing::error!(?panic_info, "FATAL: Panic occurred");
        std::process::exit(1);
    }));
    
    ::tracing::info!("PCF API starting up");
    
    // Load configuration
    let app_config = config::load_config()?;
    
    // Start server
    server::start_server(app_config).await?;
    
    Ok(())
}
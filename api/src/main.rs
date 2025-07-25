mod config;
mod error;
mod health;
mod logging;
mod server;

use std::panic;
use tracing::{error, info};

#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");

fn main() {
    // Initialize logging FIRST
    let config = config::LoggingConfig {
        level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        format: match std::env::var("ENVIRONMENT").as_deref() {
            Ok("development") => "pretty",
            _ => "json",
        }.to_string(),
    };
    
    if let Err(e) = logging::setup_tracing(&config) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }
    
    // NOW set up panic handler (so it can use logging)
    panic::set_hook(Box::new(|panic_info| {
        error!(?panic_info, "FATAL: Panic occurred");
        std::process::exit(1);
    }));
    
    info!("PCF API starting up");
    
    // Phase 1: Basic server setup will be implemented in next checkpoint
    info!("Phase 1 foundation complete - ready for server implementation");
}
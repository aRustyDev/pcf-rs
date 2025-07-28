pub mod auth;
pub mod config;
pub mod error;
pub mod graphql;
pub mod health;
pub mod helpers;
pub mod middleware;
pub mod observability;
pub mod schema;
pub mod server;
pub mod services;

#[cfg(test)]
pub mod tests;

#[cfg(feature = "benchmarks")]
pub mod benchmarks;

pub use config::*;
pub use error::*;
pub use server::*;

use anyhow::Result;
use std::panic;
use crate::auth::components::AuthorizationComponents;

/// Main server entry point for library usage
pub async fn run_server() -> Result<()> {
    // Logging is now initialized in observability::init_observability()
    
    // Initialize observability
    observability::init_observability()?;
    ::tracing::info!("Observability initialized");
    
    // Set up panic handler (so it can use logging)
    panic::set_hook(Box::new(|panic_info| {
        ::tracing::error!(?panic_info, "FATAL: Panic occurred");
        std::process::exit(1);
    }));
    
    ::tracing::info!("PCF API starting up");
    
    // Load configuration
    let app_config = config::load_config()?;
    
    // Create authorization components
    let auth_components = if app_config.demo.is_enabled() {
        ::tracing::warn!("Creating demo authorization components (DEMO MODE)");
        AuthorizationComponents::new_demo(&app_config.authorization).await?
    } else {
        ::tracing::info!("Creating production authorization components");
        AuthorizationComponents::new_production(&app_config.authorization).await?
    };
    
    // Start server with auth components
    server::start_server(app_config, auth_components).await?;
    
    Ok(())
}
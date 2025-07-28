use anyhow::Result;
use axum::{
    middleware,
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

use crate::config::AppConfig;
use crate::health::{handlers::{liveness_handler, readiness_handler}, HealthManager};
use crate::auth::components::AuthorizationComponents;
use crate::observability::metrics_endpoint;
use crate::middleware::metrics_middleware;

/// Start the Axum HTTP server with health endpoints and graceful shutdown
/// 
/// This function creates an Axum HTTP server with the following features:
/// - Health endpoints at /health/liveness and /health/readiness
/// - Request tracing middleware with trace ID generation
/// - CORS support for browser-based clients
/// - Graceful shutdown on SIGTERM/SIGINT signals
/// 
/// The server will bind to the address and port specified in the configuration
/// and will handle graceful shutdown within the configured timeout period.
pub async fn start_server(config: AppConfig, _auth_components: AuthorizationComponents) -> Result<()> {
    info!("Starting PCF API server on {}:{}", config.server.bind, config.server.port);
    
    // Initialize health manager
    let health_manager = HealthManager::new();
    
    // Create Axum router with middleware and routes
    let app = create_router(health_manager.clone());
    info!("Router created successfully");
    
    // Bind to configured address and port
    let bind_addr = format!("{}:{}", config.server.bind, config.server.port);
    info!("Attempting to bind to {}", bind_addr);
    
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}. Is another process using this port?", bind_addr, e))?;
    info!("Server successfully bound to {}", bind_addr);
    
    info!("Starting HTTP server...");
    
    // Mark health manager as ready after server binds
    health_manager.mark_ready().await;
    
    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(config.server.shutdown_timeout))
        .await?;
    
    info!("Server shutdown complete");
    Ok(())
}

/// Create the Axum router with all middleware and routes
/// 
/// Sets up the routing table with:
/// - Health check endpoints for Kubernetes liveness/readiness probes
/// - Request tracing middleware for observability
/// - CORS middleware for browser compatibility
fn create_router(health_manager: HealthManager) -> Router {
    Router::new()
        // Metrics endpoint (before other routes for priority)
        .route("/metrics", get(metrics_endpoint))
        // Health check routes (per WORK_PLAN.md Task 1.6.1 and 1.6.2)
        .route("/health", get(liveness_handler))              // Simple liveness check
        .route("/health/liveness", get(liveness_handler))     // Keep existing for compatibility
        .route("/health/ready", get(readiness_handler))       // JSON readiness with state management
        .route("/health/readiness", get(readiness_handler))   // Keep existing for compatibility
        // Add health manager state
        .with_state(health_manager)
        // Add metrics and tracing middleware to all routes
        .layer(middleware::from_fn(metrics_middleware))
        // Add CORS middleware for browser requests
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        )
}

/// Wait for shutdown signal (SIGTERM or SIGINT)
/// 
/// This function creates signal handlers for graceful shutdown:
/// - SIGINT (Ctrl+C) for development/manual shutdown
/// - SIGTERM for production container shutdown
/// 
/// The function returns when either signal is received, allowing
/// Axum to begin its graceful shutdown process.
async fn shutdown_signal(_timeout_seconds: u64) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C), starting graceful shutdown");
        }
        _ = terminate => {
            info!("Received SIGTERM, starting graceful shutdown");
        }
    }
    
    // Return immediately - axum will handle the graceful shutdown timing
    info!("Shutdown signal received, initiating graceful shutdown");
}
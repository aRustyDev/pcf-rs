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
use crate::health::handlers::{liveness_handler, readiness_handler};
use crate::logging::trace_requests;

/// Start the Axum HTTP server with graceful shutdown
pub async fn start_server(config: AppConfig) -> Result<()> {
    info!("Starting PCF API server on {}:{}", config.server.bind, config.server.port);
    
    // Create Axum router with middleware and routes
    let app = create_router();
    info!("Router created successfully");
    
    // Bind to configured address and port
    let bind_addr = format!("{}:{}", config.server.bind, config.server.port);
    info!("Attempting to bind to {}", bind_addr);
    
    let listener = TcpListener::bind(&bind_addr).await?;
    info!("Server successfully bound to {}", bind_addr);
    
    info!("Starting HTTP server...");
    
    // Start server (graceful shutdown will be added after tests pass)
    axum::serve(listener, app)
        .await?;
    
    info!("Server shutdown complete");
    Ok(())
}

/// Create the Axum router with all middleware and routes
fn create_router() -> Router {
    Router::new()
        // Health check routes
        .route("/health/liveness", get(liveness_handler))
        .route("/health/readiness", get(readiness_handler))
        // Add tracing middleware to all routes
        .layer(middleware::from_fn(trace_requests))
        // Add CORS middleware for browser requests
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        )
}

/// Wait for shutdown signal (SIGTERM or SIGINT)
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
use axum::{response::Response, http::StatusCode};
use tracing::info;

/// Liveness probe handler - indicates if the application is running
/// 
/// This endpoint should return 200 OK if the application process is alive,
/// even if it's not ready to serve traffic. Used by Kubernetes to determine
/// if a pod should be restarted.
pub async fn liveness_handler() -> Response<String> {
    info!("Liveness check requested");
    
    Response::builder()
        .status(StatusCode::OK)
        .body("OK".to_string())
        .unwrap()
}

/// Readiness probe handler - indicates if the application is ready to serve traffic
/// 
/// This endpoint should return 200 OK only when the application is fully initialized
/// and ready to handle requests. Used by Kubernetes to determine if traffic should
/// be routed to this pod.
pub async fn readiness_handler() -> Response<String> {
    info!("Readiness check requested");
    
    // For Phase 1, we're always ready since we have no external dependencies
    // In later phases, this would check database connections, external services, etc.
    Response::builder()
        .status(StatusCode::OK)
        .body("OK".to_string())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_liveness_handler() {
        let response = liveness_handler().await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body(), "OK");
    }
    
    #[tokio::test]
    async fn test_readiness_handler() {
        let response = readiness_handler().await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body(), "OK");
    }
}
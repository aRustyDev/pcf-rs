use axum::{
    extract::State,
    http::StatusCode,
    response::{Json, Response},
};
use tracing::info;

use super::{HealthManager, HealthResponse};

/// Liveness probe handler at /health - indicates if the application is running
/// 
/// This endpoint should return 200 OK if the application process is alive,
/// even if it's not ready to serve traffic. Used by Kubernetes to determine
/// if a pod should be restarted. This endpoint is simple and fast (< 1 second).
pub async fn liveness_handler() -> Response<String> {
    info!("Liveness check requested");
    
    Response::builder()
        .status(StatusCode::OK)
        .body("OK".to_string())
        .expect("Simple response build should never fail")
}

/// Readiness probe handler at /health/ready - indicates if the application is ready to serve traffic
/// 
/// This endpoint returns JSON with detailed service statuses and uses caching (5s TTL)
/// with stale data support (30s). During startup period (first 30s), it tracks initialization.
/// Used by Kubernetes to determine if traffic should be routed to this pod.
pub async fn readiness_handler(State(health_manager): State<HealthManager>) -> Result<Json<HealthResponse>, StatusCode> {
    info!("Readiness check requested");
    
    let health_response = health_manager.get_health().await;
    
    // Return appropriate HTTP status based on overall health
    match health_response.status {
        super::HealthStatus::Healthy | super::HealthStatus::Degraded => {
            Ok(Json(health_response))
        }
        super::HealthStatus::Starting => {
            // During startup grace period, return 200 OK but with starting status
            if health_manager.is_in_startup_period() {
                Ok(Json(health_response))
            } else {
                // After grace period, starting status means something is wrong
                Err(StatusCode::SERVICE_UNAVAILABLE)
            }
        }
        super::HealthStatus::Unhealthy => {
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{HealthManager, HealthStatus};
    
    #[tokio::test]
    async fn test_liveness_handler() {
        let response = liveness_handler().await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body(), "OK");
    }
    
    #[tokio::test]
    async fn test_readiness_handler_healthy() {
        let health_manager = HealthManager::new();
        health_manager.mark_ready().await;
        
        let result = readiness_handler(State(health_manager)).await;
        assert!(result.is_ok());
        
        let Json(response) = result.unwrap();
        assert_eq!(response.status, HealthStatus::Healthy);
        assert!(response.services.contains_key("api"));
    }
    
    #[tokio::test]
    async fn test_readiness_handler_starting() {
        let health_manager = HealthManager::new();
        // Don't mark as ready - should be in starting state
        
        let result = readiness_handler(State(health_manager)).await;
        assert!(result.is_ok()); // Should be OK during startup grace period
        
        let Json(response) = result.unwrap();
        assert_eq!(response.status, HealthStatus::Starting);
    }
    
    #[tokio::test]
    async fn test_readiness_handler_degraded() {
        let health_manager = HealthManager::new();
        health_manager.mark_ready().await;
        health_manager.update_service_health("cache", HealthStatus::Degraded, "High latency".to_string()).await;
        
        let result = readiness_handler(State(health_manager)).await;
        assert!(result.is_ok());
        
        let Json(response) = result.unwrap();
        assert_eq!(response.status, HealthStatus::Degraded);
    }
    
    #[tokio::test]
    async fn test_readiness_handler_unhealthy() {
        let health_manager = HealthManager::new();
        health_manager.update_service_health("database", HealthStatus::Unhealthy, "Connection failed".to_string()).await;
        
        let result = readiness_handler(State(health_manager)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
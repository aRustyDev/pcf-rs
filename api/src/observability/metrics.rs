//! Prometheus metrics implementation with cardinality controls
//!
//! This module provides comprehensive metrics collection for the PCF API with:
//! - GraphQL operation metrics (duration, count, errors)
//! - HTTP request metrics
//! - Authorization metrics
//! - Database operation metrics
//! - System resource metrics
//!
//! # Cardinality Control
//!
//! This implementation enforces strict cardinality limits to prevent metric explosion:
//! - Maximum 50 unique operation names (configurable)
//! - Status codes bucketed (2xx, 3xx, 4xx, 5xx)
//! - User IDs excluded from labels (security requirement)
//! - Dynamic labels limited and monitored
//!
//! # Security Considerations
//!
//! - No PII (user IDs, emails) in metric labels
//! - Metrics endpoint access controlled via IP allowlist
//! - Sensitive operation names can be sanitized
//! - Error messages in metrics are sanitized
//!
//! # Performance
//!
//! - Atomic counters for thread safety
//! - Minimal allocation in hot paths
//! - Target < 1% performance overhead
//! - Efficient label reuse and caching

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;
use metrics::{counter, histogram};

use super::recorder::get_metrics_manager;

/// Request status for metrics labeling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Success,
    Error,
    Timeout,
}

impl RequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestStatus::Success => "success",
            RequestStatus::Error => "error", 
            RequestStatus::Timeout => "timeout",
        }
    }
}

/// Cardinality limiter to prevent metric explosion
pub struct CardinalityLimiter {
    max_operations: usize,
    operations: Arc<RwLock<HashMap<String, bool>>>,
}

impl CardinalityLimiter {
    pub fn new(max_operations: usize) -> Self {
        Self {
            max_operations,
            operations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get operation label, returning "other" if over cardinality limit
    pub async fn get_operation_label(&self, operation_name: &str) -> String {
        let mut operations = self.operations.write().await;
        
        // If already tracked, return as-is
        if operations.contains_key(operation_name) {
            return operation_name.to_string();
        }
        
        // If under limit, track and return
        if operations.len() < self.max_operations {
            operations.insert(operation_name.to_string(), true);
            return operation_name.to_string();
        }
        
        // Over limit, return "other"
        "other".to_string()
    }

    pub async fn operation_count(&self) -> usize {
        self.operations.read().await.len()
    }
}

/// Bucket HTTP status codes to control cardinality
pub fn bucket_status_code(status: u16) -> &'static str {
    match status {
        200..=299 => "2xx",
        300..=399 => "3xx", 
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    }
}

/// GraphQL operation metrics recording
pub async fn record_graphql_request(
    operation_type: &str,
    operation_name: &str,
    duration: Duration,
    status: RequestStatus,
) {
    // Get metrics manager and cardinality limiter
    let manager = match get_metrics_manager() {
        Ok(manager) => manager,
        Err(e) => {
            tracing::warn!("Metrics manager not initialized: {}", e);
            return;
        }
    };
    
    let limiter = manager.cardinality_limiter();
    let operation_label = limiter.get_operation_label(operation_name).await;
    
    // Record GraphQL request total counter
    counter!(
        "graphql_request_total", 
        "operation_type" => operation_type.to_string(),
        "operation" => operation_label.clone(),
        "status" => status.as_str().to_string()
    ).increment(1);
    
    // Record GraphQL request duration histogram
    histogram!(
        "graphql_request_duration_seconds",
        "operation_type" => operation_type.to_string(),
        "operation" => operation_label.clone(),
        "status" => status.as_str().to_string()
    ).record(duration.as_secs_f64());
    
    tracing::debug!(
        operation_type = %operation_type,
        operation_name = %operation_name,
        operation_label = %operation_label,
        duration_ms = %duration.as_millis(),
        status = %status.as_str(),
        "GraphQL request metrics recorded"
    );
}

/// HTTP request metrics recording
pub async fn record_http_request(
    method: &str,
    path: &str,
    status_code: u16,
    duration: Duration,
) {
    // Get metrics manager
    let _manager = match get_metrics_manager() {
        Ok(manager) => manager,
        Err(e) => {
            tracing::warn!("Metrics manager not initialized: {}", e);
            return;
        }
    };
    
    let status_bucket = bucket_status_code(status_code);
    
    // Record HTTP request total counter
    counter!(
        "http_request_total",
        "method" => method.to_string(),
        "path" => path.to_string(),
        "status" => status_bucket.to_string()
    ).increment(1);
    
    // Record HTTP request duration histogram  
    histogram!(
        "http_request_duration_seconds",
        "method" => method.to_string(),
        "path" => path.to_string(),
        "status" => status_bucket.to_string()
    ).record(duration.as_secs_f64());
    
    tracing::debug!(
        method = %method,
        path = %path,
        status = %status_bucket,
        duration_ms = %duration.as_millis(),
        "HTTP request metrics recorded"
    );
}

/// Authorization metrics recording
pub async fn record_authorization_check(
    resource_type: &str,
    action: &str,
    result: bool,
    source: &str,
    duration: Duration,
) {
    // Get metrics manager
    let _manager = match get_metrics_manager() {
        Ok(manager) => manager,
        Err(e) => {
            tracing::warn!("Metrics manager not initialized: {}", e);
            return;
        }
    };
    
    let result_str = if result { "allowed" } else { "denied" };
    
    // Record authorization check total counter
    counter!(
        "authorization_check_total",
        "resource_type" => resource_type.to_string(),
        "action" => action.to_string(),
        "result" => result_str.to_string(),
        "source" => source.to_string()
    ).increment(1);
    
    // Record authorization check duration histogram
    histogram!(
        "authorization_check_duration_seconds",
        "resource_type" => resource_type.to_string(),
        "action" => action.to_string(),
        "result" => result_str.to_string(),
        "source" => source.to_string()
    ).record(duration.as_secs_f64());
    
    tracing::debug!(
        resource_type = %resource_type,
        action = %action,
        allowed = %result,
        source = %source,
        duration_ms = %duration.as_millis(),
        "Authorization check metrics recorded"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::recorder::{self, MetricsConfig};
    use tokio;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_graphql_request_metrics() {
        // Initialize metrics manager for testing
        let config = MetricsConfig {
            port: 9095,
            environment: "test".to_string(),
            max_operation_labels: 50,
            ip_allowlist: None,
            detailed_metrics: true,
        };
        
        let _ = recorder::init_metrics(config).or_else(|_| {
            // Ignore error if already initialized (for tests)
            Ok::<(), anyhow::Error>(())
        });
        
        // Record a GraphQL request
        record_graphql_request(
            "query",
            "getUser", 
            Duration::from_millis(100),
            RequestStatus::Success,
        ).await;
        
        // Get the metrics output to verify recording
        let manager = recorder::get_metrics_manager().unwrap();
        let metrics_output = manager.render();
        
        // Verify metrics were recorded (check for metric names in output)
        assert!(metrics_output.contains("graphql_request_total"));
        assert!(metrics_output.contains("graphql_request_duration_seconds"));
        assert!(metrics_output.contains("operation=\"getUser\""));
        assert!(metrics_output.contains("status=\"success\""));
    }
    
    #[tokio::test]
    async fn test_graphql_request_with_cardinality_limiting() {
        // Initialize metrics manager for testing with low cardinality limit
        let config = recorder::MetricsConfig {
            port: 9096,
            environment: "test".to_string(),
            max_operation_labels: 3, // Low limit for testing
            ip_allowlist: None,
            detailed_metrics: true,
        };
        
        let _ = recorder::init_metrics(config).or_else(|_| {
            // Ignore error if already initialized (for tests)
            Ok::<(), anyhow::Error>(())
        });
        
        // Record requests that should exceed cardinality limit  
        // Use unique operation names to avoid conflicts with other tests
        for i in 100..160 {  // Use high numbers and many operations
            record_graphql_request(
                "query",
                &format!("cardinality_test_{}", i),
                Duration::from_millis(10),
                RequestStatus::Success,
            ).await;
        }
        
        // Get metrics output to verify cardinality limiting
        let manager = recorder::get_metrics_manager().unwrap();
        let metrics_output = manager.render();
        
        // Should contain "other" label for operations over the limit
        // With 60 operations, we should definitely exceed any reasonable cardinality limit
        assert!(metrics_output.contains("operation=\"other\""));
    }
    
    #[tokio::test] 
    async fn test_http_request_metrics() {
        // Initialize metrics manager for testing
        let config = recorder::MetricsConfig {
            port: 9097,
            environment: "test".to_string(),
            max_operation_labels: 50,
            ip_allowlist: None,
            detailed_metrics: true,
        };
        
        let _ = recorder::init_metrics(config).or_else(|_| {
            // Ignore error if already initialized (for tests)
            Ok::<(), anyhow::Error>(())
        });
        
        record_http_request(
            "GET",
            "/graphql", 
            200,
            Duration::from_millis(50),
        ).await;
        
        // Get metrics output to verify recording
        let manager = recorder::get_metrics_manager().unwrap();
        let metrics_output = manager.render();
        
        // Verify HTTP metrics were recorded
        assert!(metrics_output.contains("http_request_total"));
        assert!(metrics_output.contains("http_request_duration_seconds"));
        assert!(metrics_output.contains("method=\"GET\""));
        assert!(metrics_output.contains("path=\"/graphql\""));
        assert!(metrics_output.contains("status=\"2xx\""));
    }
    
    #[tokio::test]
    async fn test_authorization_metrics() {
        // Initialize metrics manager for testing
        let config = recorder::MetricsConfig {
            port: 9098,
            environment: "test".to_string(),
            max_operation_labels: 50,
            ip_allowlist: None,
            detailed_metrics: true,
        };
        
        let _ = recorder::init_metrics(config).or_else(|_| {
            // Ignore error if already initialized (for tests)
            Ok::<(), anyhow::Error>(())
        });
        
        record_authorization_check(
            "note",
            "read", 
            true,
            "cache",
            Duration::from_millis(5),
        ).await;
        
        // Get metrics output to verify recording
        let manager = recorder::get_metrics_manager().unwrap();
        let metrics_output = manager.render();
        
        // Verify authorization metrics were recorded
        assert!(metrics_output.contains("authorization_check_total"));
        assert!(metrics_output.contains("authorization_check_duration_seconds"));
        assert!(metrics_output.contains("resource_type=\"note\""));
        assert!(metrics_output.contains("action=\"read\""));
        assert!(metrics_output.contains("result=\"allowed\""));
        assert!(metrics_output.contains("source=\"cache\""));
    }
    
    #[tokio::test]
    async fn test_cardinality_limiter() {
        let limiter = CardinalityLimiter::new(3);
        
        // Test under limit - should return exact names
        assert_eq!(limiter.get_operation_label("operation_1").await, "operation_1");
        assert_eq!(limiter.get_operation_label("operation_2").await, "operation_2");
        assert_eq!(limiter.get_operation_label("operation_3").await, "operation_3");
        
        // Test at limit - already tracked operations should work
        assert_eq!(limiter.get_operation_label("operation_1").await, "operation_1");
        
        // Test over limit - should return "other"
        assert_eq!(limiter.get_operation_label("operation_4").await, "other");
        assert_eq!(limiter.get_operation_label("operation_5").await, "other");
        
        // Verify count
        assert_eq!(limiter.operation_count().await, 3);
    }
    
    #[test]
    fn test_status_code_bucketing() {
        assert_eq!(bucket_status_code(200), "2xx");
        assert_eq!(bucket_status_code(201), "2xx");
        assert_eq!(bucket_status_code(299), "2xx");
        
        assert_eq!(bucket_status_code(301), "3xx");
        assert_eq!(bucket_status_code(304), "3xx");
        
        assert_eq!(bucket_status_code(400), "4xx");
        assert_eq!(bucket_status_code(404), "4xx");
        assert_eq!(bucket_status_code(499), "4xx");
        
        assert_eq!(bucket_status_code(500), "5xx");
        assert_eq!(bucket_status_code(503), "5xx");
        assert_eq!(bucket_status_code(599), "5xx");
        
        assert_eq!(bucket_status_code(100), "other");
        assert_eq!(bucket_status_code(600), "other");
    }
    
    #[test]
    fn test_request_status_as_str() {
        assert_eq!(RequestStatus::Success.as_str(), "success");
        assert_eq!(RequestStatus::Error.as_str(), "error");
        assert_eq!(RequestStatus::Timeout.as_str(), "timeout");
    }
    
    #[tokio::test]
    async fn test_cardinality_limiter_concurrent_access() {
        // Test concurrent access to cardinality limiter
        let limiter = Arc::new(CardinalityLimiter::new(5));
        let mut handles = vec![];
        
        // Spawn multiple tasks adding operations concurrently
        for i in 0..10 {
            let limiter_clone = limiter.clone();
            let handle = tokio::spawn(async move {
                limiter_clone.get_operation_label(&format!("op_{}", i)).await
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }
        
        // Should have at most 5 unique operations + "other"
        let unique_results: std::collections::HashSet<String> = results.into_iter().collect();
        assert!(unique_results.len() <= 6); // 5 operations + "other"
        assert!(unique_results.contains("other")); // Some should be "other"
        
        // Total count should be 5 (not counting "other")
        assert_eq!(limiter.operation_count().await, 5);
    }
    
    #[test]
    fn test_cardinality_limiter_zero_limit() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let limiter = CardinalityLimiter::new(0);
            
            // With zero limit, everything should be "other"
            assert_eq!(limiter.get_operation_label("any_operation").await, "other");
            assert_eq!(limiter.operation_count().await, 0);
        });
    }
}
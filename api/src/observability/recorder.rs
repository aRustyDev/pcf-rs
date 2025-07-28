//! Prometheus metrics recorder with cardinality controls
//!
//! This module provides the core metrics recording infrastructure with:
//! - Prometheus HTTP exporter with configurable port
//! - Global cardinality limiting to prevent metric explosion
//! - Service-level labels (service name, environment, version)
//! - IP allowlist support for metrics endpoint security

use std::sync::{Arc, OnceLock};
use std::net::SocketAddr;
use anyhow::{Result, anyhow};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use super::CardinalityLimiter;

/// Configuration for metrics collection
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Port for Prometheus HTTP endpoint
    pub port: u16,
    /// Service environment (dev, staging, prod)
    pub environment: String,
    /// Maximum unique operation labels to prevent cardinality explosion
    pub max_operation_labels: usize,
    /// Optional IP allowlist for metrics endpoint access
    pub ip_allowlist: Option<Vec<String>>,
    /// Whether to enable detailed metrics (may impact performance)
    pub detailed_metrics: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            port: 9090,
            environment: "development".to_string(),
            max_operation_labels: 50,
            ip_allowlist: None,
            detailed_metrics: false,
        }
    }
}

/// Global metrics manager with Prometheus integration
pub struct MetricsManager {
    handle: PrometheusHandle,
    config: MetricsConfig,
    cardinality_limiter: Arc<CardinalityLimiter>,
}

impl MetricsManager {
    /// Initialize Prometheus recorder with cardinality controls
    pub fn new(config: MetricsConfig) -> Result<Self> {
        // Create cardinality limiter
        let limiter = Arc::new(CardinalityLimiter::new(config.max_operation_labels));
        
        // Build Prometheus exporter
        let builder = PrometheusBuilder::new()
            .with_http_listener(SocketAddr::from(([0, 0, 0, 0], config.port)))
            .add_global_label("service", "pcf-api")
            .add_global_label("environment", &config.environment)
            .add_global_label("version", env!("CARGO_PKG_VERSION"));
        
        // Install recorder and get handle
        let handle = builder
            .install_recorder()
            .map_err(|e| anyhow!("Failed to install Prometheus recorder: {}", e))?;
        
        tracing::info!(
            port = %config.port,
            environment = %config.environment,
            max_operations = %config.max_operation_labels,
            "Prometheus metrics recorder initialized"
        );
        
        Ok(Self {
            handle,
            config,
            cardinality_limiter: limiter,
        })
    }
    
    /// Get Prometheus metrics output
    pub fn render(&self) -> String {
        self.handle.render()
    }
    
    /// Get cardinality limiter for operation labels
    pub fn cardinality_limiter(&self) -> Arc<CardinalityLimiter> {
        self.cardinality_limiter.clone()
    }
    
    /// Check if IP is allowed to access metrics endpoint
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        match &self.config.ip_allowlist {
            Some(allowlist) => allowlist.contains(&ip.to_string()),
            None => true, // No allowlist means all IPs are allowed
        }
    }
    
    /// Get current metrics configuration
    pub fn config(&self) -> &MetricsConfig {
        &self.config
    }
    
    /// Get metrics endpoint URL
    pub fn endpoint_url(&self) -> String {
        format!("http://0.0.0.0:{}/metrics", self.config.port)
    }
}

/// Global metrics instance (initialized once at startup)
static METRICS_MANAGER: OnceLock<Arc<MetricsManager>> = OnceLock::new();

/// Initialize global metrics manager
pub fn init_metrics(config: MetricsConfig) -> Result<()> {
    let manager = Arc::new(MetricsManager::new(config)?);
    
    // Initialize global instance
    match METRICS_MANAGER.set(manager) {
        Ok(()) => Ok(()),
        Err(_) => Err(anyhow!("Metrics manager was already initialized")),
    }
}

/// Get global metrics manager instance
pub fn get_metrics_manager() -> Result<Arc<MetricsManager>> {
    METRICS_MANAGER
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("Metrics manager not initialized. Call init_metrics() first."))
}

/// Extract client IP from various headers
pub fn extract_client_ip(headers: &axum::http::HeaderMap) -> String {
    // Try X-Forwarded-For first (most common proxy header)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Take first IP in case of multiple
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }
    
    // Try X-Real-IP (Nginx proxy)
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }
    
    // Fallback to connection IP (not available in headers, return placeholder)
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    
    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert_eq!(config.port, 9090);
        assert_eq!(config.environment, "development");
        assert_eq!(config.max_operation_labels, 50);
        assert!(config.ip_allowlist.is_none());
        assert!(!config.detailed_metrics);
    }
    
    #[test]
    fn test_metrics_manager_creation() {
        let config = MetricsConfig {
            port: 9091, // Use different port to avoid conflicts
            environment: "test".to_string(),
            max_operation_labels: 25,
            ip_allowlist: Some(vec!["127.0.0.1".to_string(), "::1".to_string()]),
            detailed_metrics: true,
        };
        
        let manager = MetricsManager::new(config.clone()).unwrap();
        
        assert_eq!(manager.config().port, 9091);
        assert_eq!(manager.config().environment, "test");
        assert_eq!(manager.config().max_operation_labels, 25);
        assert!(manager.config().detailed_metrics);
        
        // Test IP allowlist
        assert!(manager.is_ip_allowed("127.0.0.1"));
        assert!(manager.is_ip_allowed("::1"));
        assert!(!manager.is_ip_allowed("192.168.1.1"));
    }
    
    #[test]
    fn test_metrics_manager_no_allowlist() {
        let config = MetricsConfig {
            port: 9092,
            ip_allowlist: None,
            ..Default::default()
        };
        
        let manager = MetricsManager::new(config).unwrap();
        
        // Without allowlist, all IPs should be allowed
        assert!(manager.is_ip_allowed("127.0.0.1"));
        assert!(manager.is_ip_allowed("192.168.1.1"));
        assert!(manager.is_ip_allowed("10.0.0.1"));
    }
    
    #[test]
    fn test_extract_client_ip() {
        let mut headers = HeaderMap::new();
        
        // Test X-Forwarded-For
        headers.insert("x-forwarded-for", HeaderValue::from_static("192.168.1.1, 10.0.0.1"));
        assert_eq!(extract_client_ip(&headers), "192.168.1.1");
        
        // Test X-Real-IP (should be ignored when X-Forwarded-For exists)
        headers.insert("x-real-ip", HeaderValue::from_static("172.16.0.1"));
        assert_eq!(extract_client_ip(&headers), "192.168.1.1");
        
        // Test only X-Real-IP
        headers.remove("x-forwarded-for");
        assert_eq!(extract_client_ip(&headers), "172.16.0.1");
        
        // Test no headers
        headers.clear();
        assert_eq!(extract_client_ip(&headers), "unknown");
    }
    
    #[test]
    fn test_endpoint_url_generation() {
        let config = MetricsConfig {
            port: 9093,
            ..Default::default()
        };
        
        let manager = MetricsManager::new(config).unwrap();
        assert_eq!(manager.endpoint_url(), "http://0.0.0.0:9093/metrics");
    }
    
    #[test]
    fn test_cardinality_limiter_integration() {
        let config = MetricsConfig {
            port: 9094,
            max_operation_labels: 3,
            ..Default::default()
        };
        
        let manager = MetricsManager::new(config).unwrap();
        let limiter = manager.cardinality_limiter();
        
        // Test that the limiter has the correct max operations
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Add operations up to limit
            assert_eq!(limiter.get_operation_label("op1").await, "op1");
            assert_eq!(limiter.get_operation_label("op2").await, "op2");
            assert_eq!(limiter.get_operation_label("op3").await, "op3");
            
            // Exceed limit - should return "other"
            assert_eq!(limiter.get_operation_label("op4").await, "other");
            
            assert_eq!(limiter.operation_count().await, 3);
        });
    }
}
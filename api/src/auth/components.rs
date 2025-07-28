//! Authorization components for dependency injection
//!
//! This module provides a unified way to create and manage all authorization
//! system components, making it easy to wire them into the GraphQL context
//! and maintain consistent configuration across the application.

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

use crate::auth::cache::{AuthCache, ProductionAuthCache, MockAuthCache, CacheConfig};
use crate::auth::fallback::FallbackAuthorizer;
use crate::middleware::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::services::spicedb::{SpiceDBClient, SpiceDBClientTrait, SpiceDBConfig, SpiceDBStats};
use crate::config::AuthorizationConfig;

/// Bundle of all authorization system components
///
/// This struct contains all the components needed for the authorization system
/// to function properly. It's designed to be injected into the GraphQL context
/// via the `.data()` method during schema creation.
#[derive(Clone)]
pub struct AuthorizationComponents {
    /// SpiceDB client for permission checking
    pub spicedb: Arc<dyn SpiceDBClientTrait>,
    /// Authorization cache for performance
    pub cache: Arc<dyn AuthCache>,
    /// Circuit breaker for resilience
    pub circuit_breaker: Arc<CircuitBreaker>,
    /// Fallback authorization rules
    pub fallback: Arc<FallbackAuthorizer>,
}

impl AuthorizationComponents {
    /// Create production authorization components
    pub async fn new_production(config: &AuthorizationConfig) -> Result<Self> {
        // Create SpiceDB client
        let spicedb_config = SpiceDBConfig {
            endpoint: config.spicedb_endpoint.clone(),
            preshared_key: config.spicedb_preshared_key.clone(),
            request_timeout: Duration::from_millis(config.circuit_breaker_timeout_ms),
            connect_timeout: Duration::from_secs(2),
            max_connections: 10,
        };
        let spicedb: Arc<dyn SpiceDBClientTrait> = Arc::new(
            SpiceDBClient::new(spicedb_config).await?
        );

        // Create authorization cache
        let cache_config = CacheConfig {
            max_entries: config.cache_max_entries,
            default_ttl: Duration::from_secs(config.cache_ttl_seconds),
            cleanup_interval: Duration::from_secs(60),
            extended_ttl: Duration::from_secs(config.cache_ttl_seconds * 2), // Extended TTL for fallback
        };
        let cache: Arc<dyn AuthCache> = Arc::new(
            ProductionAuthCache::new(cache_config)
        );

        // Create circuit breaker
        let circuit_breaker_config = CircuitBreakerConfig {
            failure_threshold: config.circuit_breaker_failure_threshold,
            success_threshold: 2,
            timeout: Duration::from_millis(config.circuit_breaker_timeout_ms),
            half_open_timeout: Duration::from_secs(config.circuit_breaker_retry_timeout_seconds),
        };
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_breaker_config));

        // Create fallback authorizer
        let fallback = Arc::new(FallbackAuthorizer::new());

        Ok(Self {
            spicedb,
            cache,
            circuit_breaker,
            fallback,
        })
    }

    /// Create test/mock authorization components
    pub fn new_mock() -> Self {
        // Mock SpiceDB client that allows demo_user all actions
        let spicedb = Arc::new(MockSpiceDBClient::new());
        
        // Mock cache for testing
        let cache = Arc::new(MockAuthCache::new());

        // Test circuit breaker that never opens
        let circuit_breaker_config = CircuitBreakerConfig {
            failure_threshold: 1000, // Very high threshold
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_secs(1),
        };
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_breaker_config));

        // Real fallback authorizer (it's stateless)
        let fallback = Arc::new(FallbackAuthorizer::new());

        Self {
            spicedb,
            cache,
            circuit_breaker,
            fallback,
        }
    }

    /// Create demo mode authorization components
    pub async fn new_demo(config: &AuthorizationConfig) -> Result<Self> {
        // Use mock SpiceDB client in demo mode
        let spicedb = Arc::new(MockSpiceDBClient::new());

        // Use production cache but with shorter TTL
        let cache_config = CacheConfig {
            max_entries: config.cache_max_entries.min(1000), // Limit cache size in demo
            default_ttl: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(30),
            extended_ttl: Duration::from_secs(120), // Shorter TTL in demo
        };
        let cache: Arc<dyn AuthCache> = Arc::new(
            ProductionAuthCache::new(cache_config)
        );

        // Use lenient circuit breaker settings in demo
        let circuit_breaker_config = CircuitBreakerConfig {
            failure_threshold: 10, // More lenient
            success_threshold: 2,
            timeout: Duration::from_millis(config.circuit_breaker_timeout_ms),
            half_open_timeout: Duration::from_secs(30), // Shorter retry timeout
        };
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_breaker_config));

        // Real fallback authorizer
        let fallback = Arc::new(FallbackAuthorizer::new());

        Ok(Self {
            spicedb,
            cache,
            circuit_breaker,
            fallback,
        })
    }

    /// Get component statistics for monitoring
    pub async fn get_stats(&self) -> AuthorizationStats {
        let cache_stats = self.cache.stats().await;
        // For now, use dummy circuit breaker stats (will be fixed when proper interface is implemented)
        let cache_size = self.cache.size().await;

        AuthorizationStats {
            cache_hits: cache_stats.hits,
            cache_misses: cache_stats.misses,
            cache_size,
            circuit_breaker_state: "closed".to_string(), // Placeholder
            circuit_breaker_failures: 0, // Placeholder
            fallback_checks: 0, // Placeholder - FallbackAuthorizer doesn't track stats yet
            fallback_allowed: 0, // Placeholder
        }
    }
}

/// Statistics about the authorization system
#[derive(Debug, Clone)]
pub struct AuthorizationStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_size: usize,
    pub circuit_breaker_state: String,
    pub circuit_breaker_failures: u64,
    pub fallback_checks: u64,
    pub fallback_allowed: u64,
}

/// Mock SpiceDB client for testing and demo mode
pub struct MockSpiceDBClient {
    // In a real implementation, this might store test scenarios
}

impl MockSpiceDBClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl SpiceDBClientTrait for MockSpiceDBClient {
    async fn check_permission(
        &self,
        request: crate::services::spicedb::CheckPermissionRequest,
    ) -> Result<bool, crate::services::spicedb::SpiceDBError> {
        // Allow all permissions for demo_user, deny for others
        if request.subject.starts_with("demo_user") || request.subject == "system" {
            Ok(true)
        } else {
            // For testing, we can implement more sophisticated logic here
            Ok(false)
        }
    }

    async fn health_check(&self) -> Result<bool, crate::services::spicedb::SpiceDBError> {
        Ok(true)
    }

    async fn stats(&self) -> SpiceDBStats {
        SpiceDBStats {
            total_checks: 0,
            successful_checks: 0,
            failed_checks: 0,
            avg_response_time_ms: 0.0,
            timeouts: 0,
            connection_errors: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_components_creation() {
        let components = AuthorizationComponents::new_mock();
        
        // Test that all components are created
        assert!(Arc::strong_count(&components.spicedb) >= 1);
        assert!(Arc::strong_count(&components.cache) >= 1);
        assert!(Arc::strong_count(&components.circuit_breaker) >= 1);
        assert!(Arc::strong_count(&components.fallback) >= 1);
    }

    #[tokio::test]
    async fn test_mock_spicedb_client() {
        let client = MockSpiceDBClient::new();
        
        // Test that demo_user is allowed
        let request = crate::services::spicedb::CheckPermissionRequest {
            subject: "demo_user".to_string(),
            resource: "notes:test".to_string(),
            permission: "read".to_string(),
        };
        
        let result = client.check_permission(request).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Test that other users are denied
        let request = crate::services::spicedb::CheckPermissionRequest {
            subject: "other_user".to_string(),
            resource: "notes:test".to_string(),
            permission: "read".to_string(),
        };
        
        let result = client.check_permission(request).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_components_stats() {
        let components = AuthorizationComponents::new_mock();
        let stats = components.get_stats().await;
        
        // Stats should be initialized to sensible defaults
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.fallback_checks, 0);
    }
}
//! Health check integration for SpiceDB and circuit breaker monitoring
//!
//! This module provides comprehensive health monitoring for the authorization
//! system, including SpiceDB connectivity, circuit breaker state, and fallback
//! authorization availability.
//!
//! # Health Checks Provided
//!
//! 1. **SpiceDB Health**: Tests connectivity and response from SpiceDB service
//! 2. **Circuit Breaker Health**: Monitors circuit breaker state and statistics
//! 3. **Authorization System Health**: Overall authorization system status
//! 4. **Fallback Health**: Ensures fallback authorization is available
//!
//! # Integration with Health Manager
//!
//! This module integrates with the existing health management system to provide
//! real-time status updates that can be exposed via health endpoints.
//!
//! # Usage
//!
//! ```rust
//! use crate::services::spicedb::health::SpiceDBHealthChecker;
//! use crate::health::HealthManager;
//! use std::sync::Arc;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let health_manager = Arc::new(HealthManager::new());
//! let spicedb_client = create_spicedb_client().await?;
//! let circuit_breaker = Arc::new(CircuitBreaker::new(config));
//! let fallback = Arc::new(FallbackAuthorizer::new());
//!
//! let health_checker = SpiceDBHealthChecker::new(
//!     health_manager,
//!     spicedb_client,
//!     circuit_breaker,
//!     fallback,
//! );
//!
//! // Start periodic health checks
//! health_checker.start_periodic_checks().await;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, warn, error, info};

use crate::health::{HealthManager, HealthStatus};
use crate::services::spicedb::{SpiceDBClientTrait, SpiceDBError};
use crate::middleware::circuit_breaker::{CircuitBreaker, CircuitState};
use crate::auth::fallback::FallbackAuthorizer;

/// Health checker for SpiceDB and authorization system
#[derive(Clone)]
pub struct SpiceDBHealthChecker {
    health_manager: Arc<HealthManager>,
    spicedb_client: Arc<dyn SpiceDBClientTrait>,
    circuit_breaker: Arc<CircuitBreaker>,
    fallback_authorizer: Arc<FallbackAuthorizer>,
    check_interval: Duration,
}

impl SpiceDBHealthChecker {
    /// Create a new SpiceDB health checker
    ///
    /// # Arguments
    ///
    /// * `health_manager` - Health manager to report status to
    /// * `spicedb_client` - SpiceDB client to check
    /// * `circuit_breaker` - Circuit breaker to monitor
    /// * `fallback_authorizer` - Fallback authorizer to verify
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::spicedb::health::SpiceDBHealthChecker;
    /// use std::sync::Arc;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let health_checker = SpiceDBHealthChecker::new(
    ///     health_manager,
    ///     spicedb_client,
    ///     circuit_breaker,
    ///     fallback_authorizer,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        health_manager: Arc<HealthManager>,
        spicedb_client: Arc<dyn SpiceDBClientTrait>,
        circuit_breaker: Arc<CircuitBreaker>,
        fallback_authorizer: Arc<FallbackAuthorizer>,
    ) -> Self {
        Self {
            health_manager,
            spicedb_client,
            circuit_breaker,
            fallback_authorizer,
            check_interval: Duration::from_secs(30), // Check every 30 seconds
        }
    }

    /// Create health checker with custom check interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Start periodic health checks in the background
    ///
    /// This method spawns a background task that continuously monitors
    /// the health of the authorization system components and reports
    /// status to the health manager.
    pub async fn start_periodic_checks(&self) {
        let checker = self.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(checker.check_interval);
            
            info!(
                interval_secs = %checker.check_interval.as_secs(),
                "Starting SpiceDB health checks"
            );
            
            loop {
                interval.tick().await;
                
                // Perform all health checks
                checker.check_spicedb_health().await;
                checker.check_circuit_breaker_health().await;
                checker.check_fallback_health().await;
                checker.update_overall_authorization_health().await;
            }
        });
    }

    /// Perform immediate health check and return results
    ///
    /// This method performs all health checks synchronously and returns
    /// a summary of the current authorization system health.
    pub async fn check_all(&self) -> AuthorizationHealthSummary {
        let spicedb_healthy = self.check_spicedb_health().await;
        let circuit_healthy = self.check_circuit_breaker_health().await;
        let fallback_healthy = self.check_fallback_health().await;
        
        self.update_overall_authorization_health().await;
        
        AuthorizationHealthSummary {
            spicedb_healthy,
            circuit_breaker_healthy: circuit_healthy,
            fallback_healthy,
            overall_healthy: spicedb_healthy && circuit_healthy && fallback_healthy,
        }
    }

    /// Check SpiceDB connectivity and functionality
    async fn check_spicedb_health(&self) -> bool {
        debug!("Checking SpiceDB health");
        
        match self.spicedb_client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    self.health_manager.update_service_health(
                        "spicedb",
                        HealthStatus::Healthy,
                        "SpiceDB is responding and healthy".to_string(),
                    ).await;
                    
                    debug!("SpiceDB health check passed");
                    true
                } else {
                    self.health_manager.update_service_health(
                        "spicedb",
                        HealthStatus::Degraded,
                        "SpiceDB is responding but may have issues".to_string(),
                    ).await;
                    
                    warn!("SpiceDB health check indicates degraded service");
                    false
                }
            }
            Err(SpiceDBError::Timeout) => {
                self.health_manager.update_service_health(
                    "spicedb",
                    HealthStatus::Degraded,
                    "SpiceDB health check timed out".to_string(),
                ).await;
                
                warn!("SpiceDB health check timed out");
                false
            }
            Err(SpiceDBError::ConnectionError(msg)) => {
                self.health_manager.update_service_health(
                    "spicedb",
                    HealthStatus::Unhealthy,
                    format!("SpiceDB connection failed: {}", msg),
                ).await;
                
                error!("SpiceDB connection failed: {}", msg);
                false
            }
            Err(e) => {
                self.health_manager.update_service_health(
                    "spicedb",
                    HealthStatus::Unhealthy,
                    format!("SpiceDB health check failed: {}", e),
                ).await;
                
                error!("SpiceDB health check failed: {}", e);
                false
            }
        }
    }

    /// Check circuit breaker state and statistics
    async fn check_circuit_breaker_health(&self) -> bool {
        debug!("Checking circuit breaker health");
        
        let stats = self.circuit_breaker.stats().await;
        let state = stats.state;
        
        let (status, message, healthy) = match state {
            CircuitState::Closed => {
                let success_rate = stats.success_rate;
                if success_rate >= 95.0 {
                    (
                        HealthStatus::Healthy,
                        format!("Circuit breaker closed, success rate: {:.1}%", success_rate),
                        true,
                    )
                } else if success_rate >= 80.0 {
                    (
                        HealthStatus::Degraded,
                        format!("Circuit breaker closed but low success rate: {:.1}%", success_rate),
                        false,
                    )
                } else {
                    (
                        HealthStatus::Degraded,
                        format!("Circuit breaker closed but very low success rate: {:.1}%", success_rate),
                        false,
                    )
                }
            }
            CircuitState::HalfOpen => (
                HealthStatus::Degraded,
                format!(
                    "Circuit breaker in half-open state, testing recovery (successes: {})",
                    stats.current_success_count
                ),
                false,
            ),
            CircuitState::Open => {
                let time_until_half_open = stats.time_until_half_open
                    .map(|d| format!(" (retry in {}s)", d.as_secs()))
                    .unwrap_or_else(|| " (retry timing unknown)".to_string());
                
                (
                    HealthStatus::Unhealthy,
                    format!("Circuit breaker is open{}", time_until_half_open),
                    false,
                )
            }
        };
        
        self.health_manager.update_service_health("circuit_breaker", status, message).await;
        
        debug!(
            state = %state,
            success_rate = %stats.success_rate,
            total_operations = %stats.total_operations,
            "Circuit breaker health check completed"
        );
        
        healthy
    }

    /// Check fallback authorization availability
    async fn check_fallback_health(&self) -> bool {
        debug!("Checking fallback authorization health");
        
        // Test basic fallback functionality with a known-safe test case
        let test_cases = [
            ("user:health_test", "system:health:check", "read", true),
            ("user:alice", "notes:alice:test", "read", true),
            ("user:alice", "notes:bob:test", "read", false),
            ("user:alice", "notes:alice:test", "write", false),
        ];
        
        let mut all_passed = true;
        let mut failed_tests = Vec::new();
        
        for (subject, resource, action, expected) in test_cases {
            let result = self.fallback_authorizer.is_authorized(subject, resource, action);
            if result != expected {
                all_passed = false;
                failed_tests.push(format!("{}:{}:{} expected {} got {}", 
                    subject, resource, action, expected, result));
            }
        }
        
        if all_passed {
            self.health_manager.update_service_health(
                "fallback_authorization",
                HealthStatus::Healthy,
                "Fallback authorization is working correctly".to_string(),
            ).await;
            
            debug!("Fallback authorization health check passed");
            true
        } else {
            self.health_manager.update_service_health(
                "fallback_authorization",
                HealthStatus::Unhealthy,
                format!("Fallback authorization tests failed: {}", failed_tests.join(", ")),
            ).await;
            
            error!("Fallback authorization tests failed: {:?}", failed_tests);
            false
        }
    }

    /// Update overall authorization system health
    async fn update_overall_authorization_health(&self) {
        debug!("Updating overall authorization system health");
        
        let spicedb_healthy = self.is_service_healthy("spicedb").await;
        let circuit_healthy = self.is_service_healthy("circuit_breaker").await;
        let fallback_healthy = self.is_service_healthy("fallback_authorization").await;
        
        let (status, message) = if spicedb_healthy && circuit_healthy && fallback_healthy {
            (
                HealthStatus::Healthy,
                "Authorization system fully operational".to_string(),
            )
        } else if fallback_healthy {
            // If fallback is working, we can continue operating even if SpiceDB is down
            if !spicedb_healthy && circuit_healthy {
                (
                    HealthStatus::Degraded,
                    "Authorization system operating on fallback rules (SpiceDB unavailable)".to_string(),
                )
            } else if !spicedb_healthy && !circuit_healthy {
                (
                    HealthStatus::Degraded,
                    "Authorization system operating on fallback rules (SpiceDB and circuit breaker issues)".to_string(),
                )
            } else {
                (
                    HealthStatus::Degraded,
                    "Authorization system partially degraded".to_string(),
                )
            }
        } else {
            (
                HealthStatus::Unhealthy,
                "Authorization system is not operational (fallback authorization failed)".to_string(),
            )
        };
        
        self.health_manager.update_service_health("authorization", status.clone(), message).await;
        
        debug!(
            spicedb_healthy = %spicedb_healthy,
            circuit_healthy = %circuit_healthy,
            fallback_healthy = %fallback_healthy,
            overall_status = ?status,
            "Overall authorization health updated"
        );
    }

    /// Check if a specific service is healthy
    async fn is_service_healthy(&self, service_name: &str) -> bool {
        let health = self.health_manager.get_health().await;
        health.services.get(service_name)
            .map(|service| matches!(service.status, HealthStatus::Healthy))
            .unwrap_or(false)
    }

    /// Get detailed authorization system statistics
    pub async fn get_detailed_stats(&self) -> AuthorizationStats {
        let spicedb_stats = self.spicedb_client.stats().await;
        let circuit_stats = self.circuit_breaker.stats().await;
        let health = self.health_manager.get_health().await;
        
        AuthorizationStats {
            spicedb_stats,
            circuit_breaker_stats: circuit_stats,
            service_health: health.services,
            uptime_seconds: health.uptime_seconds,
            last_check: health.timestamp,
        }
    }

    /// Force immediate update of all health checks
    pub async fn force_update(&self) {
        info!("Forcing immediate update of all authorization health checks");
        
        // Run all checks in parallel for faster update
        let (spicedb_result, circuit_result, fallback_result) = tokio::join!(
            self.check_spicedb_health(),
            self.check_circuit_breaker_health(),
            self.check_fallback_health(),
        );
        
        self.update_overall_authorization_health().await;
        
        info!(
            spicedb_healthy = %spicedb_result,
            circuit_healthy = %circuit_result,
            fallback_healthy = %fallback_result,
            "Forced health check update completed"
        );
    }
}

/// Summary of authorization system health
#[derive(Debug, Clone)]
pub struct AuthorizationHealthSummary {
    /// Whether SpiceDB is healthy
    pub spicedb_healthy: bool,
    /// Whether circuit breaker is healthy
    pub circuit_breaker_healthy: bool,
    /// Whether fallback authorization is healthy
    pub fallback_healthy: bool,
    /// Overall authorization system health
    pub overall_healthy: bool,
}

/// Detailed authorization system statistics
#[derive(Debug, Clone)]
pub struct AuthorizationStats {
    /// SpiceDB client statistics
    pub spicedb_stats: crate::services::spicedb::SpiceDBStats,
    /// Circuit breaker statistics
    pub circuit_breaker_stats: crate::middleware::circuit_breaker::CircuitBreakerStats,
    /// Health status of all services
    pub service_health: std::collections::HashMap<String, crate::health::ServiceHealth>,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Timestamp of last health check
    pub last_check: std::time::SystemTime,
}

/// Create health checker from environment variables
///
/// This function creates health checker configuration from environment variables.
///
/// # Environment Variables
///
/// * `SPICEDB_HEALTH_CHECK_INTERVAL` - Health check interval in seconds (default: 30)
pub fn create_health_checker_from_env(
    health_manager: Arc<HealthManager>,
    spicedb_client: Arc<dyn SpiceDBClientTrait>,
    circuit_breaker: Arc<CircuitBreaker>,
    fallback_authorizer: Arc<FallbackAuthorizer>,
) -> SpiceDBHealthChecker {
    let check_interval = std::env::var("SPICEDB_HEALTH_CHECK_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));

    SpiceDBHealthChecker::new(
        health_manager,
        spicedb_client,
        circuit_breaker,
        fallback_authorizer,
    )
    .with_interval(check_interval)
}

// TODO: Enable tests when SpiceDB dependencies and MockClient are available
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::services::spicedb::tests::MockSpiceDBClient;
//     use crate::middleware::circuit_breaker::CircuitBreakerConfig;
//     use std::time::Duration;

//     async fn create_test_health_checker() -> SpiceDBHealthChecker {
//         let health_manager = Arc::new(HealthManager::new());
//         let spicedb_client = Arc::new(MockSpiceDBClient::new()) as Arc<dyn SpiceDBClientTrait>;
//         let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
//         let fallback_authorizer = Arc::new(FallbackAuthorizer::new());

//         SpiceDBHealthChecker::new(
//             health_manager,
//             spicedb_client,
//             circuit_breaker,
//             fallback_authorizer,
//         )
//         .with_interval(Duration::from_millis(100)) // Fast for testing
//     }

//     #[tokio::test]
//     async fn test_health_checker_creation() {
//         let checker = create_test_health_checker().await;
//         assert_eq!(checker.check_interval, Duration::from_millis(100));
//     }

//     ... (commented out remaining tests)
// }
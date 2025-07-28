use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::services::database::DatabaseError;

/// Database connection state for health monitoring
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Database is connected and operational
    Connected,
    /// Database is connecting (startup or reconnecting)
    Connecting,
    /// Database connection failed at the given timestamp
    Failed(Instant),
    /// Database has never connected
    Disconnected,
}

impl ConnectionState {
    /// Check if the connection is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }
    
    /// Check if the connection is in a connecting state
    pub fn is_connecting(&self) -> bool {
        matches!(self, ConnectionState::Connecting)
    }
    
    /// Get the time when the connection failed (if applicable)
    pub fn failed_at(&self) -> Option<Instant> {
        match self {
            ConnectionState::Failed(timestamp) => Some(*timestamp),
            _ => None,
        }
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
}

/// Configuration for health monitoring
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// How long to wait before returning 503 when database is unavailable
    pub unavailable_timeout: Duration,
    /// How often to check connection health in the background
    pub health_check_interval: Duration,
    /// How long to suggest clients wait before retrying (for 503 responses)
    pub retry_after_seconds: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            unavailable_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            retry_after_seconds: 60,
        }
    }
}

/// Health monitor for database connections
pub struct DatabaseHealthMonitor {
    state: Arc<RwLock<ConnectionState>>,
    config: HealthConfig,
}

impl DatabaseHealthMonitor {
    pub fn new(config: HealthConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(ConnectionState::default())),
            config,
        }
    }
    
    /// Update the connection state to connected
    pub async fn mark_connected(&self) {
        let mut state = self.state.write().await;
        *state = ConnectionState::Connected;
    }
    
    /// Update the connection state to connecting
    pub async fn mark_connecting(&self) {
        let mut state = self.state.write().await;
        *state = ConnectionState::Connecting;
    }
    
    /// Update the connection state to failed
    pub async fn mark_failed(&self) {
        let mut state = self.state.write().await;
        *state = ConnectionState::Failed(Instant::now());
    }
    
    /// Get the current connection state
    pub async fn connection_state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }
    
    /// Check if database operations should return ServiceUnavailable
    pub async fn should_return_unavailable(&self) -> Option<u64> {
        let state = self.state.read().await;
        
        match *state {
            ConnectionState::Failed(failed_at) => {
                let elapsed = failed_at.elapsed();
                if elapsed > self.config.unavailable_timeout {
                    Some(self.config.retry_after_seconds)
                } else {
                    None
                }
            }
            ConnectionState::Disconnected => {
                // If we've never connected, return unavailable immediately
                Some(self.config.retry_after_seconds)
            }
            _ => None,
        }
    }
    
    /// Check if the database is healthy for accepting operations
    pub async fn is_healthy(&self) -> bool {
        let state = self.state.read().await;
        state.is_healthy()
    }
    
    /// Get time since last connection failure (if any)
    pub async fn time_since_failure(&self) -> Option<Duration> {
        let state = self.state.read().await;
        state.failed_at().map(|instant| instant.elapsed())
    }
    
    /// Create a health check result for external monitoring
    pub async fn health_check_result(&self) -> HealthCheckResult {
        let state = self.state.read().await;
        
        match *state {
            ConnectionState::Connected => HealthCheckResult {
                status: HealthStatus::Healthy,
                message: "Database connection is healthy".to_string(),
                last_failure: None,
                uptime: None, // Would track this in real implementation
            },
            ConnectionState::Connecting => HealthCheckResult {
                status: HealthStatus::Warning,
                message: "Database is connecting".to_string(),
                last_failure: None,
                uptime: None,
            },
            ConnectionState::Failed(failed_at) => {
                let elapsed = failed_at.elapsed();
                let is_unavailable = elapsed > self.config.unavailable_timeout;
                
                HealthCheckResult {
                    status: if is_unavailable { HealthStatus::Critical } else { HealthStatus::Warning },
                    message: format!("Database connection failed {} seconds ago", elapsed.as_secs()),
                    last_failure: Some(elapsed),
                    uptime: None,
                }
            },
            ConnectionState::Disconnected => HealthCheckResult {
                status: HealthStatus::Critical,
                message: "Database has never connected".to_string(),
                last_failure: None,
                uptime: None,
            },
        }
    }
}

/// Health status levels
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Minor issues but service is still available
    Warning,
    /// Major issues, service may be degraded
    Critical,
}

/// Health check result for external monitoring
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub message: String,
    pub last_failure: Option<Duration>,
    pub uptime: Option<Duration>,
}

/// Helper function to check database availability and wrap operations
/// 
/// This should be called before any database operation to check if we should
/// return a 503 Service Unavailable response instead of attempting the operation
pub async fn check_database_availability<T, F>(
    monitor: &DatabaseHealthMonitor,
    operation: F,
) -> Result<T, DatabaseError>
where
    F: std::future::Future<Output = Result<T, DatabaseError>>,
{
    // Check if we should return unavailable
    if let Some(retry_after) = monitor.should_return_unavailable().await {
        return Err(DatabaseError::ServiceUnavailable { retry_after });
    }
    
    // If database is available, execute the operation
    operation.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    
    fn test_config() -> HealthConfig {
        HealthConfig {
            unavailable_timeout: Duration::from_millis(100), // Short timeout for testing
            health_check_interval: Duration::from_millis(50),
            retry_after_seconds: 30,
        }
    }
    
    #[tokio::test]
    async fn test_connection_state_transitions() {
        let monitor = DatabaseHealthMonitor::new(test_config());
        
        // Start disconnected
        assert_eq!(monitor.connection_state().await, ConnectionState::Disconnected);
        assert!(!monitor.is_healthy().await);
        
        // Mark connecting
        monitor.mark_connecting().await;
        assert_eq!(monitor.connection_state().await, ConnectionState::Connecting);
        assert!(!monitor.is_healthy().await);
        
        // Mark connected
        monitor.mark_connected().await;
        assert_eq!(monitor.connection_state().await, ConnectionState::Connected);
        assert!(monitor.is_healthy().await);
        
        // Mark failed
        monitor.mark_failed().await;
        let state = monitor.connection_state().await;
        assert!(matches!(state, ConnectionState::Failed(_)));
        assert!(!monitor.is_healthy().await);
    }
    
    #[tokio::test]
    async fn test_service_unavailable_after_timeout() {
        let monitor = DatabaseHealthMonitor::new(test_config());
        
        // Mark as failed
        monitor.mark_failed().await;
        
        // Should not be unavailable immediately
        assert!(monitor.should_return_unavailable().await.is_none());
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Should be unavailable now
        let retry_after = monitor.should_return_unavailable().await;
        assert_eq!(retry_after, Some(30));
    }
    
    #[tokio::test]
    async fn test_disconnected_is_immediately_unavailable() {
        let monitor = DatabaseHealthMonitor::new(test_config());
        
        // Should be unavailable when disconnected
        let retry_after = monitor.should_return_unavailable().await;
        assert_eq!(retry_after, Some(30));
    }
    
    #[tokio::test]
    async fn test_check_database_availability() {
        let monitor = DatabaseHealthMonitor::new(test_config());
        
        // When connected, operation should succeed
        monitor.mark_connected().await;
        let result = check_database_availability(&monitor, async { 
            Ok::<String, DatabaseError>("success".to_string())
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        
        // When disconnected, should return ServiceUnavailable
        monitor.mark_failed().await;
        sleep(Duration::from_millis(150)).await;
        
        let result = check_database_availability(&monitor, async { 
            Ok::<String, DatabaseError>("should not reach".to_string())
        }).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            DatabaseError::ServiceUnavailable { retry_after } => {
                assert_eq!(retry_after, 30);
            }
            _ => panic!("Expected ServiceUnavailable error"),
        }
    }
    
    #[tokio::test]
    async fn test_health_check_result() {
        let monitor = DatabaseHealthMonitor::new(test_config());
        
        // Connected state
        monitor.mark_connected().await;
        let result = monitor.health_check_result().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.contains("healthy"));
        
        // Failed state (before timeout)
        monitor.mark_failed().await;
        let result = monitor.health_check_result().await;
        assert_eq!(result.status, HealthStatus::Warning);
        assert!(result.message.contains("failed"));
        
        // Failed state (after timeout)
        sleep(Duration::from_millis(150)).await;
        let result = monitor.health_check_result().await;
        assert_eq!(result.status, HealthStatus::Critical);
        assert!(result.message.contains("failed"));
    }
    
    #[tokio::test]
    async fn test_time_since_failure() {
        let monitor = DatabaseHealthMonitor::new(test_config());
        
        // No failure initially
        assert!(monitor.time_since_failure().await.is_none());
        
        // Mark as failed
        monitor.mark_failed().await;
        sleep(Duration::from_millis(50)).await;
        
        // Should have a failure time
        let time_since = monitor.time_since_failure().await;
        assert!(time_since.is_some());
        assert!(time_since.unwrap() >= Duration::from_millis(40));
    }
}
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use futures::future::BoxFuture;
use crate::services::database::DatabaseError;

/// Exponential backoff with jitter for retry logic
pub struct ExponentialBackoff {
    attempt: u32,
    max_delay: Duration,
    base_delay: Duration,
    jitter: bool,
}

impl ExponentialBackoff {
    pub fn new() -> Self {
        Self {
            attempt: 0,
            max_delay: Duration::from_secs(60),
            base_delay: Duration::from_secs(1),
            jitter: true,
        }
    }
    
    pub fn next_delay(&mut self) -> Duration {
        let exp_delay = self.base_delay * 2u32.pow(self.attempt.min(6));
        let delay = exp_delay.min(self.max_delay);
        
        self.attempt += 1;
        
        if self.jitter {
            let jitter_ms = rand::random::<u64>() % 1000;
            delay + Duration::from_millis(jitter_ms)
        } else {
            delay
        }
    }
    
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection pool configuration
#[derive(Clone)]
pub struct PoolConfig {
    pub min_connections: usize,
    pub max_connections: usize,
    pub idle_timeout: Duration,
    pub acquire_timeout: Duration,
    pub health_check_interval: Duration,
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            acquire_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(30),
            max_lifetime: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Pooled connection wrapper
pub struct PooledConnection {
    pub id: String,
    pub created_at: Instant,
    pub last_used: Instant,
    pub is_healthy: bool,
}

impl PooledConnection {
    pub fn new(id: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            created_at: now,
            last_used: now,
            is_healthy: true,
        }
    }
    
    pub fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }
    
    pub fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }
    
    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }
}

/// Pool metrics for monitoring
pub struct PoolMetrics {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub failed_connections: usize,
}

impl PoolMetrics {
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            failed_connections: 0,
        }
    }
}

impl Default for PoolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Health monitor for the connection pool
pub struct HealthMonitor {
    pub last_check: Option<Instant>,
    pub consecutive_failures: u32,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            last_check: None,
            consecutive_failures: 0,
        }
    }
    
    pub fn record_success(&mut self) {
        self.last_check = Some(Instant::now());
        self.consecutive_failures = 0;
    }
    
    pub fn record_failure(&mut self) {
        self.last_check = Some(Instant::now());
        self.consecutive_failures += 1;
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection pool with health monitoring and retry logic
pub struct ConnectionPool {
    config: PoolConfig,
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    metrics: Arc<RwLock<PoolMetrics>>,
    health_monitor: Arc<RwLock<HealthMonitor>>,
}

impl ConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        
        Self {
            config,
            connections: Arc::new(RwLock::new(Vec::new())),
            semaphore,
            metrics: Arc::new(RwLock::new(PoolMetrics::new())),
            health_monitor: Arc::new(RwLock::new(HealthMonitor::new())),
        }
    }
    
    pub async fn initialize(&self) -> Result<(), DatabaseError> {
        // Pre-warm pool with minimum connections
        for i in 0..self.config.min_connections {
            self.create_connection(&format!("init-{}", i)).await?;
        }
        
        // Start background health monitoring (in a real implementation)
        self.start_health_monitor().await;
        
        Ok(())
    }
    
    pub async fn health(&self) -> PoolMetrics {
        let connections = self.connections.read().await;
        let active = connections.iter().filter(|c| c.is_healthy).count();
        let idle = connections.len() - active;
        
        PoolMetrics {
            total_connections: connections.len(),
            active_connections: active,
            idle_connections: idle,
            failed_connections: 0, // Would be tracked in real implementation
        }
    }
    
    async fn create_connection(&self, id: &str) -> Result<(), DatabaseError> {
        let mut connections = self.connections.write().await;
        connections.push(PooledConnection::new(id.to_string()));
        Ok(())
    }
    
    async fn start_health_monitor(&self) {
        // In a real implementation, this would spawn a background task
        // For now, just mark that monitoring is started
        let mut monitor = self.health_monitor.write().await;
        monitor.record_success();
    }
}

/// Retry logic with exponential backoff and configurable timeouts
pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    operation_name: &str,
    is_startup: bool,
) -> Result<T, E>
where
    F: FnMut() -> BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    let mut backoff = ExponentialBackoff::new();
    let start_time = Instant::now();
    
    loop {
        match operation().await {
            Ok(result) => {
                tracing::info!("{} succeeded after {:?}", operation_name, start_time.elapsed());
                return Ok(result);
            }
            Err(err) => {
                let delay = backoff.next_delay();
                
                // Check configurable timeout (default 30s for operations, 10 min for startup)
                let max_duration = if is_startup {
                    Duration::from_secs(
                        std::env::var("STARTUP_MAX_WAIT")
                            .unwrap_or_else(|_| "600".to_string())
                            .parse()
                            .unwrap_or(600)
                    )
                } else {
                    Duration::from_secs(
                        std::env::var("DB_OPERATION_TIMEOUT")
                            .unwrap_or_else(|_| "30".to_string())
                            .parse()
                            .unwrap_or(30)
                    )
                };
                
                if start_time.elapsed() > max_duration {
                    tracing::error!("{} failed after {:?}: {}", operation_name, max_duration, err);
                    return Err(err);
                }
                
                tracing::warn!(
                    "{} failed (attempt {}): {}. Retrying in {:?}",
                    operation_name, 
                    backoff.attempt, 
                    err, 
                    delay
                );
                
                tokio::time::sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;
    
    #[tokio::test]
    async fn test_exponential_backoff_with_jitter() {
        let mut backoff = ExponentialBackoff::new();
        
        // Verify sequence approximately follows: 1s, 2s, 4s, 8s, 16s, 32s, 60s (max)
        // Allow tolerance for jitter (up to +1000ms)
        let delays: Vec<_> = (0..10).map(|_| backoff.next_delay()).collect();
        
        // Check exponential growth with max cap and jitter tolerance
        assert!(delays[0] >= Duration::from_secs(1));
        assert!(delays[0] <= Duration::from_millis(2000)); // Base + jitter
        assert!(delays[1] >= Duration::from_secs(2));
        assert!(delays[1] <= Duration::from_millis(3000)); // Base + jitter
        
        // Verify it caps at max_delay (60s + jitter)
        assert!(delays[6] <= Duration::from_millis(61000)); // 60s + max jitter
        assert!(delays[8] <= Duration::from_millis(61000));
        assert!(delays[9] <= Duration::from_millis(61000));
    }
    
    #[tokio::test]
    async fn test_exponential_backoff_reset() {
        let mut backoff = ExponentialBackoff::new();
        
        // Generate some delays
        let _first = backoff.next_delay();
        let _second = backoff.next_delay();
        assert_eq!(backoff.attempt, 2);
        
        // Reset should restart from beginning
        backoff.reset();
        assert_eq!(backoff.attempt, 0);
        
        let next_delay = backoff.next_delay();
        assert!(next_delay >= Duration::from_secs(1));
        assert!(next_delay < Duration::from_millis(2000));
    }
    
    #[tokio::test]
    async fn test_exponential_backoff_without_jitter() {
        let mut backoff = ExponentialBackoff {
            attempt: 0,
            max_delay: Duration::from_secs(60),
            base_delay: Duration::from_secs(1),
            jitter: false,
        };
        
        let delays: Vec<_> = (0..5).map(|_| backoff.next_delay()).collect();
        
        // Without jitter, should be exact powers of 2
        assert_eq!(delays[0], Duration::from_secs(1));
        assert_eq!(delays[1], Duration::from_secs(2));
        assert_eq!(delays[2], Duration::from_secs(4));
        assert_eq!(delays[3], Duration::from_secs(8));
        assert_eq!(delays[4], Duration::from_secs(16));
    }
    
    #[tokio::test]
    async fn test_connection_pool_sizing() {
        let config = PoolConfig {
            min_connections: 2,
            max_connections: 10,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(config);
        pool.initialize().await.unwrap();
        
        let health = pool.health().await;
        assert_eq!(health.total_connections, 2); // Min connections created
        assert_eq!(health.active_connections, 2); // All should be healthy initially
    }
    
    #[tokio::test]
    async fn test_connection_pool_default_config() {
        let config = PoolConfig::default();
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
    }
    
    #[tokio::test]
    async fn test_pooled_connection_lifecycle() {
        let conn = PooledConnection::new("test-conn".to_string());
        
        assert_eq!(conn.id, "test-conn");
        assert!(conn.is_healthy);
        assert!(!conn.is_expired(Duration::from_secs(3600)));
        assert!(!conn.is_idle(Duration::from_secs(300)));
        
        // Sleep to ensure time passes, then test expiration with very short lifetime
        tokio::time::sleep(Duration::from_millis(2)).await;
        assert!(conn.is_expired(Duration::from_millis(1)));
    }
    
    #[tokio::test]
    async fn test_pooled_connection_mark_used() {
        let mut conn = PooledConnection::new("test-conn".to_string());
        let initial_time = conn.last_used;
        
        // Sleep a bit to ensure time difference
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        conn.mark_used();
        assert!(conn.last_used > initial_time);
    }
    
    #[tokio::test]
    async fn test_health_monitor_lifecycle() {
        let mut monitor = HealthMonitor::new();
        
        assert!(monitor.last_check.is_none());
        assert_eq!(monitor.consecutive_failures, 0);
        
        monitor.record_failure();
        assert!(monitor.last_check.is_some());
        assert_eq!(monitor.consecutive_failures, 1);
        
        monitor.record_failure();
        assert_eq!(monitor.consecutive_failures, 2);
        
        monitor.record_success();
        assert_eq!(monitor.consecutive_failures, 0);
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        // Clean up any environment variables from other tests
        unsafe {
            std::env::remove_var("DB_OPERATION_TIMEOUT");
            std::env::remove_var("STARTUP_MAX_WAIT");
        }
        
        let operation_name = "test_operation";
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        
        let result = retry_with_backoff(
            {
                let call_count = Arc::clone(&call_count);
                move || {
                    let count = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    Box::pin(async move {
                        if count < 3 {
                            Err("simulated failure")
                        } else {
                            Ok("success")
                        }
                    })
                }
            },
            operation_name,
            false,
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff_timeout() {
        unsafe {
            std::env::set_var("DB_OPERATION_TIMEOUT", "1"); // 1 second timeout
        }
        
        let operation_name = "test_timeout";
        let start_time = Instant::now();
        
        let result = retry_with_backoff(
            || {
                Box::pin(async move {
                    Err::<(), &str>("always fails")
                })
            },
            operation_name,
            false,
        ).await;
        
        assert!(result.is_err());
        assert!(start_time.elapsed() >= Duration::from_secs(1));
        assert!(start_time.elapsed() < Duration::from_secs(5)); // Should not take too long
        
        // Clean up
        unsafe {
            std::env::remove_var("DB_OPERATION_TIMEOUT");
        }
    }
    
    #[tokio::test]
    async fn test_retry_startup_vs_operation_timeout() {
        // Test startup timeout (should be longer)
        unsafe {
            std::env::set_var("STARTUP_MAX_WAIT", "2");
            std::env::set_var("DB_OPERATION_TIMEOUT", "1");
        }
        
        let start_time = Instant::now();
        
        let result = retry_with_backoff(
            || {
                Box::pin(async move {
                    Err::<(), &str>("always fails")
                })
            },
            "startup_test",
            true, // is_startup = true
        ).await;
        
        assert!(result.is_err());
        // Should use startup timeout (2s), not operation timeout (1s)
        // Allow some tolerance for timing and exponential backoff delays
        assert!(start_time.elapsed() >= Duration::from_secs(2));
        assert!(start_time.elapsed() < Duration::from_secs(8)); // More tolerance for backoff timing
        
        // Clean up
        unsafe {
            std::env::remove_var("STARTUP_MAX_WAIT");
            std::env::remove_var("DB_OPERATION_TIMEOUT");
        }
    }
}
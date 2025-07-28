//! Circuit breaker implementation for resilient service calls
//!
//! This module provides a circuit breaker pattern implementation to protect
//! against cascading failures when calling external services like SpiceDB.
//!
//! # Circuit Breaker States
//!
//! The circuit breaker operates in three states:
//!
//! 1. **Closed**: Normal operation - all calls pass through
//! 2. **Open**: Failure mode - all calls fail immediately
//! 3. **HalfOpen**: Recovery testing - limited calls allowed to test service health
//!
//! # State Transitions
//!
//! - **Closed → Open**: After reaching failure threshold
//! - **Open → HalfOpen**: After timeout period expires
//! - **HalfOpen → Closed**: After reaching success threshold
//! - **HalfOpen → Open**: On any failure during recovery
//!
//! # Usage
//!
//! ```rust
//! use crate::middleware::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = CircuitBreakerConfig {
//!     failure_threshold: 3,
//!     success_threshold: 2,
//!     timeout: Duration::from_secs(1),
//!     half_open_timeout: Duration::from_secs(30),
//! };
//!
//! let breaker = CircuitBreaker::new(config);
//!
//! let result = breaker.call(|| {
//!     Box::pin(async {
//!         // Your service call here
//!         Ok::<_, String>("success")
//!     })
//! }).await;
//!
//! match result {
//!     Ok(value) => println!("Success: {}", value),
//!     Err(e) => println!("Failed: {}", e),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Performance Considerations
//!
//! - State checks use read locks for high concurrency
//! - State updates use write locks only when necessary
//! - Timeouts are checked efficiently using Instant comparisons
//! - Metrics are updated atomically for thread safety

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn, info, error};
use futures::Future;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - calls pass through
    Closed,
    /// Failure mode - calls fail immediately
    Open,
    /// Recovery testing - limited calls allowed
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes needed to close the circuit from half-open
    pub success_threshold: u32,
    /// Timeout for individual operations
    pub timeout: Duration,
    /// Duration to wait before transitioning from open to half-open
    pub half_open_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            half_open_timeout: Duration::from_secs(30),
        }
    }
}

/// Internal circuit breaker state
#[derive(Debug)]
struct CircuitBreakerState {
    /// Current circuit state
    state: CircuitState,
    /// Count of consecutive failures in closed state
    failure_count: u32,
    /// Count of consecutive successes in half-open state
    success_count: u32,
    /// Timestamp of last failure (used for half-open timeout)
    last_failure_time: Option<Instant>,
    /// Timestamp of last state change
    last_state_change: Instant,
    /// Total number of operations attempted
    total_operations: u64,
    /// Total number of successful operations
    successful_operations: u64,
    /// Total number of failed operations
    failed_operations: u64,
    /// Total number of operations rejected (circuit open)
    rejected_operations: u64,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            last_state_change: Instant::now(),
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            rejected_operations: 0,
        }
    }
}

/// Circuit breaker error
#[derive(Debug, Clone)]
pub enum CircuitBreakerError {
    /// Circuit is open - call rejected
    CircuitOpen,
    /// Operation timed out
    Timeout,
    /// Inner operation failed
    InnerError(String),
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::Timeout => write!(f, "Operation timed out"),
            CircuitBreakerError::InnerError(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

impl From<&str> for CircuitBreakerError {
    fn from(s: &str) -> Self {
        CircuitBreakerError::InnerError(s.to_string())
    }
}

impl From<String> for CircuitBreakerError {
    fn from(s: String) -> Self {
        CircuitBreakerError::InnerError(s)
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Current circuit state
    pub state: CircuitState,
    /// Total operations attempted
    pub total_operations: u64,
    /// Total successful operations
    pub successful_operations: u64,
    /// Total failed operations
    pub failed_operations: u64,
    /// Total rejected operations (circuit open)
    pub rejected_operations: u64,
    /// Success rate as percentage
    pub success_rate: f64,
    /// Current failure count (in closed state)
    pub current_failure_count: u32,
    /// Current success count (in half-open state)
    pub current_success_count: u32,
    /// Time since last state change
    pub time_since_last_state_change: Duration,
    /// Time until next half-open attempt (if in open state)
    pub time_until_half_open: Option<Duration>,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Circuit breaker configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::middleware::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    /// use std::time::Duration;
    ///
    /// let config = CircuitBreakerConfig {
    ///     failure_threshold: 3,
    ///     success_threshold: 2,
    ///     timeout: Duration::from_secs(1),
    ///     half_open_timeout: Duration::from_secs(30),
    /// };
    ///
    /// let breaker = CircuitBreaker::new(config);
    /// ```
    pub fn new(config: CircuitBreakerConfig) -> Self {
        debug!(
            failure_threshold = config.failure_threshold,
            success_threshold = config.success_threshold,
            timeout_ms = config.timeout.as_millis(),
            half_open_timeout_ms = config.half_open_timeout.as_millis(),
            "Creating circuit breaker"
        );

        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState::default())),
        }
    }

    /// Execute a closure through the circuit breaker
    ///
    /// This method wraps the provided operation with circuit breaker logic.
    /// The operation will be executed if the circuit is closed or half-open,
    /// and will be rejected immediately if the circuit is open.
    ///
    /// # Type Parameters
    ///
    /// * `F` - Future factory function
    /// * `Fut` - Future returned by the factory
    /// * `T` - Success type
    /// * `E` - Error type (must implement Display)
    ///
    /// # Arguments
    ///
    /// * `f` - Factory function that returns a future
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - Operation succeeded
    /// * `Err(CircuitBreakerError)` - Circuit open, timeout, or operation failed
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        // Check if we should attempt the call
        let should_attempt = {
            let mut state = self.state.write().await;
            state.total_operations += 1;

            match state.state {
                CircuitState::Closed => true,
                CircuitState::Open => {
                    // Check if we should transition to half-open
                    if let Some(last_failure) = state.last_failure_time {
                        if last_failure.elapsed() >= self.config.half_open_timeout {
                            self.transition_to_half_open(&mut state).await;
                            true
                        } else {
                            state.rejected_operations += 1;
                            false
                        }
                    } else {
                        state.rejected_operations += 1;
                        false
                    }
                }
                CircuitState::HalfOpen => true,
            }
        };

        if !should_attempt {
            debug!("Circuit breaker is open - rejecting call");
            return Err(CircuitBreakerError::CircuitOpen);
        }

        // Attempt the call with timeout
        let start_time = Instant::now();
        let result = tokio::time::timeout(self.config.timeout, f()).await;

        let duration = start_time.elapsed();

        // Update state based on result
        match result {
            Ok(Ok(value)) => {
                self.on_success(duration).await;
                Ok(value)
            }
            Ok(Err(e)) => {
                let error_msg = e.to_string();
                self.on_failure(duration).await;
                error!(
                    error = %error_msg,
                    duration_ms = %duration.as_millis(),
                    "Circuit breaker: operation failed"
                );
                Err(CircuitBreakerError::InnerError(error_msg))
            }
            Err(_) => {
                self.on_timeout(duration).await;
                warn!(
                    timeout_ms = %self.config.timeout.as_millis(),
                    actual_duration_ms = %duration.as_millis(),
                    "Circuit breaker: operation timed out"
                );
                Err(CircuitBreakerError::Timeout)
            }
        }
    }

    /// Handle successful operation
    async fn on_success(&self, duration: Duration) {
        let mut state = self.state.write().await;
        state.successful_operations += 1;

        debug!(
            duration_ms = %duration.as_millis(),
            state = %state.state,
            "Circuit breaker: operation succeeded"
        );

        match state.state {
            CircuitState::Closed => {
                // Reset failure count on success
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    self.transition_to_closed(&mut state).await;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                warn!("Circuit breaker: success in open state - this shouldn't happen");
            }
        }
    }

    /// Handle failed operation
    async fn on_failure(&self, duration: Duration) {
        let mut state = self.state.write().await;
        state.failed_operations += 1;
        state.last_failure_time = Some(Instant::now());

        debug!(
            duration_ms = %duration.as_millis(),
            state = %state.state,
            failure_count = %state.failure_count,
            "Circuit breaker: operation failed"
        );

        match state.state {
            CircuitState::Closed => {
                state.failure_count += 1;
                if state.failure_count >= self.config.failure_threshold {
                    self.transition_to_open(&mut state).await;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open immediately opens the circuit
                self.transition_to_open(&mut state).await;
            }
            CircuitState::Open => {
                // Already open, just update stats
            }
        }
    }

    /// Handle timeout
    async fn on_timeout(&self, duration: Duration) {
        // Treat timeouts as failures
        self.on_failure(duration).await;
    }

    /// Transition to closed state
    async fn transition_to_closed(&self, state: &mut CircuitBreakerState) {
        if state.state != CircuitState::Closed {
            info!(
                previous_state = %state.state,
                success_count = %state.success_count,
                "Circuit breaker: transitioning to CLOSED"
            );

            state.state = CircuitState::Closed;
            state.failure_count = 0;
            state.success_count = 0;
            state.last_state_change = Instant::now();
        }
    }

    /// Transition to open state
    async fn transition_to_open(&self, state: &mut CircuitBreakerState) {
        if state.state != CircuitState::Open {
            warn!(
                previous_state = %state.state,
                failure_count = %state.failure_count,
                failure_threshold = %self.config.failure_threshold,
                "Circuit breaker: transitioning to OPEN"
            );

            state.state = CircuitState::Open;
            state.success_count = 0;
            state.last_state_change = Instant::now();
            state.last_failure_time = Some(Instant::now());
        }
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self, state: &mut CircuitBreakerState) {
        if state.state != CircuitState::HalfOpen {
            info!(
                previous_state = %state.state,
                time_since_failure = ?state.last_failure_time.map(|t| t.elapsed()),
                "Circuit breaker: transitioning to HALF-OPEN"
            );

            state.state = CircuitState::HalfOpen;
            state.success_count = 0;
            state.last_state_change = Instant::now();
        }
    }

    /// Get current circuit breaker state
    pub async fn state(&self) -> CircuitState {
        self.state.read().await.state
    }

    /// Check if circuit is open
    pub async fn is_open(&self) -> bool {
        self.state.read().await.state == CircuitState::Open
    }

    /// Check if circuit is closed
    pub async fn is_closed(&self) -> bool {
        self.state.read().await.state == CircuitState::Closed
    }

    /// Check if circuit is half-open
    pub async fn is_half_open(&self) -> bool {
        self.state.read().await.state == CircuitState::HalfOpen
    }

    /// Get comprehensive statistics
    pub async fn stats(&self) -> CircuitBreakerStats {
        let state = self.state.read().await;
        
        let success_rate = if state.total_operations > 0 {
            (state.successful_operations as f64 / state.total_operations as f64) * 100.0
        } else {
            0.0
        };

        let time_until_half_open = if state.state == CircuitState::Open {
            state.last_failure_time.map(|last_failure| {
                let elapsed = last_failure.elapsed();
                if elapsed < self.config.half_open_timeout {
                    self.config.half_open_timeout - elapsed
                } else {
                    Duration::from_secs(0)
                }
            })
        } else {
            None
        };

        CircuitBreakerStats {
            state: state.state,
            total_operations: state.total_operations,
            successful_operations: state.successful_operations,
            failed_operations: state.failed_operations,
            rejected_operations: state.rejected_operations,
            success_rate,
            current_failure_count: state.failure_count,
            current_success_count: state.success_count,
            time_since_last_state_change: state.last_state_change.elapsed(),
            time_until_half_open,
        }
    }

    /// Reset circuit breaker to initial state
    ///
    /// This method resets all counters and transitions to closed state.
    /// Use with caution - typically only for testing or administrative purposes.
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        
        info!(
            previous_state = %state.state,
            total_operations = %state.total_operations,
            "Circuit breaker: manual reset"
        );

        *state = CircuitBreakerState::default();
    }

    /// Force circuit to open state
    ///
    /// This method immediately opens the circuit, rejecting all calls.
    /// Use for emergency situations or administrative control.
    pub async fn force_open(&self) {
        let mut state = self.state.write().await;
        
        warn!(
            previous_state = %state.state,
            "Circuit breaker: forced to OPEN state"
        );

        state.state = CircuitState::Open;
        state.last_failure_time = Some(Instant::now());
        state.last_state_change = Instant::now();
    }

    /// Force circuit to closed state
    ///
    /// This method immediately closes the circuit, allowing all calls.
    /// Use with caution - bypasses normal failure detection.
    pub async fn force_closed(&self) {
        let mut state = self.state.write().await;
        
        warn!(
            previous_state = %state.state,
            "Circuit breaker: forced to CLOSED state"
        );

        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_state_change = Instant::now();
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

/// Create circuit breaker from environment variables
///
/// This function creates a circuit breaker using configuration from
/// environment variables. This is useful for production deployments.
///
/// # Environment Variables
///
/// * `CIRCUIT_BREAKER_FAILURE_THRESHOLD` - Failure threshold (default: 5)
/// * `CIRCUIT_BREAKER_SUCCESS_THRESHOLD` - Success threshold (default: 2)
/// * `CIRCUIT_BREAKER_TIMEOUT` - Operation timeout in seconds (default: 1)
/// * `CIRCUIT_BREAKER_HALF_OPEN_TIMEOUT` - Half-open timeout in seconds (default: 30)
pub fn create_from_env() -> CircuitBreaker {
    let failure_threshold = std::env::var("CIRCUIT_BREAKER_FAILURE_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(5);

    let success_threshold = std::env::var("CIRCUIT_BREAKER_SUCCESS_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(2);

    let timeout = std::env::var("CIRCUIT_BREAKER_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(1));

    let half_open_timeout = std::env::var("CIRCUIT_BREAKER_HALF_OPEN_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));

    let config = CircuitBreakerConfig {
        failure_threshold,
        success_threshold,
        timeout,
        half_open_timeout,
    };

    CircuitBreaker::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_circuit_breaker_starts_closed() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert!(!breaker.is_open().await);
        assert!(breaker.is_closed().await);
    }

    #[tokio::test]
    async fn test_successful_operation() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_failed_operation() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        let result = breaker.call(|| async { Err::<String, _>("error") }).await;
        
        assert!(result.is_err());
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(100),
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // First failure
        let result = breaker.call(|| async { Err::<String, _>("error") }).await;
        assert!(result.is_err());
        assert_eq!(breaker.state().await, CircuitState::Closed);
        
        // Second failure should open the circuit
        let result = breaker.call(|| async { Err::<String, _>("error") }).await;
        assert!(result.is_err());
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Third call should be rejected immediately
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = breaker.call(|| {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>("should not execute")
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 0); // Should not have been called
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            timeout: Duration::from_millis(50),
            half_open_timeout: Duration::from_millis(100),
        };
        
        let breaker = CircuitBreaker::new(config);
        
        let start = Instant::now();
        let result = breaker.call(|| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, String>("should timeout")
        }).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CircuitBreakerError::Timeout));
        assert!(start.elapsed() < Duration::from_millis(80)); // Should timeout quickly
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(50),
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // Open the circuit
        let result = breaker.call(|| async { Err::<String, _>("error") }).await;
        assert!(result.is_err());
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Wait for half-open timeout
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Next call should transition to half-open
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        assert!(result.is_ok());
        assert_eq!(breaker.state().await, CircuitState::Closed); // Should close after one success
    }

    #[tokio::test]
    async fn test_statistics() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        // Make some calls
        let _result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        let _result = breaker.call(|| async { Err::<String, _>("error") }).await;
        
        let stats = breaker.stats().await;
        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.failed_operations, 1);
        assert_eq!(stats.success_rate, 50.0);
    }

    #[tokio::test]
    async fn test_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        
        // Open the circuit
        let _result = breaker.call(|| async { Err::<String, _>("error") }).await;
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Reset
        breaker.reset().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
        
        let stats = breaker.stats().await;
        assert_eq!(stats.total_operations, 0);
    }

    #[tokio::test]
    async fn test_force_open() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(breaker.state().await, CircuitState::Closed);
        
        breaker.force_open().await;
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Calls should be rejected
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_force_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        
        // Open the circuit
        let _result = breaker.call(|| async { Err::<String, _>("error") }).await;
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Force closed
        breaker.force_closed().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
        
        // Calls should work again
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        assert!(result.is_ok());
    }
}
//! Retry logic with exponential backoff for SpiceDB operations
//!
//! This module provides resilient retry mechanisms for SpiceDB operations
//! that may fail due to transient network issues, temporary service
//! unavailability, or rate limiting.
//!
//! # Retry Strategies
//!
//! The module implements several retry strategies:
//!
//! 1. **Exponential Backoff**: Delays increase exponentially with each retry
//! 2. **Jittered Backoff**: Adds randomness to prevent thundering herd
//! 3. **Fixed Backoff**: Uses constant delay between retries
//! 4. **Linear Backoff**: Delays increase linearly with each retry
//!
//! # Error Classification
//!
//! Errors are classified into retryable and non-retryable categories:
//!
//! - **Retryable**: Connection errors, timeouts, rate limits, temporary failures
//! - **Non-retryable**: Authentication errors, permission denied, invalid requests
//!
//! # Usage
//!
//! ```rust
//! use crate::services::spicedb::retry::{RetryConfig, RetryExecutor};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = RetryConfig::default()
//!     .with_max_attempts(3)
//!     .with_initial_delay(Duration::from_millis(100))
//!     .with_max_delay(Duration::from_secs(5));
//!
//! let executor = RetryExecutor::new(config);
//!
//! let result = executor.execute(|| async {
//!     // Your SpiceDB operation here
//!     spicedb_client.check_permission(request).await
//! }).await;
//! # Ok(())
//! # }
//! ```

use std::time::Duration;
use tokio::time::{sleep, Instant};
use tracing::{debug, warn};
use rand::Rng;
use futures::Future;
use std::pin::Pin;

use super::{SpiceDBError, CheckPermissionRequest};

/// Retry configuration for SpiceDB operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (not including initial attempt)
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff strategy to use
    pub strategy: BackoffStrategy,
    /// Jitter configuration for randomizing delays
    pub jitter: JitterConfig,
    /// Whether to enable retry logging
    pub enable_logging: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            strategy: BackoffStrategy::ExponentialWithJitter,
            jitter: JitterConfig::default(),
            enable_logging: true,
        }
    }
}

impl RetryConfig {
    /// Create new retry configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of retry attempts
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Set initial delay before first retry
    pub fn with_initial_delay(mut self, initial_delay: Duration) -> Self {
        self.initial_delay = initial_delay;
        self
    }

    /// Set maximum delay between retries
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Set backoff strategy
    pub fn with_strategy(mut self, strategy: BackoffStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set jitter configuration
    pub fn with_jitter(mut self, jitter: JitterConfig) -> Self {
        self.jitter = jitter;
        self
    }

    /// Enable or disable retry logging
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }
}

/// Backoff strategies for retry delays
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Linear increase in delay (delay = initial_delay * attempt_number)
    Linear,
    /// Exponential increase (delay = initial_delay * 2^attempt_number)
    Exponential,
    /// Exponential with jitter to prevent thundering herd
    ExponentialWithJitter,
}

/// Jitter configuration for randomizing retry delays
#[derive(Debug, Clone)]
pub struct JitterConfig {
    /// Type of jitter to apply
    pub jitter_type: JitterType,
    /// Maximum jitter as percentage of base delay (0.0 to 1.0)
    pub max_jitter_ratio: f64,
}

impl Default for JitterConfig {
    fn default() -> Self {
        Self {
            jitter_type: JitterType::Uniform,
            max_jitter_ratio: 0.3, // 30% jitter
        }
    }
}

/// Types of jitter that can be applied
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JitterType {
    /// No jitter applied
    None,
    /// Uniform random jitter: delay ± (jitter_ratio * delay)
    Uniform,
    /// Full jitter: delay * random(0, 1)
    Full,
}

/// Retry executor that handles the retry logic
#[derive(Debug, Clone)]
pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    /// Create new retry executor with given configuration
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Create retry executor with default configuration
    pub fn default() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Execute operation with retry logic
    ///
    /// This method executes the provided operation and retries it according
    /// to the configured retry policy if it fails with a retryable error.
    ///
    /// # Type Parameters
    ///
    /// * `F` - Future factory function
    /// * `Fut` - Future returned by the factory
    /// * `T` - Success return type
    /// * `E` - Error type
    ///
    /// # Arguments
    ///
    /// * `operation` - Factory function that creates the operation future
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - Operation succeeded
    /// * `Err(E)` - Operation failed after all retries
    pub async fn execute<F, Fut, T, E>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: RetryableError + std::fmt::Display + Clone,
    {
        let start_time = Instant::now();
        let mut attempt = 0;
        let mut _last_error: Option<E> = None;

        loop {
            attempt += 1;

            if self.config.enable_logging {
                debug!(
                    attempt = %attempt,
                    max_attempts = %(self.config.max_attempts + 1),
                    "Executing operation (attempt {})", attempt
                );
            }

            // Execute the operation
            match operation().await {
                Ok(result) => {
                    if self.config.enable_logging && attempt > 1 {
                        debug!(
                            attempt = %attempt,
                            total_duration_ms = %start_time.elapsed().as_millis(),
                            "Operation succeeded after {} attempts", attempt
                        );
                    }
                    return Ok(result);
                }
                Err(error) => {
                    _last_error = Some(error.clone());

                    // Check if error is retryable
                    if !error.is_retryable() {
                        if self.config.enable_logging {
                            debug!(
                                attempt = %attempt,
                                error = %error,
                                "Operation failed with non-retryable error"
                            );
                        }
                        return Err(error);
                    }

                    // Check if we've exhausted all retry attempts
                    if attempt > self.config.max_attempts {
                        if self.config.enable_logging {
                            warn!(
                                attempts = %attempt,
                                total_duration_ms = %start_time.elapsed().as_millis(),
                                error = %error,
                                "Operation failed after all retry attempts"
                            );
                        }
                        return Err(error);
                    }

                    // Calculate delay for next retry
                    let delay = self.calculate_delay(attempt - 1); // 0-indexed for calculation

                    if self.config.enable_logging {
                        debug!(
                            attempt = %attempt,
                            delay_ms = %delay.as_millis(),
                            error = %error,
                            "Operation failed, retrying after delay"
                        );
                    }

                    // Wait before retrying
                    sleep(delay).await;
                }
            }
        }
    }

    /// Execute operation with retry logic and detailed context
    ///
    /// This method provides more detailed context about the retry process,
    /// including attempt counts and timing information.
    pub async fn execute_with_context<F, Fut, T, E>(
        &self,
        mut operation: F,
    ) -> Result<RetryResult<T>, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: RetryableError + std::fmt::Display + Clone,
    {
        let start_time = Instant::now();
        let mut attempt = 0;
        let mut retry_delays = Vec::new();

        loop {
            attempt += 1;
            let attempt_start = Instant::now();

            match operation().await {
                Ok(result) => {
                    return Ok(RetryResult {
                        result,
                        attempts: attempt,
                        total_duration: start_time.elapsed(),
                        retry_delays,
                        succeeded: true,
                    });
                }
                Err(error) => {
                    let attempt_duration = attempt_start.elapsed();

                    if !error.is_retryable() || attempt > self.config.max_attempts {
                        return Err(error);
                    }

                    let delay = self.calculate_delay(attempt - 1);
                    retry_delays.push(RetryAttempt {
                        attempt_number: attempt,
                        duration: attempt_duration,
                        delay_before_next: delay,
                    });

                    sleep(delay).await;
                }
            }
        }
    }

    /// Calculate delay for a given retry attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = match self.config.strategy {
            BackoffStrategy::Fixed => self.config.initial_delay,
            BackoffStrategy::Linear => {
                Duration::from_millis(
                    self.config.initial_delay.as_millis() as u64 * (attempt + 1) as u64
                )
            }
            BackoffStrategy::Exponential | BackoffStrategy::ExponentialWithJitter => {
                let multiplier = 2_u64.pow(attempt);
                Duration::from_millis(
                    (self.config.initial_delay.as_millis() as u64).saturating_mul(multiplier)
                )
            }
        };

        // Cap at maximum delay
        let capped_delay = std::cmp::min(base_delay, self.config.max_delay);

        // Apply jitter if configured
        match self.config.strategy {
            BackoffStrategy::ExponentialWithJitter => {
                self.apply_jitter(capped_delay)
            }
            _ => capped_delay,
        }
    }

    /// Apply jitter to delay according to jitter configuration
    fn apply_jitter(&self, base_delay: Duration) -> Duration {
        match self.config.jitter.jitter_type {
            JitterType::None => base_delay,
            JitterType::Uniform => {
                let mut rng = rand::thread_rng();
                let jitter_amount = (base_delay.as_millis() as f64 * self.config.jitter.max_jitter_ratio) as u64;
                let jitter = rng.gen_range(0..=jitter_amount * 2).saturating_sub(jitter_amount);
                
                Duration::from_millis(
                    (base_delay.as_millis() as u64).saturating_add(jitter)
                )
            }
            JitterType::Full => {
                let mut rng = rand::thread_rng();
                let random_factor: f64 = rng.r#gen();
                Duration::from_millis((base_delay.as_millis() as f64 * random_factor) as u64)
            }
        }
    }
}

/// Result of a retry operation with detailed context
#[derive(Debug, Clone)]
pub struct RetryResult<T> {
    /// The successful result
    pub result: T,
    /// Total number of attempts made
    pub attempts: u32,
    /// Total time spent including retries
    pub total_duration: Duration,
    /// Details of each retry attempt
    pub retry_delays: Vec<RetryAttempt>,
    /// Whether the operation ultimately succeeded
    pub succeeded: bool,
}

/// Information about a single retry attempt
#[derive(Debug, Clone)]
pub struct RetryAttempt {
    /// Attempt number (1-based)
    pub attempt_number: u32,
    /// Duration of this attempt
    pub duration: Duration,
    /// Delay before the next attempt
    pub delay_before_next: Duration,
}

/// Trait for errors that can be retried
pub trait RetryableError {
    /// Returns true if this error is retryable
    fn is_retryable(&self) -> bool;
    
    /// Returns the recommended delay before retry (optional)
    fn retry_after(&self) -> Option<Duration> {
        None
    }
}

impl RetryableError for SpiceDBError {
    fn is_retryable(&self) -> bool {
        match self {
            // These errors are likely transient and worth retrying
            SpiceDBError::ConnectionError(_) => true,
            SpiceDBError::Timeout => true,
            SpiceDBError::InternalError(_) => true,
            // These errors are not transient
            SpiceDBError::PermissionDenied(_) => false,
            SpiceDBError::AuthenticationFailed(_) => false,
            SpiceDBError::InvalidRequest(_) => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            // For timeouts, wait a bit longer before retrying
            SpiceDBError::Timeout => Some(Duration::from_millis(500)),
            // For connection errors, retry quickly
            SpiceDBError::ConnectionError(_) => Some(Duration::from_millis(100)),
            _ => None,
        }
    }
}

/// Convenience function to retry SpiceDB permission checks
pub async fn retry_permission_check<C>(
    client: &C,
    request: CheckPermissionRequest,
    config: RetryConfig,
) -> Result<bool, SpiceDBError>
where
    C: super::SpiceDBClientTrait,
{
    let executor = RetryExecutor::new(config);
    
    executor.execute(|| {
        let req = request.clone();
        Box::pin(async move {
            client.check_permission(req).await
        }) as Pin<Box<dyn Future<Output = Result<bool, SpiceDBError>> + Send>>
    }).await
}

/// Create retry configuration from environment variables
///
/// This function creates retry configuration using environment variables.
///
/// # Environment Variables
///
/// * `SPICEDB_RETRY_MAX_ATTEMPTS` - Maximum retry attempts (default: 3)
/// * `SPICEDB_RETRY_INITIAL_DELAY_MS` - Initial delay in milliseconds (default: 100)
/// * `SPICEDB_RETRY_MAX_DELAY_MS` - Maximum delay in milliseconds (default: 5000)
/// * `SPICEDB_RETRY_STRATEGY` - Backoff strategy: fixed, linear, exponential, exponential_jitter (default: exponential_jitter)
/// * `SPICEDB_RETRY_JITTER_RATIO` - Jitter ratio (0.0 to 1.0, default: 0.3)
/// * `SPICEDB_RETRY_LOGGING` - Enable retry logging (default: true)
pub fn create_retry_config_from_env() -> RetryConfig {
    let max_attempts = std::env::var("SPICEDB_RETRY_MAX_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(3);

    let initial_delay = std::env::var("SPICEDB_RETRY_INITIAL_DELAY_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(100));

    let max_delay = std::env::var("SPICEDB_RETRY_MAX_DELAY_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(5));

    let strategy = std::env::var("SPICEDB_RETRY_STRATEGY")
        .ok()
        .and_then(|s| match s.to_lowercase().as_str() {
            "fixed" => Some(BackoffStrategy::Fixed),
            "linear" => Some(BackoffStrategy::Linear),
            "exponential" => Some(BackoffStrategy::Exponential),
            "exponential_jitter" => Some(BackoffStrategy::ExponentialWithJitter),
            _ => None,
        })
        .unwrap_or(BackoffStrategy::ExponentialWithJitter);

    let jitter_ratio = std::env::var("SPICEDB_RETRY_JITTER_RATIO")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&r| r >= 0.0 && r <= 1.0)
        .unwrap_or(0.3);

    let enable_logging = std::env::var("SPICEDB_RETRY_LOGGING")
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);

    RetryConfig::new()
        .with_max_attempts(max_attempts)
        .with_initial_delay(initial_delay)
        .with_max_delay(max_delay)
        .with_strategy(strategy)
        .with_jitter(JitterConfig {
            jitter_type: if matches!(strategy, BackoffStrategy::ExponentialWithJitter) {
                JitterType::Uniform
            } else {
                JitterType::None
            },
            max_jitter_ratio: jitter_ratio,
        })
        .with_logging(enable_logging)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct TestError {
        retryable: bool,
        message: String,
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl std::error::Error for TestError {}

    impl RetryableError for TestError {
        fn is_retryable(&self) -> bool {
            self.retryable
        }
    }

    #[tokio::test]
    async fn test_retry_config_creation() {
        let config = RetryConfig::new()
            .with_max_attempts(5)
            .with_initial_delay(Duration::from_millis(200))
            .with_max_delay(Duration::from_secs(10))
            .with_strategy(BackoffStrategy::Linear)
            .with_logging(false);

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(200));
        assert_eq!(config.max_delay, Duration::from_secs(10));
        assert_eq!(config.strategy, BackoffStrategy::Linear);
        assert!(!config.enable_logging);
    }

    #[tokio::test]
    async fn test_successful_operation_no_retry() {
        let config = RetryConfig::default();
        let executor = RetryExecutor::new(config);
        let call_count = Arc::new(AtomicU32::new(0));

        let result = executor.execute(|| {
            let count = call_count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, TestError>(42)
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_on_retryable_error() {
        let config = RetryConfig::default().with_max_attempts(2);
        let executor = RetryExecutor::new(config);
        let call_count = Arc::new(AtomicU32::new(0));

        let result = executor.execute(|| {
            let count = call_count.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current == 0 {
                    Err(TestError {
                        retryable: true,
                        message: "Transient error".to_string(),
                    })
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_no_retry_on_non_retryable_error() {
        let config = RetryConfig::default().with_max_attempts(3);
        let executor = RetryExecutor::new(config);
        let call_count = Arc::new(AtomicU32::new(0));

        let result = executor.execute(|| {
            let count = call_count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, TestError>(TestError {
                    retryable: false,
                    message: "Non-retryable error".to_string(),
                })
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_exhaust_all_retries() {
        let config = RetryConfig::default()
            .with_max_attempts(2)
            .with_initial_delay(Duration::from_millis(1)); // Fast for testing
        let executor = RetryExecutor::new(config);
        let call_count = Arc::new(AtomicU32::new(0));

        let result = executor.execute(|| {
            let count = call_count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, TestError>(TestError {
                    retryable: true,
                    message: "Always fails".to_string(),
                })
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }

    #[tokio::test]
    async fn test_execute_with_context() {
        let config = RetryConfig::default()
            .with_max_attempts(2)
            .with_initial_delay(Duration::from_millis(1));
        let executor = RetryExecutor::new(config);
        let call_count = Arc::new(AtomicU32::new(0));

        let result = executor.execute_with_context(|| {
            let count = call_count.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current == 0 {
                    Err(TestError {
                        retryable: true,
                        message: "First failure".to_string(),
                    })
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert!(result.is_ok());
        let retry_result = result.unwrap();
        assert_eq!(retry_result.result, 42);
        assert_eq!(retry_result.attempts, 2);
        assert!(retry_result.succeeded);
        assert_eq!(retry_result.retry_delays.len(), 1);
    }

    #[tokio::test]
    async fn test_delay_calculation_fixed() {
        let config = RetryConfig::default()
            .with_strategy(BackoffStrategy::Fixed)
            .with_initial_delay(Duration::from_millis(100))
            .with_jitter(JitterConfig {
                jitter_type: JitterType::None,
                max_jitter_ratio: 0.0,
            });
        let executor = RetryExecutor::new(config);

        assert_eq!(executor.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(executor.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(executor.calculate_delay(5), Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_delay_calculation_linear() {
        let config = RetryConfig::default()
            .with_strategy(BackoffStrategy::Linear)
            .with_initial_delay(Duration::from_millis(100))
            .with_jitter(JitterConfig {
                jitter_type: JitterType::None,
                max_jitter_ratio: 0.0,
            });
        let executor = RetryExecutor::new(config);

        assert_eq!(executor.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(executor.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(executor.calculate_delay(2), Duration::from_millis(300));
    }

    #[tokio::test]
    async fn test_delay_calculation_exponential() {
        let config = RetryConfig::default()
            .with_strategy(BackoffStrategy::Exponential)
            .with_initial_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_secs(10))
            .with_jitter(JitterConfig {
                jitter_type: JitterType::None,
                max_jitter_ratio: 0.0,
            });
        let executor = RetryExecutor::new(config);

        assert_eq!(executor.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(executor.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(executor.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(executor.calculate_delay(3), Duration::from_millis(800));
    }

    #[tokio::test]
    async fn test_max_delay_cap() {
        let config = RetryConfig::default()
            .with_strategy(BackoffStrategy::Exponential)
            .with_initial_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_millis(500))
            .with_jitter(JitterConfig {
                jitter_type: JitterType::None,
                max_jitter_ratio: 0.0,
            });
        let executor = RetryExecutor::new(config);

        assert_eq!(executor.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(executor.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(executor.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(executor.calculate_delay(3), Duration::from_millis(500)); // Capped
        assert_eq!(executor.calculate_delay(10), Duration::from_millis(500)); // Still capped
    }

    #[tokio::test]
    async fn test_jitter_application() {
        let config = RetryConfig::default()
            .with_strategy(BackoffStrategy::ExponentialWithJitter)
            .with_initial_delay(Duration::from_millis(100))
            .with_jitter(JitterConfig {
                jitter_type: JitterType::Uniform,
                max_jitter_ratio: 0.5,
            });
        let executor = RetryExecutor::new(config);

        // With jitter, delays should vary
        let delay1 = executor.calculate_delay(1);
        let delay2 = executor.calculate_delay(1);
        
        // They might be the same due to randomness, but structure should be correct
        assert!(delay1.as_millis() > 0);
        assert!(delay2.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_spicedb_error_retryability() {
        // Retryable errors
        assert!(SpiceDBError::ConnectionError("network error".to_string()).is_retryable());
        assert!(SpiceDBError::Timeout.is_retryable());
        assert!(SpiceDBError::InternalError("server error".to_string()).is_retryable());

        // Non-retryable errors
        assert!(!SpiceDBError::PermissionDenied("access denied".to_string()).is_retryable());
        assert!(!SpiceDBError::AuthenticationFailed("bad token".to_string()).is_retryable());
        assert!(!SpiceDBError::InvalidRequest("bad request".to_string()).is_retryable());
    }

    #[tokio::test]
    async fn test_environment_configuration() {
        unsafe {
            std::env::set_var("SPICEDB_RETRY_MAX_ATTEMPTS", "5");
            std::env::set_var("SPICEDB_RETRY_INITIAL_DELAY_MS", "200");
            std::env::set_var("SPICEDB_RETRY_MAX_DELAY_MS", "10000");
            std::env::set_var("SPICEDB_RETRY_STRATEGY", "linear");
            std::env::set_var("SPICEDB_RETRY_JITTER_RATIO", "0.5");
            std::env::set_var("SPICEDB_RETRY_LOGGING", "false");
        }

        let config = create_retry_config_from_env();

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(200));
        assert_eq!(config.max_delay, Duration::from_millis(10000));
        assert_eq!(config.strategy, BackoffStrategy::Linear);
        assert_eq!(config.jitter.max_jitter_ratio, 0.5);
        assert!(!config.enable_logging);

        // Cleanup
        unsafe {
            std::env::remove_var("SPICEDB_RETRY_MAX_ATTEMPTS");
            std::env::remove_var("SPICEDB_RETRY_INITIAL_DELAY_MS");
            std::env::remove_var("SPICEDB_RETRY_MAX_DELAY_MS");
            std::env::remove_var("SPICEDB_RETRY_STRATEGY");
            std::env::remove_var("SPICEDB_RETRY_JITTER_RATIO");
            std::env::remove_var("SPICEDB_RETRY_LOGGING");
        }
    }
}
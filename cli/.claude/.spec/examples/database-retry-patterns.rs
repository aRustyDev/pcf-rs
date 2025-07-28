/// Database Retry Patterns - Example Implementation
/// 
/// This file demonstrates the retry patterns required for Phase 2 database implementation.
/// Following the specification requirements for infinite retry with exponential backoff.

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

/// Exponential backoff implementation with jitter
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

    /// Calculate next delay with exponential backoff
    /// Sequence: 1s, 2s, 4s, 8s, 16s, 32s, 60s (max)
    pub fn next_delay(&mut self) -> Duration {
        // Calculate base exponential delay
        let exp_delay = self.base_delay * 2u32.pow(self.attempt.min(6));
        let delay = exp_delay.min(self.max_delay);
        
        self.attempt += 1;
        
        // Add jitter to prevent thundering herd
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

/// Retry a database operation with exponential backoff
/// This will retry forever - suitable for startup connections
pub async fn retry_forever<F, T, E>(
    mut operation: F,
    operation_name: &str,
) -> T
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    let mut backoff = ExponentialBackoff::new();
    
    loop {
        match operation().await {
            Ok(result) => {
                info!("{} succeeded after {} attempts", operation_name, backoff.attempt);
                return result;
            }
            Err(err) => {
                let delay = backoff.next_delay();
                warn!(
                    "{} failed (attempt {}): {}. Retrying in {:?}",
                    operation_name, backoff.attempt, err, delay
                );
                sleep(delay).await;
            }
        }
    }
}

/// Retry with a timeout - suitable for runtime operations
pub async fn retry_with_timeout<F, T, E>(
    mut operation: F,
    operation_name: &str,
    timeout: Duration,
) -> Result<T, E>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display + From<String>,
{
    let mut backoff = ExponentialBackoff::new();
    let start = tokio::time::Instant::now();
    
    loop {
        if start.elapsed() > timeout {
            return Err(E::from(format!("{} timed out after {:?}", operation_name, timeout)));
        }
        
        match operation().await {
            Ok(result) => {
                info!("{} succeeded", operation_name);
                return Ok(result);
            }
            Err(err) => {
                let delay = backoff.next_delay();
                let remaining = timeout.saturating_sub(start.elapsed());
                
                if delay > remaining {
                    return Err(err);
                }
                
                warn!(
                    "{} failed: {}. Retrying in {:?} (timeout in {:?})",
                    operation_name, err, delay, remaining
                );
                sleep(delay).await;
            }
        }
    }
}

/// Circuit breaker pattern for external services
pub struct CircuitBreaker {
    failure_count: u32,
    failure_threshold: u32,
    success_count: u32,
    success_threshold: u32,
    state: CircuitState,
    last_failure: Option<tokio::time::Instant>,
    half_open_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed,     // Normal operation
    Open,       // Failing, reject requests
    HalfOpen,   // Testing if service recovered
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            failure_count: 0,
            failure_threshold: 5,
            success_count: 0,
            success_threshold: 3,
            state: CircuitState::Closed,
            last_failure: None,
            half_open_timeout: Duration::from_secs(30),
        }
    }

    pub async fn call<F, T, E>(&mut self, operation: F) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
        E: From<String>,
    {
        // Check if we should transition from Open to HalfOpen
        if self.state == CircuitState::Open {
            if let Some(last_failure) = self.last_failure {
                if last_failure.elapsed() > self.half_open_timeout {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                }
            }
        }

        match self.state {
            CircuitState::Open => {
                Err(E::from("Circuit breaker is open".to_string()))
            }
            CircuitState::Closed | CircuitState::HalfOpen => {
                match operation.await {
                    Ok(result) => {
                        self.on_success();
                        Ok(result)
                    }
                    Err(err) => {
                        self.on_failure();
                        Err(err)
                    }
                }
            }
        }
    }

    fn on_success(&mut self) {
        self.failure_count = 0;
        
        if self.state == CircuitState::HalfOpen {
            self.success_count += 1;
            if self.success_count >= self.success_threshold {
                self.state = CircuitState::Closed;
                info!("Circuit breaker closed after {} successes", self.success_count);
            }
        }
    }

    fn on_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(tokio::time::Instant::now());
        
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
            error!("Circuit breaker opened after {} failures", self.failure_count);
        }
        
        if self.state == CircuitState::HalfOpen {
            self.state = CircuitState::Open;
            self.success_count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_exponential_backoff_sequence() {
        let mut backoff = ExponentialBackoff {
            attempt: 0,
            max_delay: Duration::from_secs(60),
            base_delay: Duration::from_secs(1),
            jitter: false, // Disable jitter for predictable testing
        };

        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));
        assert_eq!(backoff.next_delay(), Duration::from_secs(4));
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));
        assert_eq!(backoff.next_delay(), Duration::from_secs(16));
        assert_eq!(backoff.next_delay(), Duration::from_secs(32));
        assert_eq!(backoff.next_delay(), Duration::from_secs(60)); // Max
        assert_eq!(backoff.next_delay(), Duration::from_secs(60)); // Still max
    }

    #[tokio::test]
    async fn test_retry_forever_succeeds_eventually() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();
        
        let operation = || {
            let attempts = attempts_clone.clone();
            Box::pin(async move {
                let count = attempts.fetch_add(1, Ordering::SeqCst);
                if count < 3 {
                    Err("Connection failed")
                } else {
                    Ok("Connected!")
                }
            })
        };

        let result = retry_forever(operation, "test connection").await;
        assert_eq!(result, "Connected!");
        assert_eq!(attempts.load(Ordering::SeqCst), 4); // 3 failures + 1 success
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let mut breaker = CircuitBreaker::new();
        breaker.failure_threshold = 3;

        // Simulate failures
        for _ in 0..3 {
            let _ = breaker.call(async { 
                Err::<(), String>("Service unavailable".to_string()) 
            }).await;
        }

        assert_eq!(breaker.state, CircuitState::Open);

        // Further calls should fail immediately
        let result = breaker.call(async { 
            Ok::<_, String>("This shouldn't execute") 
        }).await;
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Circuit breaker is open");
    }
}

/// Example usage in database connection
pub mod database_example {
    use super::*;
    
    pub struct DatabaseConnection {
        endpoint: String,
        circuit_breaker: CircuitBreaker,
    }
    
    impl DatabaseConnection {
        pub async fn connect(&mut self) -> Result<(), String> {
            // Use circuit breaker for connection attempts
            self.circuit_breaker.call(async {
                self.attempt_connection().await
            }).await
        }
        
        async fn attempt_connection(&self) -> Result<(), String> {
            // Simulate connection attempt
            info!("Attempting to connect to {}", self.endpoint);
            
            // In real implementation:
            // - Create TCP/WebSocket connection
            // - Authenticate
            // - Select database/namespace
            
            Ok(())
        }
        
        /// Connect with infinite retry for startup
        pub async fn connect_on_startup(&mut self) {
            retry_forever(
                || Box::pin(self.attempt_connection()),
                "Database connection"
            ).await
        }
    }
}

/// Write queue retry patterns
pub mod write_queue_example {
    use super::*;
    use std::collections::VecDeque;
    
    pub struct WriteQueue {
        queue: VecDeque<QueuedWrite>,
        max_size: usize,
        retry_delay: Duration,
    }
    
    pub struct QueuedWrite {
        pub id: String,
        pub data: serde_json::Value,
        pub retry_count: u32,
        pub max_retries: u32,
    }
    
    impl WriteQueue {
        pub fn new(max_size: usize) -> Self {
            Self {
                queue: VecDeque::with_capacity(max_size),
                max_size,
                retry_delay: Duration::from_secs(5),
            }
        }
        
        pub fn enqueue(&mut self, write: QueuedWrite) -> Result<(), String> {
            if self.queue.len() >= self.max_size {
                return Err("Queue full".to_string());
            }
            
            self.queue.push_back(write);
            Ok(())
        }
        
        pub async fn process_with_retry<F>(&mut self, mut execute: F) 
        where
            F: FnMut(&QueuedWrite) -> futures::future::BoxFuture<'static, Result<(), String>>,
        {
            let mut processed = Vec::new();
            
            for (idx, write) in self.queue.iter_mut().enumerate() {
                match execute(write).await {
                    Ok(_) => {
                        info!("Successfully processed write {}", write.id);
                        processed.push(idx);
                    }
                    Err(err) => {
                        write.retry_count += 1;
                        warn!(
                            "Failed to process write {} (attempt {}): {}",
                            write.id, write.retry_count, err
                        );
                        
                        if write.retry_count >= write.max_retries {
                            error!("Write {} exceeded max retries, removing from queue", write.id);
                            processed.push(idx);
                        }
                    }
                }
            }
            
            // Remove processed writes in reverse order
            for idx in processed.into_iter().rev() {
                self.queue.remove(idx);
            }
            
            if !self.queue.is_empty() {
                sleep(self.retry_delay).await;
            }
        }
    }
}
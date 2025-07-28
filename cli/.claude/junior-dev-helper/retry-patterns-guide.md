# Retry Patterns Guide for Database Connections

## Why Retry?

Databases aren't always ready when you need them:
- Container starting up (takes 5-30 seconds)
- Network hiccup (brief disconnect)
- Database restarting (maintenance)
- Temporary overload (too many connections)

Instead of failing immediately, we retry with smart delays.

## Exponential Backoff Explained

### The Problem with Fixed Delays

```rust
// BAD: Fixed retry every 1 second
loop {
    if connect().await.is_ok() {
        break;
    }
    sleep(Duration::from_secs(1)).await;
}
```

Why this is bad:
- If 100 services retry every 1 second, database gets 100 requests/second
- "Thundering herd" can crash recovering database
- No gradual reduction in load

### Exponential Backoff Solution

```rust
// GOOD: Exponentially increasing delays
let delays = [1, 2, 4, 8, 16, 32, 60, 60, 60...];
```

Benefits:
- Gives database time to recover
- Reduces load progressively
- Still retries frequently at first

## Basic Implementation

### Simple Exponential Backoff

```rust
pub struct ExponentialBackoff {
    attempt: u32,
    base: Duration,
    max: Duration,
}

impl ExponentialBackoff {
    pub fn new() -> Self {
        Self {
            attempt: 0,
            base: Duration::from_secs(1),
            max: Duration::from_secs(60),
        }
    }
    
    pub fn next_delay(&mut self) -> Duration {
        // Calculate: base * 2^attempt
        let exponential = self.base * 2u32.pow(self.attempt);
        
        // Cap at maximum
        let delay = exponential.min(self.max);
        
        // Increment for next time
        self.attempt += 1;
        
        delay
    }
    
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

// Usage
let mut backoff = ExponentialBackoff::new();

loop {
    match connect().await {
        Ok(conn) => {
            backoff.reset(); // Important!
            return Ok(conn);
        }
        Err(e) => {
            let delay = backoff.next_delay();
            tracing::warn!("Connection failed, retrying in {:?}", delay);
            tokio::time::sleep(delay).await;
        }
    }
}
```

## Adding Jitter

### Why Jitter Matters

Without jitter, all clients retry at exactly the same time:

```
Client 1: Retry at 2.000s
Client 2: Retry at 2.000s  
Client 3: Retry at 2.000s
Database: 💥 (overwhelmed)
```

With jitter, retries are spread out:

```
Client 1: Retry at 2.341s
Client 2: Retry at 2.892s
Client 3: Retry at 2.127s
Database: 😊 (manageable load)
```

### Implementation with Jitter

```rust
use rand::Rng;

pub struct ExponentialBackoff {
    attempt: u32,
    base: Duration,
    max: Duration,
    jitter: bool,
}

impl ExponentialBackoff {
    pub fn with_jitter() -> Self {
        Self {
            attempt: 0,
            base: Duration::from_secs(1),
            max: Duration::from_secs(60),
            jitter: true,
        }
    }
    
    pub fn next_delay(&mut self) -> Duration {
        // Base exponential calculation
        let exponential = self.base * 2u32.pow(self.attempt);
        let mut delay = exponential.min(self.max);
        
        // Add jitter (0-25% additional delay)
        if self.jitter {
            let jitter_ms = rand::thread_rng().gen_range(0..250);
            let jitter_percent = delay.as_millis() as u64 * jitter_ms / 1000;
            delay += Duration::from_millis(jitter_percent);
        }
        
        self.attempt += 1;
        delay
    }
}
```

## Complete Retry Pattern

### Production-Ready Implementation

```rust
use std::future::Future;
use std::time::{Duration, Instant};

pub struct RetryConfig {
    pub max_attempts: Option<u32>,
    pub max_duration: Option<Duration>,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: None,  // Retry forever
            max_duration: Some(Duration::from_secs(600)), // 10 minutes
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter: true,
        }
    }
}

pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let start_time = Instant::now();
    let mut backoff = ExponentialBackoff {
        attempt: 0,
        base: config.base_delay,
        max: config.max_delay,
        jitter: config.jitter,
    };
    
    loop {
        // Check if we've exceeded max attempts
        if let Some(max) = config.max_attempts {
            if backoff.attempt >= max {
                tracing::error!("Max retry attempts ({}) exceeded", max);
                break;
            }
        }
        
        // Check if we've exceeded max duration
        if let Some(max_duration) = config.max_duration {
            if start_time.elapsed() >= max_duration {
                tracing::error!("Max retry duration ({:?}) exceeded", max_duration);
                break;
            }
        }
        
        // Try the operation
        match operation().await {
            Ok(result) => {
                if backoff.attempt > 0 {
                    tracing::info!(
                        "Operation succeeded after {} attempts", 
                        backoff.attempt
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                let delay = backoff.next_delay();
                
                // Check if delay would exceed max duration
                if let Some(max_duration) = config.max_duration {
                    if start_time.elapsed() + delay > max_duration {
                        tracing::error!(
                            "Next retry would exceed max duration, giving up"
                        );
                        return Err(e);
                    }
                }
                
                tracing::warn!(
                    attempt = backoff.attempt,
                    delay = ?delay,
                    error = %e,
                    "Operation failed, retrying"
                );
                
                tokio::time::sleep(delay).await;
            }
        }
    }
    
    // Should be unreachable if operation returns error
    panic!("Retry loop exited without returning")
}
```

### Usage Examples

```rust
// Basic usage with defaults
let connection = retry_with_backoff(
    RetryConfig::default(),
    || async { database.connect().await }
).await?;

// Custom configuration
let connection = retry_with_backoff(
    RetryConfig {
        max_attempts: Some(5),
        max_duration: Some(Duration::from_secs(30)),
        base_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(10),
        jitter: true,
    },
    || async { database.connect().await }
).await?;

// With error filtering (only retry certain errors)
let connection = retry_with_backoff(
    RetryConfig::default(),
    || async {
        match database.connect().await {
            Err(e) if is_retryable(&e) => Err(e),
            other => other,
        }
    }
).await?;
```

## Advanced Patterns

### 1. Adaptive Backoff

```rust
pub struct AdaptiveBackoff {
    base: Duration,
    current: Duration,
    success_count: u32,
}

impl AdaptiveBackoff {
    pub fn next_delay(&mut self, was_successful: bool) -> Duration {
        if was_successful {
            self.success_count += 1;
            // Reduce delay after consecutive successes
            if self.success_count > 3 {
                self.current = (self.current / 2).max(self.base);
            }
        } else {
            self.success_count = 0;
            // Increase delay on failure
            self.current = (self.current * 2).min(Duration::from_secs(60));
        }
        
        self.current
    }
}
```

### 2. Circuit Breaker Integration

```rust
pub struct CircuitBreakerRetry {
    backoff: ExponentialBackoff,
    consecutive_failures: u32,
    circuit_open: bool,
    circuit_opened_at: Option<Instant>,
}

impl CircuitBreakerRetry {
    pub async fn call<F, Fut, T>(&mut self, operation: F) -> Result<T, Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        // Check if circuit is open
        if self.circuit_open {
            if let Some(opened_at) = self.circuit_opened_at {
                if opened_at.elapsed() < Duration::from_secs(30) {
                    return Err(Error::CircuitOpen);
                } else {
                    // Try to close circuit
                    self.circuit_open = false;
                }
            }
        }
        
        match operation().await {
            Ok(result) => {
                self.consecutive_failures = 0;
                self.backoff.reset();
                Ok(result)
            }
            Err(e) => {
                self.consecutive_failures += 1;
                
                // Open circuit after too many failures
                if self.consecutive_failures >= 5 {
                    self.circuit_open = true;
                    self.circuit_opened_at = Some(Instant::now());
                    tracing::error!("Circuit breaker opened");
                }
                
                Err(e)
            }
        }
    }
}
```

### 3. Deadline-Aware Retry

```rust
pub async fn retry_with_deadline<F, Fut, T>(
    deadline: Instant,
    mut operation: F,
) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut backoff = ExponentialBackoff::with_jitter();
    
    while Instant::now() < deadline {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let delay = backoff.next_delay();
                let remaining = deadline.saturating_duration_since(Instant::now());
                
                if delay > remaining {
                    tracing::warn!(
                        "Next retry would exceed deadline, trying one more time"
                    );
                    // One final attempt
                    return operation().await;
                }
                
                tokio::time::sleep(delay).await;
            }
        }
    }
    
    Err(Error::DeadlineExceeded)
}
```

## Testing Retry Logic

### 1. Deterministic Testing

```rust
pub struct TestBackoff {
    delays: Vec<Duration>,
    index: usize,
}

impl TestBackoff {
    pub fn new(delays: Vec<Duration>) -> Self {
        Self { delays, index: 0 }
    }
    
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.delays.get(self.index)
            .copied()
            .unwrap_or(Duration::from_secs(60));
        self.index += 1;
        delay
    }
}

#[tokio::test]
async fn test_retry_behavior() {
    let mut attempt = 0;
    
    let result = retry_with_backoff(
        RetryConfig {
            max_attempts: Some(3),
            ..Default::default()
        },
        || async {
            attempt += 1;
            if attempt < 3 {
                Err("Not yet")
            } else {
                Ok("Success")
            }
        }
    ).await;
    
    assert_eq!(result, Ok("Success"));
    assert_eq!(attempt, 3);
}
```

### 2. Timeout Testing

```rust
#[tokio::test]
async fn test_retry_timeout() {
    let start = Instant::now();
    
    let result = retry_with_backoff(
        RetryConfig {
            max_duration: Some(Duration::from_secs(2)),
            base_delay: Duration::from_secs(1),
            ..Default::default()
        },
        || async {
            Err::<(), _>("Always fails")
        }
    ).await;
    
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_secs(3));
}
```

## Best Practices

### 1. Always Set Maximum Duration

```rust
// BAD - Could retry forever
RetryConfig {
    max_attempts: None,
    max_duration: None, // Dangerous!
    ..Default::default()
}

// GOOD - Will eventually give up
RetryConfig {
    max_duration: Some(Duration::from_secs(600)), // 10 minutes max
    ..Default::default()
}
```

### 2. Log Retry Attempts

```rust
tracing::warn!(
    attempt = attempt_number,
    delay = ?next_delay,
    elapsed = ?total_elapsed,
    error = %error_message,
    "Retrying failed operation"
);
```

### 3. Make Retries Configurable

```rust
pub struct DatabaseConfig {
    pub url: String,
    pub retry: RetryConfig,
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        Self {
            url: env::var("DATABASE_URL").expect("DATABASE_URL required"),
            retry: RetryConfig {
                max_duration: env::var("DB_RETRY_MAX_DURATION")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .map(Duration::from_secs)
                    .unwrap_or(Duration::from_secs(600)),
                ..Default::default()
            },
        }
    }
}
```

### 4. Consider Operation Type

```rust
// Read operations - retry aggressively
let read_retry = RetryConfig {
    max_attempts: Some(10),
    base_delay: Duration::from_millis(100),
    ..Default::default()
};

// Write operations - retry cautiously
let write_retry = RetryConfig {
    max_attempts: Some(3),
    base_delay: Duration::from_secs(1),
    ..Default::default()
};

// Idempotent operations - retry freely
let idempotent_retry = RetryConfig {
    max_attempts: None,
    max_duration: Some(Duration::from_secs(300)),
    ..Default::default()
};
```

## Common Mistakes

### 1. Not Resetting Backoff

```rust
// BAD - Backoff keeps growing
let mut backoff = ExponentialBackoff::new();

for operation in operations {
    retry_with_backoff(&mut backoff, operation).await?;
    // backoff.attempt keeps incrementing!
}

// GOOD - Reset between operations
for operation in operations {
    let mut backoff = ExponentialBackoff::new();
    retry_with_backoff(&mut backoff, operation).await?;
}
```

### 2. Retrying Non-Retryable Errors

```rust
// BAD - Retrying errors that won't fix themselves
match database.query(bad_sql).await {
    Err(e) => retry(), // SQL syntax won't magically fix itself!
}

// GOOD - Only retry transient errors
fn is_retryable(error: &DatabaseError) -> bool {
    matches!(error, 
        DatabaseError::ConnectionLost |
        DatabaseError::Timeout |
        DatabaseError::TooManyConnections
    )
}
```

### 3. No Maximum Delay

```rust
// BAD - Delay can grow infinitely
let delay = Duration::from_secs(2u64.pow(attempt));

// GOOD - Cap at reasonable maximum
let delay = Duration::from_secs(2u64.pow(attempt)).min(Duration::from_secs(60));
```

## Summary

Retry patterns are essential for reliable database connections:

1. **Use exponential backoff** - Start small, increase gradually
2. **Add jitter** - Prevent thundering herd
3. **Set maximum duration** - Don't retry forever
4. **Log attempts** - Help with debugging
5. **Reset after success** - Start fresh for next operation
6. **Only retry transient errors** - Some errors won't fix themselves

Remember: The goal is to handle temporary failures gracefully without overwhelming the recovering system!
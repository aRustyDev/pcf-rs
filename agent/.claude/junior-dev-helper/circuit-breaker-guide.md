# Circuit Breaker Pattern Guide for Phase 4

## What is a Circuit Breaker?

A circuit breaker is like an electrical circuit breaker in your home - it prevents cascading failures by "tripping" when things go wrong. In software, it stops your app from repeatedly calling a failing service.

## The Three States

```
CLOSED → OPEN → HALF-OPEN → CLOSED
  ↑                           ↓
  └───────── Success ─────────┘
```

### 1. CLOSED (Normal Operation)
- All requests pass through
- Failures are counted
- Too many failures → OPEN

### 2. OPEN (Circuit Tripped)
- All requests fail immediately
- No calls to the service
- After timeout → HALF-OPEN

### 3. HALF-OPEN (Testing Recovery)
- Limited requests allowed
- Success → CLOSED
- Failure → OPEN

## Why Use Circuit Breakers?

### Without Circuit Breaker
```rust
// This keeps hammering a dead service
for _ in 0..1000 {
    match spicedb.check().await {
        Err(_) => {
            // Each failure takes 30 seconds to timeout
            // 1000 requests = 8+ hours of waiting!
        }
    }
}
```

### With Circuit Breaker
```rust
// After 3 failures, subsequent calls fail instantly
for _ in 0..1000 {
    match circuit_breaker.call(|| spicedb.check()).await {
        Err(CircuitOpen) => {
            // Fails immediately - no waiting
            // Total time: seconds, not hours
        }
    }
}
```

## Basic Implementation

### 1. Simple Circuit Breaker Structure

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open(Instant), // When it opened
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
}
```

### 2. Core Logic

```rust
impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            failure_threshold: 3,    // Open after 3 failures
            success_threshold: 2,    // Close after 2 successes
            timeout: Duration::from_secs(60), // Try again after 60s
        }
    }
    
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitError<E>>
    where
        F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
    {
        // Check state
        let state = self.state.read().await.clone();
        
        match state {
            CircuitState::Open(opened_at) => {
                if opened_at.elapsed() > self.timeout {
                    // Time to try again
                    *self.state.write().await = CircuitState::HalfOpen;
                } else {
                    // Still open - fail fast
                    return Err(CircuitError::CircuitOpen);
                }
            }
            _ => {} // Closed or HalfOpen - proceed
        }
        
        // Execute operation
        match operation().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(error) => {
                self.record_failure().await;
                Err(CircuitError::OperationFailed(error))
            }
        }
    }
}
```

### 3. State Management

```rust
impl CircuitBreaker {
    async fn record_success(&self) {
        // Reset failure count
        *self.failure_count.write().await = 0;
        
        // If half-open, count successes
        if matches!(*self.state.read().await, CircuitState::HalfOpen) {
            let mut success_count = self.success_count.write().await;
            *success_count += 1;
            
            // Enough successes? Close the circuit
            if *success_count >= self.success_threshold {
                *self.state.write().await = CircuitState::Closed;
                *success_count = 0;
                tracing::info!("Circuit breaker closed");
            }
        }
    }
    
    async fn record_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;
        
        // Too many failures? Open the circuit
        if *failure_count >= self.failure_threshold {
            *self.state.write().await = CircuitState::Open(Instant::now());
            tracing::warn!("Circuit breaker opened after {} failures", *failure_count);
        }
        
        // If half-open, one failure reopens immediately
        if matches!(*self.state.read().await, CircuitState::HalfOpen) {
            *self.state.write().await = CircuitState::Open(Instant::now());
            tracing::warn!("Circuit breaker reopened from half-open");
        }
    }
}
```

## Using in Authorization

### 1. Wrap SpiceDB Calls

```rust
pub async fn check_permission_with_circuit_breaker(
    circuit_breaker: &CircuitBreaker,
    spicedb: &SpiceDBClient,
    user_id: &str,
    resource: &str,
    action: &str,
) -> Result<bool, Error> {
    match circuit_breaker.call(|| {
        // Create the future for the circuit breaker
        let spicedb = spicedb.clone();
        let subject = format!("user:{}", user_id);
        let resource = resource.to_string();
        let permission = action.to_string();
        
        Box::pin(async move {
            // Add timeout to prevent hanging
            tokio::time::timeout(
                Duration::from_secs(2),
                spicedb.check_permission(subject, resource, permission)
            ).await
            .map_err(|_| "SpiceDB timeout")?
        })
    }).await {
        Ok(result) => Ok(result),
        Err(CircuitError::CircuitOpen) => {
            // Circuit is open - use fallback
            tracing::warn!("Circuit open, using fallback rules");
            Ok(apply_fallback_rules(user_id, resource, action))
        }
        Err(CircuitError::OperationFailed(_)) => {
            // Operation failed - use fallback
            tracing::warn!("SpiceDB failed, using fallback rules");
            Ok(apply_fallback_rules(user_id, resource, action))
        }
    }
}
```

### 2. Configure for Your Needs

```rust
impl CircuitBreaker {
    pub fn with_config(
        failure_threshold: u32,
        success_threshold: u32,
        timeout: Duration,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            failure_threshold,
            success_threshold,
            timeout,
        }
    }
}

// Example configurations
let strict_breaker = CircuitBreaker::with_config(
    3,  // Open after 3 failures
    5,  // Need 5 successes to close
    Duration::from_secs(120), // 2 minute timeout
);

let lenient_breaker = CircuitBreaker::with_config(
    10, // Open after 10 failures
    2,  // Only 2 successes to close
    Duration::from_secs(30), // 30 second timeout
);
```

## Testing Circuit Breakers

### 1. Test State Transitions

```rust
#[tokio::test]
async fn test_circuit_opens_after_failures() {
    let breaker = CircuitBreaker::new();
    let failing_op = || Box::pin(async { Err::<(), _>("error") });
    
    // First failures counted
    for i in 1..=3 {
        let result = breaker.call(failing_op).await;
        assert!(matches!(result, Err(CircuitError::OperationFailed(_))));
    }
    
    // Fourth call should fail fast
    let result = breaker.call(failing_op).await;
    assert!(matches!(result, Err(CircuitError::CircuitOpen)));
}
```

### 2. Test Half-Open Recovery

```rust
#[tokio::test]
async fn test_circuit_recovers() {
    let breaker = CircuitBreaker::with_config(2, 2, Duration::from_millis(100));
    
    // Open the circuit
    let failing_op = || Box::pin(async { Err::<(), _>("error") });
    for _ in 0..2 {
        let _ = breaker.call(failing_op).await;
    }
    
    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Success should move to half-open then closed
    let success_op = || Box::pin(async { Ok(42) });
    for _ in 0..2 {
        let result = breaker.call(success_op).await;
        assert!(result.is_ok());
    }
    
    // Should be closed now
    assert!(matches!(
        *breaker.state.read().await,
        CircuitState::Closed
    ));
}
```

## Common Patterns

### 1. Metrics Collection

```rust
impl CircuitBreaker {
    pub async fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            state: self.state.read().await.clone(),
            failure_count: *self.failure_count.read().await,
            success_count: *self.success_count.read().await,
        }
    }
}

// Export to Prometheus
let metrics = breaker.metrics().await;
match metrics.state {
    CircuitState::Closed => circuit_state.set(0.0),
    CircuitState::Open(_) => circuit_state.set(1.0),
    CircuitState::HalfOpen => circuit_state.set(0.5),
}
```

### 2. Multiple Service Breakers

```rust
pub struct ServiceBreakers {
    spicedb: Arc<CircuitBreaker>,
    cache: Arc<CircuitBreaker>,
    database: Arc<CircuitBreaker>,
}

impl ServiceBreakers {
    pub fn health_status(&self) -> HealthStatus {
        // Check all breakers
        let spicedb_open = self.spicedb.is_open().await;
        let cache_open = self.cache.is_open().await;
        let database_open = self.database.is_open().await;
        
        if database_open {
            HealthStatus::Critical // Can't function without DB
        } else if spicedb_open {
            HealthStatus::Degraded // Can use fallback auth
        } else if cache_open {
            HealthStatus::Warning  // Just slower
        } else {
            HealthStatus::Healthy
        }
    }
}
```

### 3. Configurable Fallback

```rust
pub struct AuthorizationService {
    circuit_breaker: Arc<CircuitBreaker>,
    fallback_strategy: FallbackStrategy,
}

pub enum FallbackStrategy {
    DenyAll,        // Most secure
    AllowHealthOnly, // Allow health checks
    AllowReadOnly,   // Allow all reads
    UseCachedRules,  // Use cached permissions
}

impl AuthorizationService {
    async fn check_with_fallback(&self, user: &str, resource: &str, action: &str) -> bool {
        if self.circuit_breaker.is_open().await {
            match self.fallback_strategy {
                FallbackStrategy::DenyAll => false,
                FallbackStrategy::AllowHealthOnly => {
                    resource == "health:status" && action == "read"
                }
                FallbackStrategy::AllowReadOnly => action == "read",
                FallbackStrategy::UseCachedRules => {
                    self.check_cached_rules(user, resource, action).await
                }
            }
        } else {
            // Normal operation
            self.check_with_spicedb(user, resource, action).await
        }
    }
}
```

## Best Practices

### 1. Choose Appropriate Thresholds
```rust
// For critical services - be more tolerant
let critical_service = CircuitBreaker::with_config(
    10,  // More failures before opening
    3,   // Fewer successes to recover
    Duration::from_secs(30), // Quick retry
);

// For optional services - be strict
let optional_service = CircuitBreaker::with_config(
    2,   // Open quickly
    5,   // Need solid recovery
    Duration::from_secs(300), // Longer timeout
);
```

### 2. Add Jitter to Prevent Thundering Herd
```rust
use rand::Rng;

async fn with_jitter(&self) -> Duration {
    let base = self.timeout;
    let jitter = rand::thread_rng().gen_range(0..1000);
    base + Duration::from_millis(jitter)
}
```

### 3. Log State Changes
```rust
async fn change_state(&self, new_state: CircuitState) {
    let old_state = self.state.read().await.clone();
    *self.state.write().await = new_state.clone();
    
    tracing::info!(
        "Circuit breaker state changed: {:?} -> {:?}",
        old_state,
        new_state
    );
    
    // Alert on critical changes
    if matches!(new_state, CircuitState::Open(_)) {
        alert_ops_team("Circuit breaker opened!");
    }
}
```

## Common Mistakes

### 1. Wrong Error Handling
```rust
// BAD: Treats all errors the same
match operation().await {
    Err(_) => self.record_failure(),
}

// GOOD: Distinguish error types
match operation().await {
    Err(e) if is_transient(&e) => self.record_failure(),
    Err(e) if is_client_error(&e) => {
        // Don't count client errors as failures
        return Err(e);
    }
}
```

### 2. No Timeout Protection
```rust
// BAD: Can hang forever
spicedb.check().await

// GOOD: Always use timeout
tokio::time::timeout(
    Duration::from_secs(2),
    spicedb.check()
).await
```

### 3. Shared State Issues
```rust
// BAD: Multiple breakers share state
let state = Arc::new(RwLock::new(CircuitState::Closed));
let breaker1 = CircuitBreaker { state: state.clone(), ... };
let breaker2 = CircuitBreaker { state: state.clone(), ... };

// GOOD: Each breaker has own state
let breaker1 = CircuitBreaker::new();
let breaker2 = CircuitBreaker::new();
```

## Debugging Tips

### 1. Add Debug Endpoints
```rust
async fn debug_handler(State(breakers): State<ServiceBreakers>) -> Json<DebugInfo> {
    Json(DebugInfo {
        spicedb_state: breakers.spicedb.state().await,
        spicedb_failures: breakers.spicedb.failure_count().await,
        cache_state: breakers.cache.state().await,
        // ... etc
    })
}
```

### 2. Test with Chaos
```rust
#[cfg(test)]
mod chaos_tests {
    #[tokio::test]
    async fn test_random_failures() {
        let breaker = CircuitBreaker::new();
        let mut rng = rand::thread_rng();
        
        for _ in 0..100 {
            let should_fail = rng.gen_bool(0.3); // 30% failure rate
            
            let result = breaker.call(|| {
                Box::pin(async move {
                    if should_fail {
                        Err("random failure")
                    } else {
                        Ok(())
                    }
                })
            }).await;
            
            // Breaker should handle this gracefully
            assert!(breaker.metrics().await.is_valid());
        }
    }
}
```

## Integration Example

Here's how it all comes together in the authorization service:

```rust
pub struct AuthorizationService {
    spicedb: Arc<SpiceDBClient>,
    circuit_breaker: Arc<CircuitBreaker>,
    cache: Arc<AuthCache>,
    metrics: Arc<Metrics>,
}

impl AuthorizationService {
    pub async fn is_authorized(
        &self,
        ctx: &Context<'_>,
        resource: &str,
        action: &str,
    ) -> Result<(), Error> {
        let auth = ctx.data::<AuthContext>()?;
        let user_id = auth.require_auth()?;
        
        // Track request
        self.metrics.auth_requests.inc();
        let start = Instant::now();
        
        // Check cache
        let cache_key = format!("{}:{}:{}", user_id, resource, action);
        if let Some(allowed) = self.cache.get(&cache_key).await {
            self.metrics.cache_hits.inc();
            return if allowed { Ok(()) } else { Err(forbidden()) };
        }
        
        // Check with circuit breaker
        let allowed = match self.circuit_breaker.call(|| {
            self.check_spicedb(user_id, resource, action)
        }).await {
            Ok(allowed) => {
                self.metrics.spicedb_success.inc();
                allowed
            }
            Err(CircuitError::CircuitOpen) => {
                self.metrics.circuit_open.inc();
                apply_fallback_rules(user_id, resource, action)
            }
            Err(CircuitError::OperationFailed(_)) => {
                self.metrics.spicedb_failures.inc();
                apply_fallback_rules(user_id, resource, action)
            }
        };
        
        // Cache positive results
        if allowed {
            self.cache.set(cache_key, true, Duration::from_secs(300)).await;
        }
        
        // Record timing
        self.metrics.auth_duration.observe(start.elapsed().as_secs_f64());
        
        if allowed { Ok(()) } else { Err(forbidden()) }
    }
}
```

## Summary

Circuit breakers are essential for building resilient systems. They:
- Prevent cascading failures
- Reduce load on struggling services  
- Provide fast failure responses
- Enable graceful degradation

Remember: It's better to serve some requests with fallback rules than to have your entire system hang waiting for a dead service!

## Next Steps

1. Implement a basic circuit breaker
2. Add it to your SpiceDB calls
3. Test failure scenarios
4. Monitor circuit breaker state
5. Tune thresholds based on real usage

For more details, see:
- [Authorization Tutorial](./authorization-tutorial.md) - How this fits in auth
- [Cache Strategies](./cache-strategies-guide.md) - Caching with circuit breakers
- [Common Authorization Errors](./authorization-common-errors.md) - Debugging issues
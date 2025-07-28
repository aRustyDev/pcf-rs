# Timeout Management Best Practices

## Why Timeouts Matter

Without proper timeouts, your application can:
- **Hang forever**: One slow dependency blocks everything
- **Cascade failures**: Timeouts trigger more timeouts
- **Waste resources**: Connections held waiting for responses
- **Poor user experience**: Users stare at loading spinners

## The Timeout Hierarchy

Timeouts must cascade properly - each layer should timeout BEFORE its parent:

```
┌─────────────────────────────────────┐
│   HTTP Server (30s)                 │ <- Longest timeout
│  ┌───────────────────────────────┐  │
│  │  GraphQL Execution (25s)      │  │ <- Shorter than HTTP
│  │ ┌───────────────────────────┐ │  │
│  │ │  Database Query (20s)     │ │  │ <- Shortest timeout
│  │ └───────────────────────────┘ │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

### Why This Order?

If database timeout (20s) > GraphQL timeout (25s):
- GraphQL times out at 25s
- Database query continues running
- Wastes database resources
- Connection not returned to pool

## Implementing Timeout Hierarchy

### 1. HTTP Server Timeout

```rust
use axum::{Router, Server};
use tower::timeout::TimeoutLayer;
use std::time::Duration;

pub fn create_server(app: Router) -> Server {
    Server::bind(&addr)
        .serve(
            app.layer(TimeoutLayer::new(Duration::from_secs(30)))
                .into_make_service()
        )
}

// Or in the handler
async fn graphql_handler(
    State(schema): State<Schema>,
    req: Request,
) -> Result<Response> {
    // Apply timeout to entire request
    tokio::time::timeout(
        Duration::from_secs(30),
        schema.execute(req)
    )
    .await
    .map_err(|_| Error::RequestTimeout)?
}
```

### 2. GraphQL Execution Timeout

```rust
use async_graphql::*;

pub struct TimeoutExtension;

#[async_trait::async_trait]
impl Extension for TimeoutExtension {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let timeout = Duration::from_secs(25);
        
        match tokio::time::timeout(timeout, next.run(ctx)).await {
            Ok(resp) => resp,
            Err(_) => Response::from_errors(vec![
                ServerError::new("Query execution timeout", None)
            ]),
        }
    }
}

// Add to schema
let schema = Schema::build(Query, Mutation, Subscription)
    .extension(TimeoutExtension)
    .finish();
```

### 3. Database Query Timeout

```rust
pub struct DatabaseService {
    pool: PgPool,
    query_timeout: Duration,
}

impl DatabaseService {
    pub async fn query<T>(&self, sql: &str) -> Result<T> {
        // Set statement timeout
        let query = format!(
            "SET LOCAL statement_timeout = {}; {}",
            self.query_timeout.as_millis(),
            sql
        );
        
        // Also apply Rust-level timeout
        tokio::time::timeout(
            self.query_timeout,
            self.pool.fetch_one(&query)
        )
        .await
        .map_err(|_| Error::DatabaseTimeout)?
    }
}
```

## Context-Aware Timeouts

Pass remaining time budget through the request:

```rust
#[derive(Clone)]
pub struct TimeoutContext {
    deadline: Instant,
}

impl TimeoutContext {
    pub fn new(total_timeout: Duration) -> Self {
        Self {
            deadline: Instant::now() + total_timeout,
        }
    }
    
    pub fn remaining(&self) -> Duration {
        self.deadline
            .saturating_duration_since(Instant::now())
    }
    
    pub fn child_timeout(&self, buffer: Duration) -> Duration {
        // Leave buffer for parent to handle timeout
        self.remaining().saturating_sub(buffer)
    }
}

// In GraphQL context
impl Query {
    async fn complex_query(&self, ctx: &Context<'_>) -> Result<Data> {
        let timeout_ctx = ctx.data::<TimeoutContext>()?;
        
        // Give database 5s less than we have
        let db_timeout = timeout_ctx.child_timeout(Duration::from_secs(5));
        
        // If we only have 2s left, use minimum timeout
        let db_timeout = db_timeout.max(Duration::from_secs(1));
        
        self.db.query_with_timeout(query, db_timeout).await
    }
}
```

## Smart Timeout Strategies

### 1. Different Timeouts for Different Operations

```rust
pub enum OperationType {
    SimpleQuery,      // Fast queries
    ComplexQuery,     // Reports, analytics
    Mutation,         // Writes
    FileUpload,       // Large data
}

impl OperationType {
    pub fn timeout(&self) -> Duration {
        match self {
            OperationType::SimpleQuery => Duration::from_secs(5),
            OperationType::ComplexQuery => Duration::from_secs(30),
            OperationType::Mutation => Duration::from_secs(10),
            OperationType::FileUpload => Duration::from_secs(300),
        }
    }
}

// Detect operation type from GraphQL
fn detect_operation_type(query: &str) -> OperationType {
    if query.contains("mutation") {
        if query.contains("uploadFile") {
            OperationType::FileUpload
        } else {
            OperationType::Mutation
        }
    } else if query.contains("report") || query.contains("analytics") {
        OperationType::ComplexQuery
    } else {
        OperationType::SimpleQuery
    }
}
```

### 2. Adaptive Timeouts

Adjust timeouts based on system load:

```rust
pub struct AdaptiveTimeout {
    base_timeout: Duration,
    current_load: Arc<AtomicUsize>,
}

impl AdaptiveTimeout {
    pub fn calculate(&self) -> Duration {
        let load = self.current_load.load(Ordering::Relaxed);
        
        if load > 1000 {
            // Under heavy load, fail fast
            self.base_timeout / 2
        } else if load < 100 {
            // Low load, be generous
            self.base_timeout * 2
        } else {
            self.base_timeout
        }
    }
}
```

### 3. Retry with Exponential Backoff

```rust
pub async fn with_retry<T, F, Fut>(
    operation: F,
    max_retries: u32,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay = Duration::from_millis(100);
    
    loop {
        match tokio::time::timeout(delay * 2, operation()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) if !e.is_retryable() => return Err(e),
            Ok(Err(e)) | Err(_) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(Error::MaxRetriesExceeded);
                }
                
                // Exponential backoff with jitter
                delay = delay * 2 + Duration::from_millis(
                    rand::random::<u64>() % 100
                );
                
                tokio::time::sleep(delay).await;
            }
        }
    }
}
```

## Handling Timeout Errors

### 1. Graceful Degradation

```rust
impl Query {
    async fn dashboard(&self, ctx: &Context<'_>) -> Result<Dashboard> {
        // Try to load all data with timeouts
        let (user, stats, recommendations) = tokio::join!(
            timeout(Duration::from_secs(2), self.load_user(ctx)),
            timeout(Duration::from_secs(3), self.load_stats(ctx)),
            timeout(Duration::from_secs(1), self.load_recommendations(ctx)),
        );
        
        Ok(Dashboard {
            user: user.unwrap_or_else(|_| {
                warn!("User data timeout, using cached");
                self.cached_user(ctx)
            }),
            stats: stats.unwrap_or_else(|_| {
                warn!("Stats timeout, using defaults");
                Stats::default()
            }),
            recommendations: recommendations.unwrap_or_else(|_| {
                warn!("Recommendations timeout, skipping");
                vec![]
            }),
        })
    }
}
```

### 2. Circuit Breaker Pattern

```rust
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure: RwLock<Option<Instant>>,
    state: RwLock<CircuitState>,
    timeout: Duration,
}

#[derive(Clone, Copy)]
enum CircuitState {
    Closed,  // Normal operation
    Open,    // Failing, reject requests
    HalfOpen, // Testing if recovered
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        match *self.state.read().await {
            CircuitState::Open => {
                // Check if we should try again
                if self.should_attempt().await {
                    *self.state.write().await = CircuitState::HalfOpen;
                } else {
                    return Err(Error::CircuitOpen);
                }
            }
            CircuitState::HalfOpen => {
                // Single test request
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }
        
        match timeout(self.timeout, f).await {
            Ok(Ok(result)) => {
                self.on_success().await;
                Ok(result)
            }
            Ok(Err(e)) | Err(_) => {
                self.on_failure().await;
                Err(Error::CircuitTimeout)
            }
        }
    }
    
    async fn on_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst);
        
        if count >= 5 {
            *self.state.write().await = CircuitState::Open;
            *self.last_failure.write().await = Some(Instant::now());
            warn!("Circuit breaker opened after {} failures", count);
        }
    }
}
```

### 3. Timeout Budgets

Allocate time budgets for request phases:

```rust
pub struct TimeoutBudget {
    total: Duration,
    checkpoints: Vec<(&'static str, Duration)>,
}

impl TimeoutBudget {
    pub fn new(total: Duration) -> Self {
        Self {
            total,
            checkpoints: vec![
                ("auth", Duration::from_millis(500)),
                ("validation", Duration::from_millis(200)),
                ("database", Duration::from_secs(5)),
                ("post_processing", Duration::from_millis(300)),
            ],
        }
    }
    
    pub async fn run_with_budget<F, T>(&self, phase: &str, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        let budget = self.checkpoints
            .iter()
            .find(|(name, _)| name == &phase)
            .map(|(_, duration)| *duration)
            .unwrap_or(Duration::from_secs(1));
        
        let start = Instant::now();
        
        match timeout(budget, f).await {
            Ok(result) => {
                let elapsed = start.elapsed();
                if elapsed > budget * 80 / 100 {
                    warn!(
                        "Phase {} used {}ms of {}ms budget",
                        phase,
                        elapsed.as_millis(),
                        budget.as_millis()
                    );
                }
                result
            }
            Err(_) => {
                error!("Phase {} exceeded {}ms budget", phase, budget.as_millis());
                Err(Error::BudgetExceeded(phase))
            }
        }
    }
}
```

## Testing Timeouts

### 1. Unit Tests with Simulated Delays

```rust
#[tokio::test]
async fn test_timeout_handling() {
    let service = MyService::new();
    
    // Mock slow dependency
    let slow_db = MockDatabase::new()
        .with_delay(Duration::from_secs(10));
    
    service.set_db(slow_db);
    
    // Should timeout
    let result = timeout(
        Duration::from_secs(1),
        service.fetch_data()
    ).await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Operation timed out");
}
```

### 2. Integration Tests with Real Timeouts

```rust
#[tokio::test]
async fn test_cascading_timeouts() {
    let app = create_test_app();
    
    // Query that intentionally takes long
    let query = r#"
        query {
            slowOperation(delay: 35)
        }
    "#;
    
    let start = Instant::now();
    let response = app.graphql_query(query).await;
    let elapsed = start.elapsed();
    
    // Should timeout at HTTP layer (30s), not wait full 35s
    assert!(elapsed < Duration::from_secs(32));
    assert!(elapsed > Duration::from_secs(29));
    
    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
}
```

### 3. Chaos Testing

```rust
// Randomly inject delays to test timeout handling
pub struct ChaosMiddleware {
    delay_probability: f64,
    max_delay: Duration,
}

impl ChaosMiddleware {
    pub async fn maybe_delay(&self) {
        if rand::random::<f64>() < self.delay_probability {
            let delay = Duration::from_millis(
                rand::random::<u64>() % self.max_delay.as_millis() as u64
            );
            
            warn!("Chaos: injecting {}ms delay", delay.as_millis());
            tokio::time::sleep(delay).await;
        }
    }
}
```

## Monitoring Timeouts

```rust
// Track timeout metrics
counter!("request_timeouts_total",
    "layer" => "http",
    "operation" => operation_name,
).increment(1);

histogram!("timeout_margin_seconds",
    "layer" => "database",
).record((deadline - Instant::now()).as_secs_f64());

// Alert on timeout storms
if timeout_rate > 0.01 { // > 1% timeouts
    alert!("High timeout rate: {:.1}%", timeout_rate * 100.0);
}
```

## Common Timeout Mistakes

### 1. No Timeout Coordination

```rust
// ❌ BAD: Random timeouts
let http_timeout = Duration::from_secs(10);
let db_timeout = Duration::from_secs(30); // Longer than HTTP!

// ✅ GOOD: Coordinated hierarchy
let http_timeout = Duration::from_secs(30);
let graphql_timeout = http_timeout - Duration::from_secs(5);
let db_timeout = graphql_timeout - Duration::from_secs(5);
```

### 2. Timeout Too Short

```rust
// ❌ BAD: 100ms timeout for complex query
timeout(Duration::from_millis(100), complex_analytics_query())

// ✅ GOOD: Appropriate timeout for operation
let timeout = match query_complexity {
    Low => Duration::from_secs(1),
    Medium => Duration::from_secs(5),
    High => Duration::from_secs(30),
};
```

### 3. Not Handling Timeout Errors

```rust
// ❌ BAD: Timeout crashes everything
let data = timeout(duration, fetch_data()).await?;

// ✅ GOOD: Graceful handling
let data = match timeout(duration, fetch_data()).await {
    Ok(data) => data,
    Err(_) => {
        warn!("Fetch timeout, using stale data");
        cache.get_stale(key).unwrap_or_default()
    }
};
```

## Summary

Proper timeout management requires:
1. **Hierarchy**: Each layer times out before its parent
2. **Context**: Pass timeout budget through request
3. **Flexibility**: Different timeouts for different operations
4. **Resilience**: Handle timeouts gracefully
5. **Monitoring**: Track timeout patterns

Remember: It's better to fail fast than hang forever!
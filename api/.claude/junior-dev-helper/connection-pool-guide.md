# Connection Pool Guide for Junior Developers

## What is Connection Pooling?

Imagine a restaurant with limited tables. Instead of building a new table for each customer (expensive!), you reuse tables as customers leave. Connection pooling works the same way - we reuse database connections instead of creating new ones for each request.

## Why Use Connection Pools?

### Without Pooling (Slow!)
```
Request 1: Connect (100ms) → Query (10ms) → Disconnect (50ms) = 160ms
Request 2: Connect (100ms) → Query (10ms) → Disconnect (50ms) = 160ms
Request 3: Connect (100ms) → Query (10ms) → Disconnect (50ms) = 160ms
Total: 480ms
```

### With Pooling (Fast!)
```
Startup: Create 3 connections (300ms)
Request 1: Get connection (1ms) → Query (10ms) → Return (1ms) = 12ms
Request 2: Get connection (1ms) → Query (10ms) → Return (1ms) = 12ms
Request 3: Get connection (1ms) → Query (10ms) → Return (1ms) = 12ms
Total: 36ms (after startup)
```

## Basic Pool Structure

```rust
pub struct ConnectionPool<T> {
    // Available connections ready to use
    available: Arc<RwLock<Vec<T>>>,
    
    // Currently in-use connections
    in_use: Arc<RwLock<HashSet<Uuid>>>,
    
    // Configuration
    config: PoolConfig,
    
    // Semaphore to limit total connections
    semaphore: Arc<Semaphore>,
}

pub struct PoolConfig {
    pub min_connections: usize,    // Minimum to keep ready
    pub max_connections: usize,    // Maximum allowed total
    pub idle_timeout: Duration,    // When to close idle connections
    pub max_lifetime: Duration,    // When to recreate connections
}
```

## Pool Lifecycle

### 1. Initialization

```rust
impl<T> ConnectionPool<T> {
    pub async fn new(config: PoolConfig) -> Self {
        let pool = Self {
            available: Arc::new(RwLock::new(Vec::new())),
            in_use: Arc::new(RwLock::new(HashSet::new())),
            config,
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
        };
        
        // Pre-create minimum connections
        pool.initialize().await;
        pool
    }
    
    async fn initialize(&self) {
        for _ in 0..self.config.min_connections {
            match self.create_connection().await {
                Ok(conn) => {
                    self.available.write().await.push(conn);
                }
                Err(e) => {
                    tracing::warn!("Failed to create initial connection: {}", e);
                }
            }
        }
    }
}
```

### 2. Acquiring Connections

```rust
pub async fn acquire(&self) -> Result<PooledConnection<T>, PoolError> {
    // Try to get an existing connection
    if let Some(conn) = self.try_get_available().await {
        return Ok(conn);
    }
    
    // No available connections - try to create new one
    if self.can_create_more().await {
        return self.create_and_acquire().await;
    }
    
    // At max capacity - wait for one to be returned
    self.wait_for_available().await
}

async fn try_get_available(&self) -> Option<PooledConnection<T>> {
    let mut available = self.available.write().await;
    
    if let Some(conn) = available.pop() {
        let id = Uuid::new_v4();
        self.in_use.write().await.insert(id);
        
        Some(PooledConnection {
            conn: Some(conn),
            pool: Arc::clone(self),
            id,
        })
    } else {
        None
    }
}
```

### 3. Returning Connections

```rust
pub struct PooledConnection<T> {
    conn: Option<T>,
    pool: Arc<ConnectionPool<T>>,
    id: Uuid,
}

impl<T> Drop for PooledConnection<T> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            // Return to pool when dropped
            let pool = Arc::clone(&self.pool);
            let id = self.id;
            
            tokio::spawn(async move {
                pool.return_connection(conn, id).await;
            });
        }
    }
}

impl<T> ConnectionPool<T> {
    async fn return_connection(&self, conn: T, id: Uuid) {
        // Remove from in-use set
        self.in_use.write().await.remove(&id);
        
        // Check if connection is still healthy
        if self.is_connection_healthy(&conn).await {
            self.available.write().await.push(conn);
        } else {
            // Unhealthy - create replacement
            if let Ok(new_conn) = self.create_connection().await {
                self.available.write().await.push(new_conn);
            }
        }
    }
}
```

## Health Monitoring

### Connection Health Checks

```rust
async fn is_connection_healthy(&self, conn: &T) -> bool {
    // Check if connection is alive
    match conn.ping().await {
        Ok(_) => true,
        Err(_) => false,
    }
}

// Background health monitor
async fn start_health_monitor(self: Arc<Self>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            self.check_pool_health().await;
        }
    });
}

async fn check_pool_health(&self) {
    let mut available = self.available.write().await;
    let mut healthy = Vec::new();
    
    // Check each connection
    while let Some(conn) = available.pop() {
        if self.is_connection_healthy(&conn).await {
            healthy.push(conn);
        } else {
            tracing::warn!("Removing unhealthy connection from pool");
        }
    }
    
    // Put healthy connections back
    for conn in healthy {
        available.push(conn);
    }
    
    // Ensure minimum connections
    let current_count = available.len();
    if current_count < self.config.min_connections {
        for _ in current_count..self.config.min_connections {
            if let Ok(conn) = self.create_connection().await {
                available.push(conn);
            }
        }
    }
}
```

## Advanced Features

### 1. Connection Lifetime Management

```rust
pub struct PooledConnection<T> {
    conn: Option<T>,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
}

impl<T> ConnectionPool<T> {
    async fn should_retire_connection(&self, conn: &PooledConnection<T>) -> bool {
        // Too old?
        if conn.created_at.elapsed() > self.config.max_lifetime {
            return true;
        }
        
        // Used too many times?
        if conn.use_count > 1000 {
            return true;
        }
        
        // Idle too long?
        if conn.last_used.elapsed() > self.config.idle_timeout {
            return true;
        }
        
        false
    }
}
```

### 2. Adaptive Pool Sizing

```rust
pub struct AdaptivePool<T> {
    base_pool: ConnectionPool<T>,
    metrics: Arc<PoolMetrics>,
}

impl<T> AdaptivePool<T> {
    async fn adjust_pool_size(&self) {
        let metrics = self.metrics.snapshot();
        
        // High wait times? Increase pool size
        if metrics.avg_wait_time > Duration::from_millis(100) {
            let new_max = (self.base_pool.config.max_connections * 1.2) as usize;
            self.base_pool.resize(new_max).await;
        }
        
        // Many idle connections? Decrease pool size
        if metrics.idle_ratio > 0.8 {
            let new_max = (self.base_pool.config.max_connections * 0.8) as usize;
            self.base_pool.resize(new_max.max(self.base_pool.config.min_connections)).await;
        }
    }
}
```

### 3. Warmup Strategies

```rust
impl<T> ConnectionPool<T> {
    /// Gradually warm up connections to avoid thundering herd
    pub async fn gradual_warmup(&self) {
        let target = self.config.min_connections;
        let batch_size = 5;
        
        for i in (0..target).step_by(batch_size) {
            let batch_end = (i + batch_size).min(target);
            
            // Create batch in parallel
            let mut tasks = Vec::new();
            for _ in i..batch_end {
                tasks.push(self.create_connection());
            }
            
            let results = futures::future::join_all(tasks).await;
            
            // Add successful connections
            let mut available = self.available.write().await;
            for result in results {
                if let Ok(conn) = result {
                    available.push(conn);
                }
            }
            
            // Small delay between batches
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
```

## Common Pool Patterns

### 1. Connection Wrapper

```rust
pub struct DatabaseConnection {
    inner: SurrealDB,
    id: Uuid,
    created_at: Instant,
    stats: ConnectionStats,
}

impl DatabaseConnection {
    pub async fn query(&mut self, sql: &str) -> Result<Value, Error> {
        self.stats.query_count += 1;
        self.stats.last_used = Instant::now();
        
        let start = Instant::now();
        let result = self.inner.query(sql).await;
        
        self.stats.total_query_time += start.elapsed();
        result
    }
}
```

### 2. Pool with Metrics

```rust
#[derive(Default)]
pub struct PoolMetrics {
    connections_created: AtomicU64,
    connections_closed: AtomicU64,
    acquire_count: AtomicU64,
    acquire_timeout_count: AtomicU64,
    total_wait_time: AtomicU64,
}

impl PoolMetrics {
    pub fn record_acquire(&self, wait_time: Duration) {
        self.acquire_count.fetch_add(1, Ordering::Relaxed);
        self.total_wait_time.fetch_add(
            wait_time.as_millis() as u64,
            Ordering::Relaxed
        );
    }
    
    pub fn avg_wait_time(&self) -> Duration {
        let total = self.total_wait_time.load(Ordering::Relaxed);
        let count = self.acquire_count.load(Ordering::Relaxed);
        
        if count > 0 {
            Duration::from_millis(total / count)
        } else {
            Duration::ZERO
        }
    }
}
```

### 3. Typed Pool

```rust
// Different pools for different operations
pub struct DatabasePools {
    read_pool: Arc<ConnectionPool<ReadConnection>>,
    write_pool: Arc<ConnectionPool<WriteConnection>>,
    admin_pool: Arc<ConnectionPool<AdminConnection>>,
}

impl DatabasePools {
    pub async fn read(&self) -> Result<PooledConnection<ReadConnection>, Error> {
        self.read_pool.acquire().await
    }
    
    pub async fn write(&self) -> Result<PooledConnection<WriteConnection>, Error> {
        self.write_pool.acquire().await
    }
}
```

## Configuration Best Practices

### 1. Pool Sizing

```rust
impl PoolConfig {
    pub fn for_environment(env: &str) -> Self {
        match env {
            "development" => Self {
                min_connections: 2,
                max_connections: 10,
                idle_timeout: Duration::from_secs(300),
                max_lifetime: Duration::from_secs(3600),
            },
            "production" => Self {
                min_connections: 10,
                max_connections: 100,
                idle_timeout: Duration::from_secs(60),
                max_lifetime: Duration::from_secs(1800),
            },
            _ => Self::default(),
        }
    }
}
```

### 2. Timeout Configuration

```rust
pub struct TimeoutConfig {
    pub acquire_timeout: Duration,    // How long to wait for connection
    pub query_timeout: Duration,      // How long queries can run
    pub idle_timeout: Duration,       // When to close idle connections
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            acquire_timeout: Duration::from_secs(5),
            query_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
        }
    }
}
```

### 3. Retry Configuration (Phase 6 Requirement)

```rust
pub struct RetryConfig {
    pub initial_interval: Duration,   // First retry delay
    pub max_interval: Duration,       // Maximum retry delay
    pub exponential_base: u32,        // Multiplier for backoff
    pub max_elapsed_time: Option<Duration>, // Total retry duration
}

impl RetryConfig {
    /// Configuration per SPEC.md requirements
    pub fn for_production() -> Self {
        Self {
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(60),
            exponential_base: 2,
            max_elapsed_time: None, // Retry indefinitely during startup
        }
    }
}

/// Health-aware pool with retry logic
pub struct HealthAwarePool<T> {
    inner: Pool<T>,
    retry_config: RetryConfig,
    health_status: Arc<RwLock<HealthStatus>>,
    is_startup: AtomicBool,
}

impl<T: Connection> HealthAwarePool<T> {
    pub async fn acquire(&self) -> Result<PooledConnection<T>> {
        let mut attempt = 0;
        let mut delay = self.retry_config.initial_interval;
        let start = Instant::now();
        
        loop {
            match self.inner.acquire().await {
                Ok(conn) => {
                    self.mark_healthy();
                    return Ok(conn);
                }
                Err(e) => {
                    attempt += 1;
                    self.mark_unhealthy();
                    
                    // Check if we should give up
                    if !self.should_retry(&start, attempt) {
                        return Err(Error::ServiceUnavailable(
                            "Database connection failed after retries"
                        ));
                    }
                    
                    warn!(
                        "Connection attempt {} failed: {}, retrying in {:?}",
                        attempt, e, delay
                    );
                    
                    tokio::time::sleep(delay).await;
                    
                    // Exponential backoff
                    delay = (delay * self.retry_config.exponential_base)
                        .min(self.retry_config.max_interval);
                }
            }
        }
    }
    
    fn should_retry(&self, start: &Instant, attempt: u32) -> bool {
        // During startup, retry indefinitely
        if self.is_startup.load(Ordering::Relaxed) {
            return true;
        }
        
        // Check elapsed time
        if let Some(max_elapsed) = self.retry_config.max_elapsed_time {
            if start.elapsed() > max_elapsed {
                return false;
            }
        }
        
        // After 30s, return 503 for operational requests
        if start.elapsed() > Duration::from_secs(30) {
            return false;
        }
        
        true
    }
}
```

### 4. Monitoring Connection Health

```rust
/// Metrics for connection pool monitoring
pub struct PoolMetrics {
    pub connections_created: Counter,
    pub connections_closed: Counter,
    pub connection_errors: Counter,
    pub acquire_time: Histogram,
    pub active_connections: Gauge,
    pub idle_connections: Gauge,
    pub waiting_requests: Gauge,
}

impl PoolMetrics {
    pub fn record_acquire(&self, duration: Duration) {
        self.acquire_time.record(duration.as_secs_f64());
    }
    
    pub fn record_error(&self, error_type: &str) {
        self.connection_errors
            .with_label_values(&[error_type])
            .inc();
    }
}

// Export metrics for Prometheus
metrics! {
    gauge!("db_pool_connections_active").set(pool.active() as f64);
    gauge!("db_pool_connections_idle").set(pool.idle() as f64);
    gauge!("db_pool_connections_waiting").set(pool.waiting() as f64);
    counter!("db_connection_retries_total", "reason" => "timeout").increment(1);
}
```

## Testing Connection Pools

### 1. Unit Tests

```rust
#[tokio::test]
async fn test_pool_limits() {
    let config = PoolConfig {
        min_connections: 2,
        max_connections: 5,
        ..Default::default()
    };
    
    let pool = ConnectionPool::new(config).await;
    
    // Acquire max connections
    let mut conns = Vec::new();
    for _ in 0..5 {
        conns.push(pool.acquire().await.unwrap());
    }
    
    // Next acquire should timeout
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        pool.acquire()
    ).await;
    
    assert!(result.is_err()); // Timeout
}
```

### 2. Load Tests

```rust
#[tokio::test]
async fn test_pool_under_load() {
    let pool = Arc::new(ConnectionPool::new(PoolConfig::default()).await);
    let mut handles = Vec::new();
    
    // Spawn many concurrent tasks
    for i in 0..100 {
        let pool = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let conn = pool.acquire().await.unwrap();
                // Simulate work
                tokio::time::sleep(Duration::from_millis(10)).await;
                // Connection returned on drop
            }
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Check pool health
    let stats = pool.stats();
    assert!(stats.avg_wait_time < Duration::from_secs(1));
}
```

## Common Pitfalls

### 1. Connection Leaks

```rust
// BAD - Connection never returned!
let conn = pool.acquire().await?;
let conn_ref = Box::leak(Box::new(conn));

// GOOD - Connection returned when dropped
{
    let conn = pool.acquire().await?;
    do_work(&conn).await?;
} // conn dropped and returned here
```

### 2. Pool Exhaustion

```rust
// BAD - Holding connections too long
let conn = pool.acquire().await?;
expensive_computation().await; // Connection idle!
conn.query("SELECT 1").await?;

// GOOD - Acquire only when needed
expensive_computation().await;
let conn = pool.acquire().await?;
conn.query("SELECT 1").await?;
```

### 3. Not Handling Acquire Timeouts

```rust
// BAD - Can wait forever
let conn = pool.acquire().await?;

// GOOD - Set reasonable timeout
let conn = tokio::time::timeout(
    Duration::from_secs(5),
    pool.acquire()
).await
.map_err(|_| Error::PoolTimeout)??;
```

## Monitoring and Debugging

### Pool Status Endpoint

```rust
async fn pool_status_handler(State(pool): State<Arc<ConnectionPool>>) -> Json<PoolStatus> {
    let stats = pool.stats();
    
    Json(PoolStatus {
        total_connections: stats.total,
        available_connections: stats.available,
        in_use_connections: stats.in_use,
        wait_queue_length: stats.waiting,
        avg_wait_time_ms: stats.avg_wait_time.as_millis() as u64,
        connections_created_total: stats.created_total,
        connections_closed_total: stats.closed_total,
    })
}
```

### Debug Logging

```rust
impl<T> ConnectionPool<T> {
    async fn acquire_with_logging(&self) -> Result<PooledConnection<T>, Error> {
        let start = Instant::now();
        
        tracing::debug!(
            available = %self.available.read().await.len(),
            in_use = %self.in_use.read().await.len(),
            "Acquiring connection"
        );
        
        let result = self.acquire().await;
        
        tracing::debug!(
            success = %result.is_ok(),
            duration = ?start.elapsed(),
            "Connection acquire complete"
        );
        
        result
    }
}
```

## Summary

Connection pooling is essential for database performance. Remember:

1. **Pre-create connections** - Avoid connection overhead per request
2. **Set reasonable limits** - Balance resources vs availability
3. **Monitor pool health** - Track metrics and adjust sizing
4. **Handle timeouts** - Don't wait forever for connections
5. **Return connections promptly** - Avoid pool exhaustion

A well-configured connection pool can improve your application's performance by 10-100x!
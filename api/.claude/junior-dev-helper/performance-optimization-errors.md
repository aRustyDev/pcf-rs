# Common Performance Errors and Optimization Guide

## Top 10 Performance Mistakes in GraphQL APIs

### 1. The Classic N+1 Query Problem

**Error**: Loading related data in a loop

```rust
// ❌ BAD: N+1 queries
impl Post {
    async fn author(&self, ctx: &Context<'_>) -> Result<User> {
        // This runs once per post!
        ctx.db.get_user(&self.author_id).await
    }
}

// If you have 100 posts, this makes 101 queries:
// 1 query for posts + 100 queries for authors
```

**Fix**: Use DataLoader
```rust
// ✅ GOOD: Batched loading
impl Post {
    async fn author(&self, ctx: &Context<'_>) -> Result<User> {
        ctx.data::<DataLoader<UserLoader>>()?
            .load_one(self.author_id.clone())
            .await
    }
}
// Now it's just 2 queries total!
```

**How to Detect**:
```rust
// Add query counting in tests
let query_count = Arc::new(AtomicUsize::new(0));
db.on_query(|_| query_count.fetch_add(1, Ordering::SeqCst));

// Execute GraphQL query
execute_query(query).await;

// Check query count
assert!(query_count.load(Ordering::SeqCst) < 5, "Too many queries!");
```

### 2. Unbounded Query Complexity

**Error**: Allowing infinitely nested queries

```graphql
# This can crash your server!
query Evil {
  users {
    posts {
      author {
        posts {
          author {
            posts {
              # ... continues forever
            }
          }
        }
      }
    }
  }
}
```

**Fix**: Implement query depth limiting
```rust
use async_graphql::*;

let schema = Schema::build(Query, Mutation, Subscription)
    .limit_depth(5)  // Max nesting depth
    .limit_complexity(1000)  // Max complexity score
    .finish();

// Or implement custom complexity calculation
#[Object]
impl Query {
    #[graphql(complexity = "2 + child_complexity")]
    async fn users(
        &self,
        #[graphql(default = 10)] first: i32,
    ) -> Vec<User> {
        // Complexity = 2 + (first * complexity of User fields)
    }
}
```

### 3. Missing Pagination

**Error**: Loading entire tables into memory

```rust
// ❌ BAD: Loads ALL users
async fn users(&self) -> Result<Vec<User>> {
    self.db.query("SELECT * FROM users").await
}
```

**Fix**: Always paginate collections
```rust
// ✅ GOOD: Cursor-based pagination
async fn users(
    &self,
    first: Option<i32>,
    after: Option<String>,
) -> Result<Connection<User>> {
    let limit = first.unwrap_or(20).min(100); // Cap at 100
    
    let query = if let Some(cursor) = after {
        format!(
            "SELECT * FROM users WHERE id > {} ORDER BY id LIMIT {}",
            cursor, limit + 1  // +1 to check hasNextPage
        )
    } else {
        format!("SELECT * FROM users ORDER BY id LIMIT {}", limit + 1)
    };
    
    let users = self.db.query(&query).await?;
    
    // Build connection with edges and pageInfo
    Connection::from_slice(&users, first, after)
}
```

### 4. Cache Key Collisions

**Error**: Sharing cache between users

```rust
// ❌ BAD: Same cache key for all users
let cache_key = format!("posts:page:{}", page);
let cached = cache.get(&cache_key).await;

// User A sees User B's private posts!
```

**Fix**: Include user context in cache keys
```rust
// ✅ GOOD: User-specific cache keys
let cache_key = CacheKey {
    query_hash: hash_query(query),
    user_id: auth.user_id.clone(),
    permissions: hash_permissions(&auth.permissions),
};

// Each user has isolated cache
```

### 5. Synchronous Blocking Operations

**Error**: Blocking the async runtime

```rust
// ❌ BAD: Blocks the thread
async fn process_image(&self, image: Vec<u8>) -> Result<String> {
    // This blocks the async runtime!
    let processed = std::fs::write("temp.jpg", &image)?;
    let result = image_magic::process("temp.jpg")?; // CPU intensive
    Ok(result)
}
```

**Fix**: Use tokio::task::spawn_blocking
```rust
// ✅ GOOD: Run in blocking thread pool
async fn process_image(&self, image: Vec<u8>) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        // Now it won't block other async tasks
        let processed = image_magic::process_bytes(&image)?;
        Ok(processed)
    })
    .await?
}
```

### 6. Inefficient Database Queries

**Error**: Not using indexes or doing work in application

```rust
// ❌ BAD: Filtering in application
let all_posts = db.query("SELECT * FROM posts").await?;
let user_posts: Vec<_> = all_posts
    .into_iter()
    .filter(|p| p.author_id == user_id)
    .collect();
```

**Fix**: Let database do the work
```rust
// ✅ GOOD: Filter in database with index
// First, create index: CREATE INDEX idx_posts_author ON posts(author_id);

let user_posts = db.query(
    "SELECT * FROM posts WHERE author_id = $1",
    &[&user_id]
).await?;
```

### 7. Connection Pool Starvation

**Error**: Not returning connections to pool

```rust
// ❌ BAD: Holding connection too long
let conn = pool.get().await?;
let user = query_user(&conn).await?;

// Do lots of other work while holding connection
let processed = expensive_computation(user).await; // 5 seconds!

// Connection unavailable to others for 5 seconds
update_user(&conn, processed).await?;
```

**Fix**: Minimize connection hold time
```rust
// ✅ GOOD: Get connection only when needed
let user = {
    let conn = pool.get().await?;
    query_user(&conn).await?
    // Connection returned to pool here
};

let processed = expensive_computation(user).await;

// Get fresh connection for update
let conn = pool.get().await?;
update_user(&conn, processed).await?;
```

### 8. No Request Timeouts

**Error**: Requests can hang forever

```rust
// ❌ BAD: No timeout
let result = external_api.fetch_data().await?;
// Could wait forever if API is down
```

**Fix**: Always set timeouts
```rust
// ✅ GOOD: Cascading timeouts
use tokio::time::timeout;

// HTTP timeout > GraphQL timeout > DB timeout
let result = timeout(
    Duration::from_secs(20),  // DB timeout
    db.query(query)
).await??;

// Or use timeout middleware
app.layer(TimeoutLayer::new(Duration::from_secs(30)));
```

### 9. Inefficient Serialization

**Error**: Serializing large objects repeatedly

```rust
// ❌ BAD: Serializing same data multiple times
impl User {
    async fn profile_json(&self) -> String {
        // This runs for EVERY user in a list!
        serde_json::to_string(&self.profile).unwrap()
    }
}
```

**Fix**: Cache serialized data or use references
```rust
// ✅ GOOD: Return reference, let GraphQL serialize once
impl User {
    async fn profile(&self) -> &Profile {
        &self.profile
    }
}

// Or cache serialized form
pub struct User {
    profile: Profile,
    #[serde(skip)]
    profile_json_cache: OnceCell<String>,
}
```

### 10. Memory Leaks from Unbounded Caches

**Error**: Caches growing forever

```rust
// ❌ BAD: Unbounded cache
static CACHE: Lazy<Mutex<HashMap<String, Value>>> = Lazy::new(Default::default);

// Just keeps growing!
CACHE.lock().unwrap().insert(key, value);
```

**Fix**: Use LRU cache with size limit
```rust
// ✅ GOOD: Bounded LRU cache
use lru::LruCache;

static CACHE: Lazy<Mutex<LruCache<String, Value>>> = Lazy::new(|| {
    Mutex::new(LruCache::new(NonZeroUsize::new(10_000).unwrap()))
});

// Automatically evicts least recently used
CACHE.lock().unwrap().put(key, value);
```

## Performance Debugging Techniques

### 1. Add Timing to Every Layer

```rust
use tracing::{info_span, Instrument};

// GraphQL layer
async fn resolve_users(&self) -> Result<Vec<User>> {
    async move {
        // ... resolution logic
    }
    .instrument(info_span!("graphql.resolve_users"))
    .await
}

// Service layer
async fn get_users(&self) -> Result<Vec<User>> {
    async move {
        // ... service logic
    }
    .instrument(info_span!("service.get_users"))
    .await
}

// Database layer
async fn query_users(&self) -> Result<Vec<User>> {
    async move {
        // ... database logic
    }
    .instrument(info_span!("db.query_users"))
    .await
}

// Now you can see where time is spent!
```

### 2. Database Query Analysis

```rust
// Log slow queries
pub struct InstrumentedDb {
    inner: Database,
}

impl InstrumentedDb {
    pub async fn query<T>(&self, sql: &str, params: &[&dyn ToSql]) -> Result<T> {
        let start = Instant::now();
        
        let result = self.inner.query(sql, params).await;
        
        let duration = start.elapsed();
        
        if duration > Duration::from_millis(100) {
            warn!(
                sql = %sql,
                duration_ms = duration.as_millis(),
                "Slow query detected"
            );
            
            // In development, explain the query
            #[cfg(debug_assertions)]
            {
                let plan = self.inner.explain(sql, params).await?;
                warn!("Query plan: {}", plan);
            }
        }
        
        result
    }
}
```

### 3. Memory Profiling

```rust
// Track memory allocations
use jemalloc_ctl::{stats, epoch};

pub async fn log_memory_stats() {
    // Update stats
    epoch::advance().unwrap();
    
    let allocated = stats::allocated::read().unwrap();
    let resident = stats::resident::read().unwrap();
    
    info!(
        allocated_mb = allocated / 1_000_000,
        resident_mb = resident / 1_000_000,
        "Memory stats"
    );
}

// Run periodically during load tests
tokio::spawn(async {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        log_memory_stats().await;
    }
});
```

## Optimization Strategies

### 1. Query Batching

```rust
// Batch multiple operations
pub struct BatchedOperations {
    pending: Arc<Mutex<Vec<PendingOp>>>,
    notify: Arc<Notify>,
}

impl BatchedOperations {
    pub async fn execute(&self, op: Operation) -> Result<Value> {
        let (tx, rx) = oneshot::channel();
        
        // Add to batch
        self.pending.lock().unwrap().push(PendingOp { op, tx });
        
        // Notify batcher
        self.notify.notify_one();
        
        // Wait for result
        rx.await?
    }
    
    async fn batch_processor(&self) {
        loop {
            // Wait for notification or timeout
            tokio::select! {
                _ = self.notify.notified() => {},
                _ = tokio::time::sleep(Duration::from_millis(10)) => {},
            }
            
            // Process batch
            let ops = self.pending.lock().unwrap().drain(..).collect::<Vec<_>>();
            
            if !ops.is_empty() {
                self.process_batch(ops).await;
            }
        }
    }
}
```

### 2. Parallel Loading

```rust
// Load independent data in parallel
async fn dashboard_data(&self) -> Result<Dashboard> {
    // ❌ BAD: Sequential loading (300ms total)
    let user = load_user().await?;      // 100ms
    let stats = load_stats().await?;    // 100ms  
    let notices = load_notices().await?; // 100ms
    
    // ✅ GOOD: Parallel loading (100ms total)
    let (user, stats, notices) = tokio::try_join!(
        load_user(),
        load_stats(),
        load_notices(),
    )?;
    
    Ok(Dashboard { user, stats, notices })
}
```

### 3. Preloading and Warming

```rust
// Preload common queries on startup
pub async fn warm_cache(cache: &Cache, db: &Database) {
    info!("Warming cache...");
    
    let common_queries = vec![
        ("user_list", "{ users(first: 20) { id name } }"),
        ("recent_posts", "{ posts(first: 10) { id title } }"),
    ];
    
    for (name, query) in common_queries {
        match execute_query(db, query).await {
            Ok(result) => {
                cache.set(query, &json!({}), "system", result).await;
                info!("Warmed cache for {}", name);
            }
            Err(e) => warn!("Failed to warm {}: {}", name, e),
        }
    }
}
```

## Performance Monitoring in Production

### Key Metrics to Track

```rust
// Response time percentiles
histogram!("graphql_request_duration_seconds",
    "operation_type" => op_type,
    "operation_name" => op_name,
).record(duration.as_secs_f64());

// Cache effectiveness
counter!("cache_hits_total", "cache" => cache_name).increment(1);
counter!("cache_misses_total", "cache" => cache_name).increment(1);

// Database performance
histogram!("db_query_duration_seconds",
    "query_type" => query_type,
).record(duration.as_secs_f64());

// Connection pool health
gauge!("db_pool_connections_active").set(pool.active() as f64);
gauge!("db_pool_connections_idle").set(pool.idle() as f64);

// N+1 detection
gauge!("graphql_queries_per_request",
    "operation_name" => op_name,
).set(query_count as f64);
```

### Alerting Rules

```yaml
# Prometheus alerts
groups:
  - name: performance
    rules:
      - alert: HighP99Latency
        expr: histogram_quantile(0.99, graphql_request_duration_seconds_bucket) > 0.2
        for: 5m
        annotations:
          summary: "P99 latency above 200ms"
          
      - alert: LowCacheHitRate
        expr: |
          rate(cache_hits_total[5m]) / 
          (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m])) < 0.5
        for: 10m
        annotations:
          summary: "Cache hit rate below 50%"
          
      - alert: PossibleNPlusOne
        expr: graphql_queries_per_request > 50
        for: 5m
        annotations:
          summary: "Possible N+1 query pattern detected"
```

## Summary

Performance optimization is an ongoing process:

1. **Measure first**: Don't optimize blindly
2. **Fix the biggest issues**: Use profiling to find them
3. **Test under load**: Development performance ≠ production
4. **Monitor continuously**: Performance can degrade over time
5. **Cache wisely**: With proper invalidation and bounds

Remember: The fastest query is the one you don't make!
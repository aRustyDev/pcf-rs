# Phase 6 Performance Integration Guide

## How the Pieces Fit Together

Phase 6 implements three major performance optimizations that work together to achieve sub-200ms P99 latency at 1000 RPS:

```
┌─────────────────────────────────────────────────────────────┐
│                    Incoming GraphQL Request                  │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                   Timeout Middleware (30s)                   │
│  - Sets deadline for entire request                         │
│  - Propagates timeout context through layers                │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                  Response Cache Check                        │
│  - Hash: Query + Variables + User ID                        │
│  - Cache hit? Return immediately (< 1ms)                    │
│  - Cache miss? Continue to execution                        │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              GraphQL Execution (25s timeout)                 │
│  ┌─────────────────────────────────────────────────┐       │
│  │               Field Resolution                    │       │
│  │  - DataLoader prevents N+1 queries               │       │
│  │  - Batches multiple ID lookups                   │       │
│  │  - Request-scoped caching                        │       │
│  └─────────────────────────────────────────────────┘       │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                Database Queries (20s timeout)                │
│  - Connection pool with retry logic                          │
│  - Batched queries from DataLoader                          │
│  - Exponential backoff on failures                          │
└─────────────────────────────────────────────────────────────┘
```

## Complete Integration Example

Here's a real example showing all optimizations working together:

```rust
use std::time::Duration;
use async_graphql::*;
use crate::performance::{DataLoader, ResponseCache, TimeoutContext};

/// GraphQL Context with all performance optimizations
pub struct GraphQLContext {
    // DataLoaders for N+1 prevention
    pub user_loader: DataLoader<UserLoader>,
    pub note_loader: DataLoader<NoteLoader>,
    pub tag_loader: DataLoader<TagLoader>,
    
    // Response cache
    pub response_cache: Arc<ResponseCache>,
    
    // Timeout context from middleware
    pub timeout_context: TimeoutContext,
    
    // Database with connection pooling
    pub db: Arc<HealthAwarePool>,
}

/// Root Query with all optimizations
pub struct Query;

#[Object]
impl Query {
    /// Get user's notes - optimized with all Phase 6 features
    async fn user_notes(
        &self,
        ctx: &Context<'_>,
        user_id: String,
        first: Option<i32>,
    ) -> Result<Vec<Note>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // 1. Check timeout budget
        if context.timeout_context.remaining() < Duration::from_secs(1) {
            return Err(Error::new("Insufficient time remaining"));
        }
        
        // 2. Try response cache first
        let cache_key = format!("user_notes:{}:{}", user_id, first.unwrap_or(20));
        if let Some(cached) = context.response_cache.get(&cache_key, &user_id).await {
            return Ok(cached);
        }
        
        // 3. Load with connection pool (includes retry logic)
        let notes = context.db
            .query_with_timeout(
                "SELECT * FROM notes WHERE user_id = $1 LIMIT $2",
                &[&user_id, &first.unwrap_or(20)],
                context.timeout_context.child_budget("database"),
            )
            .await?;
        
        // 4. Pre-load all authors to prevent N+1
        let author_ids: Vec<String> = notes.iter()
            .map(|n| n.author_id.clone())
            .collect();
        
        // DataLoader batches these into a single query
        context.user_loader.load_many(&author_ids).await?;
        
        // 5. Pre-load all tags
        let note_ids: Vec<String> = notes.iter()
            .map(|n| n.id.clone())
            .collect();
        
        // This also gets batched
        context.tag_loader.load_many_for_notes(&note_ids).await?;
        
        // 6. Cache the result
        context.response_cache
            .set(&cache_key, &user_id, &notes, Duration::from_secs(300))
            .await;
        
        Ok(notes)
    }
}

/// Note type with optimized resolvers
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub created_at: DateTime<Utc>,
}

#[Object]
impl Note {
    /// Author resolver using DataLoader (no N+1!)
    async fn author(&self, ctx: &Context<'_>) -> Result<User> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // DataLoader ensures this is batched with other author lookups
        context.user_loader
            .load_one(self.author_id.clone())
            .await
            .map(|arc| (*arc).clone())
    }
    
    /// Tags resolver using DataLoader
    async fn tags(&self, ctx: &Context<'_>) -> Result<Vec<Tag>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Also batched automatically
        context.tag_loader
            .load_for_note(self.id.clone())
            .await
    }
}

/// Middleware setup combining all optimizations
pub fn create_optimized_app() -> Router {
    let schema = create_schema();
    
    Router::new()
        .route("/graphql", post(graphql_handler))
        // Timeout middleware (30s for HTTP)
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        // Metrics for monitoring
        .layer(MetricsLayer::new())
        // Connection pool health checks
        .layer(HealthCheckLayer::new())
}

/// Request handler with context setup
async fn graphql_handler(
    State(schema): State<Schema<Query, Mutation, Subscription>>,
    timeout_ctx: Extension<TimeoutContext>,
    headers: HeaderMap,
    Json(request): Json<GraphQLRequest>,
) -> Result<Json<GraphQLResponse>> {
    // Create context with all optimizations
    let context = GraphQLContext {
        user_loader: DataLoader::new(UserLoader::new(db.clone())),
        note_loader: DataLoader::new(NoteLoader::new(db.clone())),
        tag_loader: DataLoader::new(TagLoader::new(db.clone())),
        response_cache: cache.clone(),
        timeout_context: timeout_ctx.0,
        db: db.clone(),
    };
    
    // Execute with timeout
    let response = tokio::time::timeout(
        Duration::from_secs(25), // GraphQL timeout < HTTP timeout
        schema.execute(request.data(context))
    )
    .await
    .map_err(|_| Error::new("Query execution timeout"))?;
    
    // Clear DataLoader caches for next request
    context.user_loader.clear_cache().await;
    context.note_loader.clear_cache().await;
    context.tag_loader.clear_cache().await;
    
    Ok(Json(response))
}
```

## Performance Testing Your Integration

Test that all optimizations work together:

```bash
#!/bin/bash
# Test N+1 prevention
echo "Testing N+1 prevention..."
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ 
      users(first: 100) { 
        id 
        name 
        notes { 
          id 
          title 
          author { name } 
          tags { name } 
        } 
      } 
    }"
  }' \
  -w "\nTime: %{time_total}s\n"

# Monitor metrics during request
curl -s http://localhost:8080/metrics | grep -E "dataloader_batch|cache_hit|db_query_count"

# Test cache effectiveness
echo -e "\nTesting cache effectiveness..."
for i in {1..5}; do
  echo "Request $i:"
  curl -X POST http://localhost:8080/graphql \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer test-token" \
    -d '{"query":"{ user(id: \"1\") { notes { title } } }"}' \
    -w "Time: %{time_total}s\n" \
    -o /dev/null -s
done

# Test timeout cascade
echo -e "\nTesting timeout cascade..."
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ slowQuery(delay: 35) }"}' \
  -m 40 \
  -w "\nHTTP Code: %{http_code}, Time: %{time_total}s\n"
```

## Common Integration Issues

### 1. DataLoader Cache Not Cleared Between Requests

**Problem**: Users see each other's data
```rust
// ❌ BAD: Reusing DataLoader across requests
static USER_LOADER: Lazy<DataLoader<UserLoader>> = Lazy::new(|| {
    DataLoader::new(UserLoader::new())
});
```

**Solution**: Create fresh DataLoader per request
```rust
// ✅ GOOD: Fresh DataLoader per request
let context = GraphQLContext {
    user_loader: DataLoader::new(UserLoader::new(db.clone())),
    // ... other fields
};
```

### 2. Cache Keys Don't Include User Context

**Problem**: Users see cached data from other users
```rust
// ❌ BAD: Cache key without user isolation
let cache_key = format!("query:{}", query_hash);
```

**Solution**: Always include user ID in cache keys
```rust
// ✅ GOOD: User-specific cache keys
let cache_key = format!("query:{}:user:{}", query_hash, user_id);
```

### 3. Timeout Context Not Propagated

**Problem**: Database queries don't respect request timeout
```rust
// ❌ BAD: Ignoring timeout context
let result = db.query("SELECT ...").await?;
```

**Solution**: Pass timeout context to all operations
```rust
// ✅ GOOD: Respecting timeout hierarchy
let result = db.query_with_timeout(
    "SELECT ...",
    params,
    ctx.timeout_context.child_budget("database")
).await?;
```

### 4. Metrics Cardinality Explosion

**Problem**: Too many unique label combinations
```rust
// ❌ BAD: User ID in metrics
histogram!("query_duration", "user_id" => user_id).record(duration);
```

**Solution**: Use bounded labels
```rust
// ✅ GOOD: Bounded cardinality
histogram!("query_duration", 
    "operation_type" => op_type,
    "complexity_bucket" => match complexity {
        0..=10 => "simple",
        11..=50 => "moderate", 
        _ => "complex",
    }
).record(duration);
```

## Debugging Performance Issues

### Check DataLoader Effectiveness
```sql
-- Enable query logging
SET log_statement = 'all';

-- Look for patterns like:
-- SELECT * FROM users WHERE id = 1;
-- SELECT * FROM users WHERE id = 2;
-- SELECT * FROM users WHERE id = 3;
-- Should be: SELECT * FROM users WHERE id IN (1, 2, 3);
```

### Monitor Cache Hit Rates
```bash
# Cache metrics
curl -s http://localhost:8080/metrics | grep cache_ | grep -E "hit|miss|ratio"

# Calculate hit rate
hits=$(curl -s http://localhost:8080/metrics | grep response_cache_hits_total | awk '{print $2}')
misses=$(curl -s http://localhost:8080/metrics | grep response_cache_misses_total | awk '{print $2}')
echo "Hit rate: $(echo "scale=2; $hits / ($hits + $misses) * 100" | bc)%"
```

### Profile Under Load
```bash
# Generate flame graph during load test
cargo flamegraph --bin pcf-api &
PID=$!
sleep 5
./scripts/load-test.sh 500 60
kill $PID

# Analyze slow queries
RUST_LOG=trace cargo run 2>&1 | grep "SLOW QUERY" | sort | uniq -c
```

## Performance Optimization Checklist

Before considering Phase 6 complete, verify:

- [ ] **DataLoader Integration**
  - [ ] All relationship fields use DataLoader
  - [ ] Batch metrics show efficiency > 5x
  - [ ] No N+1 patterns in query logs
  - [ ] Per-request cache clearing works

- [ ] **Response Caching**
  - [ ] Cache keys include user context
  - [ ] Hit rate > 50% after warmup
  - [ ] TTLs appropriate for data freshness
  - [ ] Invalidation works correctly

- [ ] **Timeout Management**
  - [ ] HTTP timeout > GraphQL timeout > DB timeout
  - [ ] All operations respect timeout context
  - [ ] No hanging requests under load
  - [ ] Clear timeout error messages

- [ ] **Connection Pool Health**
  - [ ] Retry logic follows exponential backoff
  - [ ] Pool size appropriate for load
  - [ ] No connection exhaustion under load
  - [ ] Health checks report accurately

- [ ] **Metrics and Monitoring**
  - [ ] Cardinality < 1000 per metric
  - [ ] All key operations instrumented
  - [ ] Dashboards show real-time performance
  - [ ] Alerts configured for SLO violations

## Next Steps

1. Run the complete integration test suite
2. Perform load testing at 1000 RPS
3. Profile and optimize any bottlenecks
4. Document production tuning parameters
5. Set up monitoring dashboards

Remember: Performance optimization is iterative. Measure, improve, repeat!
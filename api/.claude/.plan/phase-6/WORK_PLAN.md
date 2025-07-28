# Phase 6: Performance Optimization - Work Plan

## Prerequisites

Before starting Phase 6, ensure you have:
- **Completed Phases 1-5**: Server foundation, database layer, GraphQL implementation, authorization, and observability operational
- **DataLoader Knowledge**: Understanding of the N+1 query problem and batching strategies
- **Caching Experience**: Familiarity with multi-level caching, TTL strategies, and cache invalidation
- **Performance Testing**: Experience with load testing tools and performance profiling
- **Async Rust Proficiency**: Deep understanding of async patterns and performance implications

## Quick Reference - Essential Resources

### Example Files
All example files are located in `/api/.claude/.spec/examples/`:
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Comprehensive test examples following TDD
- **[Cache Strategies](../../.spec/examples/cache-strategies.rs)** - LRU cache and multi-level caching patterns
- **[Connection Pool](../../.spec/examples/connection-pool.rs)** - Connection pooling with health checks
- **[DataLoader Patterns](../../.spec/examples/dataloader-patterns.rs)** - N+1 prevention patterns (to be created)
- **[Performance Testing](../../.spec/examples/performance-testing.rs)** - Load testing patterns (to be created)

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[SPEC.md](../../SPEC.md)** - Performance requirements (lines 64-73)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 6 objectives (lines 156-183)
- **[graphql-schema.md](../../.spec/graphql-schema.md)** - Query complexity limits

### Junior Developer Resources
Comprehensive guides in `/api/.claude/junior-dev-helper/`:
- **[DataLoader N+1 Tutorial](../../junior-dev-helper/dataloader-n1-tutorial.md)** - Understanding and solving N+1 queries
- **[DataLoader Guide](../../junior-dev-helper/dataloader-guide.md)** - Complete DataLoader implementation patterns
- **[Response Caching Guide](../../junior-dev-helper/response-caching-guide.md)** - Response caching with user isolation
- **[Performance Testing Tutorial](../../junior-dev-helper/performance-testing-tutorial.md)** - Load testing and benchmarking
- **[Performance Optimization Errors](../../junior-dev-helper/performance-optimization-errors.md)** - Common mistakes and fixes
- **[Timeout Management Guide](../../junior-dev-helper/timeout-management-guide.md)** - Hierarchical timeout implementation
- **[Cardinality Control Guide](../../junior-dev-helper/cardinality-control-guide.md)** - Managing metric cardinality limits
- **[Connection Pool Guide](../../junior-dev-helper/connection-pool-guide.md)** - Database connection optimization
- **[Retry Patterns Guide](../../junior-dev-helper/retry-patterns-guide.md)** - Resilient retry strategies

### Quick Links
- **Verification Script**: `scripts/verify-phase-6.sh` (create using examples from Phase 1-5)
- **Load Test Suite**: `scripts/load-test.sh` (adapt from performance-testing-tutorial.md)
- **Performance Profile**: `scripts/profile.sh` (use standard Rust profiling tools)

**If scripts don't exist**: 
1. Create based on similar scripts from previous phases
2. Use manual commands documented in the guides
3. Document what you did in `api/.claude/.reviews/phase-6-tooling.md`

## Overview
This work plan implements comprehensive performance optimizations focusing on N+1 query prevention, intelligent caching, and proper timeout management. The goal is to achieve sub-200ms P99 latency at 1000 RPS. Each checkpoint represents a natural boundary for review.

## Build and Test Commands

Continue using `just` as the command runner:
- `just test` - Run all tests including performance tests
- `just test-perf` - Run only performance-related tests
- `just bench` - Run benchmarks
- `just load-test` - Run load testing suite
- `just profile` - Generate flame graphs

Always use these commands instead of direct cargo commands to ensure consistency.

## IMPORTANT: Review Process

**This plan includes 4 mandatory review checkpoints where work MUST stop for external review.**

At each checkpoint:
1. **STOP work on the current section** and commit your code
2. **Write any questions** you have to `api/.claude/.reviews/checkpoint-X-questions.md` (where X is the checkpoint number)
3. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
4. **Wait for approval** before continuing to next section
   - **If no response within 24 hours**: Document your plan in `api/.claude/.reviews/checkpoint-X-self-review.md` and proceed cautiously
   - **If blocked over 48 hours**: Escalate in `api/.claude/.reviews/phase-6-blockers.md` and continue with non-dependent sections
5. **If you wrote questions**, wait for answers to be provided in the same questions file before proceeding
   - **If no answers within 24 hours**: Document your assumptions and proceed with best judgment

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Continue following TDD practices from previous phases:
1. **Write tests FIRST** - Before any implementation
   - **Exception**: Exploratory spikes allowed if followed immediately by tests
   - **Document** any prototype code that helped inform the test design
2. **Run tests to see them FAIL** - Confirms test is valid
3. **Write minimal code to make tests PASS** - No more than necessary
4. **REFACTOR** - Clean up while keeping tests green
5. **Document as you go** - Add rustdoc comments and inline explanations

**Note**: If you need to explore an approach before writing tests, timebox to 30 minutes and immediately backfill tests for any code you keep.

## Done Criteria Checklist
- [ ] No N+1 queries detected in tests
- [ ] P99 response times under 200ms (at 1000 RPS load on 4-core test environment)
- [ ] Timeouts cascade properly without hanging
- [ ] Cache hit rate > 50% for common queries (measured after 5-minute warmup)
- [ ] Load tests pass at 1000 RPS (99% success rate over 5 minutes)
- [ ] Memory usage stable under load (less than 10% growth over 1 hour)
- [ ] CPU usage scales linearly (R² > 0.9)
- [ ] All code has corresponding tests written first (spikes immediately backfilled)

**Note**: Document any criteria not met with explanation and remediation plan in `api/.claude/.reviews/phase-6-exceptions.md`

## Work Breakdown with Review Checkpoints

### 6.1 DataLoader Implementation (3-4 work units)

**Work Unit Definition**: 1 work unit ≈ 4-6 hours of focused development time

**Work Unit Context:**
- **Complexity**: High - Complex async batching and caching
- **Scope**: ~1000 lines across 8-10 files
- **Time Estimate**: 12-24 hours total
- **Key Components**: 
  - DataLoader trait and implementation (~300 lines)
  - Batch loading logic (~200 lines)
  - Per-request cache integration (~150 lines)
  - GraphQL context setup (~100 lines)
  - N+1 detection in tests (~150 lines)
  - Resolver modifications (~100 lines)
- **Patterns**: Async batching, request-scoped caching, lazy loading

#### Task 6.1.1: Write DataLoader Tests First

**💡 Junior Dev Tip**: Start by reading the [DataLoader N+1 Tutorial](../../junior-dev-helper/dataloader-n1-tutorial.md) to understand why we need DataLoader and how it prevents N+1 queries.

Create `src/performance/dataloader.rs` with comprehensive test module:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{Context, dataloader::DataLoader};
    
    #[tokio::test]
    async fn test_batching_prevents_n_plus_one() {
        let db = MockDatabase::new();
        let loader = UserLoader::new(db.clone());
        
        // Track database calls
        let call_count = Arc::new(AtomicUsize::new(0));
        db.set_call_counter(call_count.clone());
        
        // Load multiple users
        let user_ids = vec!["1", "2", "3", "4", "5"];
        let futures: Vec<_> = user_ids.iter()
            .map(|id| loader.load_one(id.to_string()))
            .collect();
        
        let users = futures::future::join_all(futures).await;
        
        // Should batch into single query
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
        assert_eq!(users.len(), 5);
    }
    
    #[tokio::test]
    async fn test_request_scoped_caching() {
        let loader = create_test_loader();
        
        // Load same ID multiple times
        let user1 = loader.load_one("123").await?;
        let user2 = loader.load_one("123").await?;
        
        // Should return same instance (cached)
        assert!(Arc::ptr_eq(&user1, &user2));
    }
    
    #[tokio::test]
    async fn test_batch_size_limits() {
        let loader = UserLoader::with_config(BatchConfig {
            max_batch_size: 100,
            batch_delay: Duration::from_millis(10),
        });
        
        // Load more than batch size
        let ids: Vec<_> = (0..250).map(|i| i.to_string()).collect();
        let results = loader.load_many(&ids).await?;
        
        // Should split into 3 batches
        assert_eq!(loader.batch_count(), 3);
    }
}
```

#### Task 6.1.2: Implement DataLoader Trait
Create the core DataLoader abstraction:
```rust
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Loader: Send + Sync + 'static {
    type Key: Send + Sync + Clone + Eq + Hash;
    type Value: Send + Sync + Clone;
    type Error: Send + Sync;
    
    /// Load multiple keys in a single batch
    async fn load_batch(
        &self,
        keys: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, Self::Error>;
}

pub struct DataLoader<L: Loader> {
    loader: Arc<L>,
    cache: Arc<RwLock<HashMap<L::Key, Arc<L::Value>>>>,
    pending: Arc<RwLock<HashMap<L::Key, Vec<oneshot::Sender<Arc<L::Value>>>>>>,
    config: BatchConfig,
}
```

#### Task 6.1.3: Batch Aggregation Logic
Implement intelligent batching with delays:
```rust
impl<L: Loader> DataLoader<L> {
    pub async fn load_one(&self, key: L::Key) -> Result<Arc<L::Value>, L::Error> {
        // Check cache first
        if let Some(value) = self.get_cached(&key).await {
            return Ok(value);
        }
        
        // Add to pending batch
        let receiver = self.add_to_batch(key.clone()).await;
        
        // Trigger batch if needed
        self.maybe_flush_batch().await;
        
        // Wait for result
        receiver.await
            .map_err(|_| /* handle cancelled */)
    }
    
    async fn flush_batch(&self) {
        let pending = self.pending.write().await.drain().collect();
        
        // Split into manageable batches
        for chunk in pending.chunks(self.config.max_batch_size) {
            self.execute_batch(chunk).await;
        }
    }
}
```

#### Task 6.1.4: GraphQL Context Integration
Integrate DataLoader with async-graphql context:
```rust
pub struct GraphQLContext {
    pub db: Arc<DatabaseService>,
    pub user_loader: DataLoader<UserLoader>,
    pub note_loader: DataLoader<NoteLoader>,
    pub auth_loader: DataLoader<AuthLoader>,
}

impl GraphQLContext {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        Self {
            user_loader: DataLoader::new(UserLoader::new(db.clone())),
            note_loader: DataLoader::new(NoteLoader::new(db.clone())),
            auth_loader: DataLoader::new(AuthLoader::new(db.clone())),
            db,
        }
    }
}

// In resolvers
async fn notes(&self, ctx: &Context<'_>) -> Result<Vec<Note>> {
    let context = ctx.data::<GraphQLContext>()?;
    let notes = self.db.get_notes().await?;
    
    // Pre-load all authors to prevent N+1
    let author_ids: Vec<_> = notes.iter()
        .map(|n| n.author_id.clone())
        .collect();
    
    context.user_loader.load_many(&author_ids).await?;
    
    Ok(notes)
}
```

### 🛑 CHECKPOINT 1: DataLoader Implementation Review
**Deliverables**:
- DataLoader trait and core implementation
- Batch aggregation with configurable delays
- Request-scoped caching working
- Integration with GraphQL context
- N+1 query prevention verified

**Verification**:
- Run `scripts/verify-phase-6.sh checkpoint1` if available
- Otherwise, manually verify each deliverable and document in review request
- Include performance metrics from your local testing

---

### 6.2 Response Caching (2-3 work units)

**Work Unit Definition**: 1 work unit ≈ 4-6 hours of focused development time

**Work Unit Context:**
- **Complexity**: Medium - Cache key generation and invalidation
- **Scope**: ~600 lines across 4-5 files
- **Key Components**: 
  - Response cache layer (~200 lines)
  - Cache key strategies (~150 lines)
  - TTL configuration (~100 lines)
  - Invalidation logic (~100 lines)
  - Cache warming (~50 lines)
- **Patterns**: Query fingerprinting, user isolation, smart invalidation

#### Task 6.2.1: Write Response Cache Tests

**🔐 Security Alert**: Before implementing caching, review the [Caching Strategies Guide](../../junior-dev-helper/caching-strategies-guide.md) to understand cache isolation and security requirements. Never share cached data between users!

Create comprehensive caching tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_query_result_caching() {
        let cache = ResponseCache::new(CacheConfig {
            max_size: 1000,
            default_ttl: Duration::from_secs(300),
        });
        
        let query = "{ users { id name } }";
        let variables = json!({});
        let result = json!({ "users": [...] });
        
        // Cache miss
        assert!(cache.get(query, &variables, "user123").await.is_none());
        
        // Store result
        cache.set(query, &variables, "user123", &result).await;
        
        // Cache hit
        let cached = cache.get(query, &variables, "user123").await;
        assert_eq!(cached, Some(result));
    }
    
    #[tokio::test]
    async fn test_user_isolation() {
        let cache = ResponseCache::new(Default::default());
        
        // Same query, different users
        cache.set(QUERY, &VARS, "user1", &json!({"data": 1})).await;
        cache.set(QUERY, &VARS, "user2", &json!({"data": 2})).await;
        
        // Each user sees their own data
        assert_eq!(cache.get(QUERY, &VARS, "user1").await.unwrap()["data"], 1);
        assert_eq!(cache.get(QUERY, &VARS, "user2").await.unwrap()["data"], 2);
    }
}
```

#### Task 6.2.2: Implement Response Cache
Create intelligent response caching:
```rust
pub struct ResponseCache {
    cache: Arc<LruCache<CacheKey, CachedResponse>>,
    config: CacheConfig,
    metrics: CacheMetrics,
}

#[derive(Hash, Eq, PartialEq)]
struct CacheKey {
    query_hash: u64,
    variables_hash: u64,
    user_id: String,
}

impl ResponseCache {
    pub async fn get(
        &self,
        query: &str,
        variables: &Value,
        user_id: &str,
    ) -> Option<Value> {
        let key = self.generate_key(query, variables, user_id);
        
        if let Some(cached) = self.cache.get(&key).await {
            self.metrics.record_hit();
            Some(cached.data.clone())
        } else {
            self.metrics.record_miss();
            None
        }
    }
    
    fn generate_key(&self, query: &str, variables: &Value, user_id: &str) -> CacheKey {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        
        // Normalize query (remove whitespace, comments)
        let normalized = normalize_graphql_query(query);
        normalized.hash(&mut hasher);
        let query_hash = hasher.finish();
        
        // Hash variables
        hasher = DefaultHasher::new();
        variables.to_string().hash(&mut hasher);
        let variables_hash = hasher.finish();
        
        CacheKey {
            query_hash,
            variables_hash,
            user_id: user_id.to_string(),
        }
    }
}
```

#### Task 6.2.3: Cache Invalidation Strategy
Implement smart cache invalidation:
```rust
pub struct CacheInvalidator {
    patterns: Vec<InvalidationPattern>,
}

impl CacheInvalidator {
    pub async fn invalidate_for_mutation(
        &self,
        mutation_type: &str,
        affected_types: &[&str],
    ) {
        match mutation_type {
            "createNote" | "updateNote" | "deleteNote" => {
                // Invalidate all queries containing "notes"
                self.invalidate_pattern(r"\bnotes\b").await;
                
                // Invalidate specific note queries
                self.invalidate_pattern(r"\bnote\s*\(").await;
            }
            "updateUser" => {
                // Only invalidate affected user
                self.invalidate_user_queries(affected_user_id).await;
            }
            _ => {
                // Conservative: invalidate all for unknown mutations
                self.invalidate_all().await;
            }
        }
    }
}
```

### 🛑 CHECKPOINT 2: Response Caching Review
**Deliverables**:
- Response cache with user isolation
- Smart cache key generation
- TTL-based expiration
- Intelligent invalidation
- Cache metrics tracking

**Verification**:
- Test cache isolation between users
- Verify hit rates meet targets
- Check memory usage is bounded

---

### 6.3 Timeout Hierarchy (2-3 work units)

**Work Unit Definition**: 1 work unit ≈ 4-6 hours of focused development time

**Work Unit Context:**
- **Complexity**: Medium - Cascading timeout management
- **Scope**: ~500 lines across 4-5 files
- **Key Components**: 
  - Timeout middleware (~150 lines)
  - Cascading timeout context (~150 lines)
  - Timeout configuration (~50 lines)
  - Error propagation (~100 lines)
  - Graceful degradation (~50 lines)
- **Patterns**: Context deadlines, timeout budgets, graceful cancellation

#### Task 6.3.1: Write Timeout Tests

**⏱️ Timing is Critical**: Read the [Timeout Management Guide](../../junior-dev-helper/timeout-management-guide.md) to understand why proper timeout hierarchy prevents hanging requests and cascading failures.

Create timeout behavior tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_timeout_cascade() {
        // HTTP: 30s > GraphQL: 25s > DB: 20s
        let app = create_test_app();
        
        // Simulate slow query
        let result = app.graphql_request_with_timeout(
            "{ slowQuery }",
            Duration::from_secs(35),
        ).await;
        
        // Should timeout at HTTP layer (30s)
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().status(),
            StatusCode::REQUEST_TIMEOUT
        );
    }
    
    #[tokio::test]
    async fn test_remaining_budget_propagation() {
        let ctx = TimeoutContext::new(Duration::from_secs(10));
        
        // Simulate time passing
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Check remaining budget
        let remaining = ctx.remaining_budget();
        assert!(remaining < Duration::from_secs(8));
        assert!(remaining > Duration::from_secs(6));
    }
}
```

#### Task 6.3.2: Implement Timeout Middleware
Create hierarchical timeout management:
```rust
pub struct TimeoutMiddleware {
    http_timeout: Duration,
    graphql_timeout: Duration,
    database_timeout: Duration,
}

impl TimeoutMiddleware {
    pub async fn handle<B>(
        &self,
        req: Request<B>,
        next: Next<B>,
    ) -> Result<Response, Error> {
        // Create timeout context
        let deadline = Instant::now() + self.http_timeout;
        let ctx = TimeoutContext::new(deadline);
        
        // Add to request extensions
        req.extensions_mut().insert(ctx.clone());
        
        // Execute with timeout
        tokio::time::timeout(
            self.http_timeout,
            next.run(req)
        )
        .await
        .map_err(|_| Error::timeout("Request timeout"))?
    }
}

#[derive(Clone)]
pub struct TimeoutContext {
    deadline: Instant,
    budgets: Arc<RwLock<HashMap<String, Duration>>>,
}

impl TimeoutContext {
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
    
    pub fn child_budget(&self, name: &str) -> Duration {
        let remaining = self.remaining();
        let buffer = Duration::from_millis(500); // Safety buffer
        
        match name {
            "graphql" => remaining.saturating_sub(buffer),
            "database" => remaining.saturating_sub(Duration::from_secs(5)),
            _ => remaining.saturating_sub(buffer),
        }
    }
}
```

#### Task 6.3.3: Database Timeout Integration
Apply timeouts to database operations:
```rust
impl DatabaseService {
    pub async fn query_with_timeout<T>(
        &self,
        ctx: &TimeoutContext,
        query: Query,
    ) -> Result<T, Error> {
        let budget = ctx.child_budget("database");
        
        // Ensure minimum viable timeout
        let timeout = budget.max(Duration::from_secs(1));
        
        let result = tokio::time::timeout(
            timeout,
            self.execute_query(query)
        ).await;
        
        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Log timeout with context
                tracing::warn!(
                    remaining_budget = ?ctx.remaining(),
                    query = ?query,
                    "Database query timeout"
                );
                
                Err(Error::timeout("Database query timeout"))
            }
        }
    }
}
```

### 🛑 CHECKPOINT 3: Timeout Implementation Review
**Deliverables**:
- Hierarchical timeout structure
- Proper timeout propagation
- Graceful degradation
- No hanging requests
- Clear timeout errors

**Verification**:
- Test cascade behavior under load
- Verify no resource leaks on timeout
- Check error messages are helpful

---

### 6.4 Performance Testing & Optimization (2-3 work units)

**Work Unit Definition**: 1 work unit ≈ 4-6 hours of focused development time

**Work Unit Context:**
- **Complexity**: High - Load testing and profiling
- **Scope**: ~700 lines across 5-6 files
- **Key Components**: 
  - Load test scenarios (~200 lines)
  - Performance benchmarks (~150 lines)
  - Profiling helpers (~100 lines)
  - Optimization implementations (~150 lines)
  - Performance monitoring (~100 lines)
- **Patterns**: Realistic load patterns, profiling integration, bottleneck identification

#### Task 6.4.1: Create Load Test Suite

**📊 Performance Testing**: Study the [Performance Testing Tutorial](../../junior-dev-helper/performance-testing-tutorial.md) to learn how to create realistic load tests. Also check [Performance Optimization Errors](../../junior-dev-helper/performance-optimization-errors.md) for common pitfalls.

Build comprehensive load testing:
```rust
#[cfg(test)]
mod load_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sustained_load_1000_rps() {
        let app = setup_test_app().await;
        
        // Configure load pattern
        let config = LoadTestConfig {
            target_rps: 1000,
            duration: Duration::from_secs(60),
            ramp_up: Duration::from_secs(10),
            connections: 100,
        };
        
        let results = run_load_test(app, config).await;
        
        // Verify SLOs
        assert!(results.success_rate > 0.99);
        assert!(results.p99_latency < Duration::from_millis(200));
        assert_eq!(results.errors_by_type.get("timeout"), None);
    }
    
    #[test]
    fn benchmark_query_performance() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        criterion::bench_function("graphql_query", |b| {
            b.iter(|| {
                rt.block_on(async {
                    app.graphql_query("{ users { id name notes { title } } }")
                        .await
                })
            })
        });
    }
}
```

**Create Load Test Scripts**:

Create `scripts/load-test.sh` with this content:
```bash
#!/bin/bash
# Phase 6 Load Testing Script

RPS=${1:-1000}
DURATION=${2:-300}
ENDPOINT=${3:-"http://localhost:8080/graphql"}

echo "Starting load test: ${RPS} RPS for ${DURATION}s"

# Using vegeta (install: go install github.com/tsenart/vegeta@latest)
cat > queries.jsonl << EOF
{"query": "{ users(first: 20) { id name } }", "variables": {}}
{"query": "{ notes(first: 10) { id title author { name } } }", "variables": {}}
{"query": "{ user(id: \"1\") { notes { id title tags { name } } } }", "variables": {}}
EOF

# Generate load
vegeta attack -format=json \
  -rate=${RPS} \
  -duration=${DURATION}s \
  -targets=queries.jsonl \
  -header="Content-Type: application/json" \
  -output=results.bin

# Analyze results
vegeta report -type=text results.bin
vegeta plot results.bin > results.html

# Check if we met SLOs
P99=$(vegeta report -type=json results.bin | jq '.latencies.p99 / 1000000')
SUCCESS_RATE=$(vegeta report -type=json results.bin | jq '.success')

if (( $(echo "$P99 < 200" | bc -l) )) && (( $(echo "$SUCCESS_RATE > 0.99" | bc -l) )); then
    echo "✅ PASSED: P99=${P99}ms, Success=${SUCCESS_RATE}"
    exit 0
else
    echo "❌ FAILED: P99=${P99}ms, Success=${SUCCESS_RATE}"
    exit 1
fi
```

Alternative using k6 (install: https://k6.io/docs/getting-started/installation):
```javascript
// scripts/k6-load-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
    stages: [
        { duration: '30s', target: 100 },  // Ramp up
        { duration: '5m', target: 1000 },  // Stay at 1000 RPS
        { duration: '30s', target: 0 },    // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(99)<200'], // P99 under 200ms
        http_req_failed: ['rate<0.01'],   // Error rate under 1%
    },
};

const queries = [
    {
        name: 'userList',
        query: '{ users(first: 20) { id name } }',
        weight: 0.3,
    },
    {
        name: 'noteDetail', 
        query: '{ note(id: "1") { id title author { name } } }',
        weight: 0.4,
    },
    {
        name: 'userNotes',
        query: '{ user(id: "1") { notes { id title } } }',
        weight: 0.3,
    },
];

export default function() {
    // Select query based on weight
    const query = selectWeighted(queries);
    
    const payload = JSON.stringify({
        query: query.query,
        variables: {},
    });
    
    const params = {
        headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer test-token',
        },
        tags: { name: query.name },
    };
    
    const res = http.post('http://localhost:8080/graphql', payload, params);
    
    check(res, {
        'status is 200': (r) => r.status === 200,
        'no errors': (r) => !r.json('errors'),
        'has data': (r) => r.json('data') !== null,
    });
    
    sleep(0.1); // Think time
}

function selectWeighted(items) {
    const random = Math.random();
    let sum = 0;
    for (const item of items) {
        sum += item.weight;
        if (random <= sum) return item;
    }
    return items[0];
}
```

Run with: `k6 run scripts/k6-load-test.js`

#### Task 6.4.2: Implement Performance Optimizations
Apply discovered optimizations:
```rust
// Connection pool tuning
pub fn optimize_connection_pools(config: &mut Config) {
    // Based on load testing results
    config.database.pool_size = 50;
    config.database.acquire_timeout = Duration::from_secs(2);
    
    // HTTP server tuning
    config.server.worker_threads = num_cpus::get();
    config.server.blocking_threads = 512;
    config.server.keep_alive = Duration::from_secs(75);
}

// Query optimization
pub struct QueryOptimizer {
    complexity_analyzer: ComplexityAnalyzer,
    query_planner: QueryPlanner,
}

impl QueryOptimizer {
    pub fn optimize_query(&self, query: &Document) -> Result<ExecutionPlan> {
        // Analyze query complexity
        let complexity = self.complexity_analyzer.analyze(query)?;
        
        if complexity.score > 1000 {
            return Err(Error::complexity_exceeded());
        }
        
        // Generate optimal execution plan
        let plan = self.query_planner.plan(query, complexity)?;
        
        Ok(plan)
    }
}
```

#### Task 6.4.3: Performance Monitoring
Add production performance monitoring:
```rust
pub struct PerformanceMonitor {
    latency_tracker: LatencyTracker,
    throughput_counter: ThroughputCounter,
    error_analyzer: ErrorAnalyzer,
}

impl PerformanceMonitor {
    pub async fn record_request(&self, duration: Duration, status: RequestStatus) {
        self.latency_tracker.record(duration);
        self.throughput_counter.increment();
        
        if !status.is_success() {
            self.error_analyzer.record(status);
        }
        
        // Alert on SLO violations
        if self.latency_tracker.p99() > Duration::from_millis(200) {
            self.send_alert("P99 latency exceeds 200ms").await;
        }
    }
}
```

### 🛑 CHECKPOINT 4: Complete Phase 6 System Review
**Deliverables**:
- All performance optimizations implemented
- Load tests passing at 1000 RPS
- P99 latency under 200ms
- No N+1 queries detected
- Cache hit rate > 50%

**Final Verification**:
- Run complete performance test suite
- Profile under sustained load
- Document any remaining optimization opportunities

---

## Common Troubleshooting

**📚 First Stop**: Check the [Performance Optimization Errors](../../junior-dev-helper/performance-optimization-errors.md) guide for detailed solutions to these and other common issues.

### Issue: N+1 queries still occurring
**Solution**: 
1. Check DataLoader is used in all resolver relationships
2. See [DataLoader N+1 Tutorial](../../junior-dev-helper/dataloader-n1-tutorial.md) section on "Testing for N+1 Queries"
3. If tutorial doesn't cover your case, document the pattern and solution in `api/.claude/.reviews/phase-6-patterns.md`

### Issue: Cache hit rate too low
**Solution**: Analyze query patterns, adjust TTLs, implement cache warming. Review [Caching Strategies Guide](../../junior-dev-helper/caching-strategies-guide.md) section on "Monitoring Cache Performance".

### Issue: Timeouts not cascading properly
**Solution**: Verify TimeoutContext propagation through all layers. See [Timeout Management Guide](../../junior-dev-helper/timeout-management-guide.md) section on "The Timeout Hierarchy".

### Issue: Load test failures
**Solution**: Profile bottlenecks, tune connection pools, optimize queries. Consult [Performance Testing Tutorial](../../junior-dev-helper/performance-testing-tutorial.md) for debugging techniques.

## Performance Considerations

1. **DataLoader**: Batch size affects latency vs throughput trade-off
2. **Caching**: Memory usage vs hit rate balance
3. **Timeouts**: Too aggressive causes failures, too relaxed causes hanging
4. **Load Testing**: Use realistic query patterns from production

## Security Requirements

1. **Cache Isolation**: Never share cached data between users
2. **Query Complexity**: Prevent DoS through complex queries
3. **Timeout Errors**: Don't leak internal details
4. **Load Testing**: Use sanitized test data

## Success Criteria

By the end of Phase 6:
1. Consistent sub-200ms P99 latency
2. No N+1 queries in any code path
3. Successful 1000 RPS sustained load
4. >50% cache hit rate in production patterns
5. Zero timeout-related hangs
6. Linear scaling with CPU cores

## Next Phase Preview

Phase 7 will focus on Container & Deployment:
- Multi-stage Dockerfile optimization
- Kubernetes deployment manifests
- Security scanning and hardening
- Secret management integration
# Caching Strategies Guide

## Why Caching Matters

Caching is storing frequently accessed data in fast storage to avoid expensive recomputation or database queries. In a GraphQL API, caching can dramatically improve performance:

- **Reduce database load**: Fewer queries = happier database
- **Lower latency**: Memory access is ~100,000x faster than disk
- **Better scalability**: Handle more users with same resources
- **Cost savings**: Less compute and database usage

## Types of Caching in GraphQL

### 1. Response Caching (What Phase 6 Implements)

Stores complete GraphQL query responses:

```rust
// Query + Variables + User = Cached Response
Query: "{ user(id: $id) { name posts { title } } }"
Variables: { "id": "123" }
User: "user_456"
=> Cached JSON response
```

**Pros**: Fast, simple to implement
**Cons**: Memory usage, cache invalidation complexity

### 2. Field-Level Caching

Caches individual field resolver results:

```rust
// Each field can have its own cache
#[graphql(cache = "5m")]
async fn expensive_calculation(&self) -> Result<i32> {
    // This result is cached for 5 minutes
}
```

### 3. DataLoader Caching (Request-Scoped)

Prevents duplicate work within a single request:

```rust
// First access: loads from DB
let user1 = loader.load_one("123").await?;

// Second access: returns cached
let user2 = loader.load_one("123").await?;
```

## Response Cache Implementation

### Cache Key Generation

The cache key must be:
1. **Unique** per query/variables combination
2. **Isolated** per user (security!)
3. **Stable** across requests

```rust
#[derive(Hash, Eq, PartialEq)]
struct CacheKey {
    query_hash: u64,      // Hash of normalized query
    variables_hash: u64,  // Hash of variables
    user_id: String,      // User isolation
}

impl ResponseCache {
    fn generate_key(&self, query: &str, variables: &Value, user_id: &str) -> CacheKey {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        // Normalize query (remove whitespace, formatting)
        let normalized = self.normalize_query(query);
        
        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        let query_hash = hasher.finish();
        
        // Hash variables (order-independent)
        hasher = DefaultHasher::new();
        self.hash_json_value(variables, &mut hasher);
        let variables_hash = hasher.finish();
        
        CacheKey {
            query_hash,
            variables_hash,
            user_id: user_id.to_string(),
        }
    }
    
    fn normalize_query(&self, query: &str) -> String {
        // Remove comments, extra whitespace, etc.
        query
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
```

### TTL (Time To Live) Strategies

Different queries need different cache durations:

```rust
pub struct CacheConfig {
    pub default_ttl: Duration,
    pub query_ttls: HashMap<String, Duration>,
}

impl ResponseCache {
    fn get_ttl(&self, query: &str) -> Duration {
        // Check for specific query patterns
        if query.contains("currentUser") {
            Duration::from_secs(60)  // 1 minute for user data
        } else if query.contains("posts") {
            Duration::from_secs(300) // 5 minutes for posts
        } else if query.contains("statistics") {
            Duration::from_secs(3600) // 1 hour for stats
        } else {
            self.config.default_ttl
        }
    }
}
```

### User Isolation

**CRITICAL**: Never share cached data between users!

```rust
// ❌ BAD: Shared cache key
let key = format!("{:?}:{:?}", query, variables);

// ✅ GOOD: User-specific key
let key = CacheKey {
    query_hash,
    variables_hash,
    user_id: auth.user_id.clone(),
};

// Even better: Include permissions
let key = CacheKey {
    query_hash,
    variables_hash,
    user_id: auth.user_id.clone(),
    permissions_hash: hash_permissions(&auth.permissions),
};
```

## Cache Invalidation

> "There are only two hard things in Computer Science: cache invalidation and naming things." - Phil Karlton

### Strategy 1: Time-Based (Simple)

```rust
struct CachedResponse {
    data: Value,
    expires_at: Instant,
}

impl ResponseCache {
    async fn get(&self, key: &CacheKey) -> Option<Value> {
        let cache = self.cache.read().await;
        
        if let Some(cached) = cache.get(key) {
            if Instant::now() < cached.expires_at {
                self.metrics.record_hit();
                return Some(cached.data.clone());
            }
        }
        
        self.metrics.record_miss();
        None
    }
}
```

### Strategy 2: Mutation-Based (Smart)

```rust
impl CacheInvalidator {
    async fn invalidate_for_mutation(&self, mutation: &str, args: &Value) {
        match mutation {
            "createPost" | "updatePost" | "deletePost" => {
                // Invalidate all queries containing "posts"
                self.invalidate_pattern(r"\bposts\b").await;
                self.invalidate_pattern(r"\bpost\s*\(").await;
            }
            
            "updateUser" => {
                // Only invalidate specific user's data
                if let Some(user_id) = args.get("id").and_then(|v| v.as_str()) {
                    self.invalidate_user_queries(user_id).await;
                }
            }
            
            _ => {
                // Unknown mutation - be safe
                warn!("Unknown mutation {}, invalidating all", mutation);
                self.invalidate_all().await;
            }
        }
    }
}
```

### Strategy 3: Tag-Based (Advanced)

```rust
// Tag cached entries with related entities
struct CachedResponse {
    data: Value,
    tags: HashSet<String>,  // e.g., ["user:123", "post:456"]
    expires_at: Instant,
}

impl ResponseCache {
    async fn invalidate_by_tags(&self, tags: &[String]) {
        let mut cache = self.cache.write().await;
        
        cache.retain(|_, cached| {
            // Keep if no tags match
            !tags.iter().any(|tag| cached.tags.contains(tag))
        });
    }
}

// Usage
cache.invalidate_by_tags(&["user:123", "post:*"]).await;
```

## Memory Management

### Bounded Cache Size

```rust
use lru::LruCache;

pub struct ResponseCache {
    // LRU = Least Recently Used eviction
    cache: Arc<Mutex<LruCache<CacheKey, CachedResponse>>>,
    config: CacheConfig,
}

impl ResponseCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(Mutex::new(
                LruCache::new(config.max_entries.unwrap_or(10_000))
            )),
            config,
        }
    }
}
```

### Memory Limits

```rust
pub struct MemoryLimitedCache {
    cache: HashMap<CacheKey, CachedResponse>,
    total_size: usize,
    max_memory: usize,
}

impl MemoryLimitedCache {
    fn insert(&mut self, key: CacheKey, response: CachedResponse) {
        let size = self.estimate_size(&response);
        
        // Evict entries until we have space
        while self.total_size + size > self.max_memory && !self.cache.is_empty() {
            // Remove oldest/largest/least used
            self.evict_one();
        }
        
        self.total_size += size;
        self.cache.insert(key, response);
    }
    
    fn estimate_size(&self, response: &CachedResponse) -> usize {
        // Rough estimate of JSON size
        response.data.to_string().len()
    }
}
```

## Cache Warming

Pre-populate cache with common queries:

```rust
pub struct CacheWarmer {
    cache: Arc<ResponseCache>,
    common_queries: Vec<(String, Value)>,
}

impl CacheWarmer {
    pub async fn warm_cache(&self, user_ids: &[String]) {
        for user_id in user_ids {
            for (query, variables) in &self.common_queries {
                // Execute query and cache result
                if let Ok(result) = self.execute_query(query, variables, user_id).await {
                    self.cache.set(query, variables, user_id, &result).await;
                }
            }
        }
    }
    
    fn common_queries() -> Vec<(String, Value)> {
        vec![
            // Dashboard query
            (
                "{ currentUser { name notifications { count } } }".to_string(),
                json!({}),
            ),
            // Posts list
            (
                "{ posts(first: 20) { id title author { name } } }".to_string(),
                json!({}),
            ),
        ]
    }
}
```

## Monitoring Cache Performance

### Metrics to Track

```rust
pub struct CacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    invalidations: AtomicU64,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let total = hits + self.misses.load(Ordering::Relaxed) as f64;
        
        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }
    
    pub fn record_to_prometheus(&self) {
        gauge!("cache_hit_rate").set(self.hit_rate());
        counter!("cache_hits_total").increment(self.hits.load(Ordering::Relaxed));
        counter!("cache_misses_total").increment(self.misses.load(Ordering::Relaxed));
        counter!("cache_evictions_total").increment(self.evictions.load(Ordering::Relaxed));
    }
}
```

### What to Monitor

1. **Hit Rate**: Should be > 50% for effective caching
2. **Eviction Rate**: High = cache too small
3. **Invalidation Patterns**: Identify hot spots
4. **Memory Usage**: Don't let cache grow unbounded

```rust
// Log cache stats periodically
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        
        info!(
            hit_rate = metrics.hit_rate(),
            total_hits = metrics.hits.load(Ordering::Relaxed),
            total_misses = metrics.misses.load(Ordering::Relaxed),
            cache_size = cache.size().await,
            "Cache statistics"
        );
    }
});
```

## Common Caching Mistakes

### 1. Caching Personalized Data Globally

```rust
// ❌ BAD: One cache for all users
static GLOBAL_CACHE: Lazy<Cache> = Lazy::new(Cache::new);

// ✅ GOOD: User isolation
struct CacheKey {
    user_id: String,
    query_hash: u64,
}
```

### 2. No Cache Invalidation Strategy

```rust
// ❌ BAD: Cache forever
cache.insert(key, value);

// ✅ GOOD: TTL + invalidation
cache.insert_with_ttl(key, value, Duration::from_secs(300));
cache_invalidator.register_tags(&key, &["user:123", "posts"]);
```

### 3. Caching Errors

```rust
// ❌ BAD: Cache errors
match fetch_data().await {
    Ok(data) => cache.set(key, data),
    Err(e) => cache.set(key, e), // Don't cache errors!
}

// ✅ GOOD: Only cache success
if let Ok(data) = fetch_data().await {
    cache.set(key, data);
}
```

### 4. Not Monitoring Cache

```rust
// ❌ BAD: No visibility
cache.get(key).unwrap_or_else(|| fetch())

// ✅ GOOD: Track metrics
let result = if let Some(cached) = cache.get(key) {
    metrics.record_hit();
    cached
} else {
    metrics.record_miss();
    let fresh = fetch().await?;
    cache.set(key, fresh.clone());
    fresh
};
```

## Testing Cache Behavior

### Test Cache Isolation

```rust
#[tokio::test]
async fn test_user_cache_isolation() {
    let cache = ResponseCache::new(Default::default());
    
    let query = "{ posts { id } }";
    let vars = json!({});
    
    // User 1 caches their data
    cache.set(query, &vars, "user1", &json!({"posts": [{"id": "1"}]})).await;
    
    // User 2 shouldn't see it
    assert_eq!(cache.get(query, &vars, "user2").await, None);
    
    // User 1 should see their data
    assert_eq!(
        cache.get(query, &vars, "user1").await,
        Some(json!({"posts": [{"id": "1"}]}))
    );
}
```

### Test Invalidation

```rust
#[tokio::test]
async fn test_mutation_invalidation() {
    let cache = ResponseCache::new(Default::default());
    let invalidator = CacheInvalidator::new(cache.clone());
    
    // Cache some queries
    cache.set("{ posts { id } }", &json!({}), "user1", &json!({"posts": []})).await;
    cache.set("{ users { id } }", &json!({}), "user1", &json!({"users": []})).await;
    
    // Mutation should invalidate posts
    invalidator.invalidate_for_mutation("createPost", &json!({})).await;
    
    // Posts cache should be cleared
    assert_eq!(cache.get("{ posts { id } }", &json!({}), "user1").await, None);
    
    // Users cache should remain
    assert!(cache.get("{ users { id } }", &json!({}), "user1").await.is_some());
}
```

## Summary

Effective caching requires:
1. **Security first**: Always isolate by user
2. **Smart invalidation**: Know when to clear
3. **Memory bounds**: Don't let cache grow forever
4. **Monitoring**: Track hit rates and performance
5. **Testing**: Verify isolation and invalidation

Remember: A badly configured cache is worse than no cache!
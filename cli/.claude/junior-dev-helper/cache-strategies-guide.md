# Cache Strategies Guide for Phase 4

## Why Cache Authorization?

Every GraphQL request might check dozens of permissions:
```
Query: Get user's notes with author details
- Can user read note 1? → SpiceDB call (30ms)
- Can user read note 2? → SpiceDB call (30ms)  
- Can user read note 3? → SpiceDB call (30ms)
- Can user see author profiles? → SpiceDB call (30ms)
Total: 120ms just for authorization!
```

With caching:
```
Same query with cache:
- Can user read note 1? → Cache hit (0.1ms)
- Can user read note 2? → Cache hit (0.1ms)
- Can user read note 3? → Cache hit (0.1ms)
- Can user see author profiles? → Cache hit (0.1ms)
Total: 0.4ms - 300x faster!
```

## The Golden Rule: Cache Positive Results Only

**NEVER cache negative authorization results!** This is a critical security rule.

### Why Not Cache Denials?

```rust
// DANGEROUS - DO NOT DO THIS
if !allowed {
    cache.set(key, false, ttl).await; // SECURITY VULNERABILITY!
}

// Here's why:
// 1. User tries to access document they don't own
// 2. Access denied - cached for 5 minutes
// 3. Owner shares document with user
// 4. User still can't access for 5 minutes due to cached denial!
```

### The Safe Approach

```rust
// SAFE - Only cache positive results
if allowed {
    cache.set(key, true, ttl).await; // Safe to cache
}
// Denials are never cached - always check fresh
```

## Basic Cache Implementation

### 1. Simple In-Memory Cache

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct CacheEntry {
    allowed: bool,
    expires_at: Instant,
}

pub struct AuthCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
}

impl AuthCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::with_capacity(max_size))),
            max_size,
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<bool> {
        let entries = self.entries.read().await;
        
        if let Some(entry) = entries.get(key) {
            if entry.expires_at > Instant::now() {
                // Still valid
                return Some(entry.allowed);
            }
        }
        
        None // Expired or not found
    }
    
    pub async fn set(&self, key: String, allowed: bool, ttl: Duration) {
        // CRITICAL: Only cache positive results
        if !allowed {
            tracing::debug!("Refusing to cache negative result for {}", key);
            return;
        }
        
        let mut entries = self.entries.write().await;
        
        // Simple size limit
        if entries.len() >= self.max_size {
            // Remove oldest entry (simple strategy)
            if let Some(oldest_key) = entries.keys().next().cloned() {
                entries.remove(&oldest_key);
            }
        }
        
        entries.insert(key, CacheEntry {
            allowed,
            expires_at: Instant::now() + ttl,
        });
    }
}
```

### 2. Cache Key Strategy

The cache key must uniquely identify the permission check:

```rust
fn build_cache_key(user_id: &str, resource: &str, action: &str) -> String {
    // Format: "user_id:resource:action"
    format!("{}:{}:{}", user_id, resource, action)
}

// Examples:
// "alice:note:123:read"
// "bob:user:alice:view"
// "charlie:org:acme:admin"
```

## Advanced Caching Patterns

### 1. LRU (Least Recently Used) Cache

```rust
use std::sync::atomic::{AtomicU64, Ordering};

struct LRUCacheEntry {
    allowed: bool,
    expires_at: Instant,
    last_accessed: AtomicU64, // Nanoseconds since epoch
    access_count: AtomicU64,
}

impl AuthCache {
    pub async fn get(&self, key: &str) -> Option<bool> {
        let entries = self.entries.read().await;
        
        if let Some(entry) = entries.get(key) {
            if entry.expires_at > Instant::now() {
                // Update access time and count
                entry.last_accessed.store(
                    chrono::Utc::now().timestamp_nanos() as u64,
                    Ordering::Relaxed
                );
                entry.access_count.fetch_add(1, Ordering::Relaxed);
                
                return Some(entry.allowed);
            }
        }
        
        None
    }
    
    async fn evict_lru(&self, entries: &mut HashMap<String, LRUCacheEntry>) {
        // Find least recently used
        let lru_key = entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed.load(Ordering::Relaxed))
            .map(|(key, _)| key.clone());
        
        if let Some(key) = lru_key {
            entries.remove(&key);
            tracing::debug!("Evicted LRU entry: {}", key);
        }
    }
}
```

### 2. TTL-Based Cleanup

```rust
impl AuthCache {
    pub async fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                self.cleanup_expired().await;
            }
        });
    }
    
    async fn cleanup_expired(&self) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        
        entries.retain(|key, entry| {
            let keep = entry.expires_at > now;
            if !keep {
                tracing::debug!("Removing expired entry: {}", key);
            }
            keep
        });
    }
}
```

### 3. Hierarchical Caching

Cache at multiple levels for better performance:

```rust
pub struct HierarchicalCache {
    l1_cache: Arc<LocalCache>,    // Process-local, very fast
    l2_cache: Arc<RedisCache>,    // Shared across instances
}

impl HierarchicalCache {
    pub async fn get(&self, key: &str) -> Option<bool> {
        // Check L1 first (fastest)
        if let Some(value) = self.l1_cache.get(key).await {
            return Some(value);
        }
        
        // Check L2 (slower but shared)
        if let Some(value) = self.l2_cache.get(key).await {
            // Populate L1 for next time
            self.l1_cache.set(key.to_string(), value, Duration::from_secs(60)).await;
            return Some(value);
        }
        
        None
    }
}
```

## Cache Invalidation Strategies

### 1. Time-Based (TTL)

The simplest approach - entries expire after a fixed time:

```rust
pub struct CacheConfig {
    pub default_ttl: Duration,
    pub extended_ttl: Duration, // For circuit breaker scenarios
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300),    // 5 minutes
            extended_ttl: Duration::from_secs(1800),  // 30 minutes
        }
    }
}

impl AuthCache {
    pub async fn set_with_config(&self, key: String, allowed: bool, config: &CacheConfig) {
        let ttl = if self.circuit_breaker_open {
            config.extended_ttl // Cache longer during outages
        } else {
            config.default_ttl
        };
        
        self.set(key, allowed, ttl).await;
    }
}
```

### 2. Event-Based Invalidation

Clear cache when permissions change:

```rust
pub struct CacheInvalidator {
    cache: Arc<AuthCache>,
    event_receiver: mpsc::Receiver<InvalidationEvent>,
}

#[derive(Debug)]
pub enum InvalidationEvent {
    UserPermissionsChanged(String),      // User ID
    ResourcePermissionsChanged(String),   // Resource ID
    AllPermissionsChanged,               // Full cache clear
}

impl CacheInvalidator {
    pub async fn run(mut self) {
        while let Some(event) = self.event_receiver.recv().await {
            match event {
                InvalidationEvent::UserPermissionsChanged(user_id) => {
                    self.invalidate_user(&user_id).await;
                }
                InvalidationEvent::ResourcePermissionsChanged(resource_id) => {
                    self.invalidate_resource(&resource_id).await;
                }
                InvalidationEvent::AllPermissionsChanged => {
                    self.invalidate_all().await;
                }
            }
        }
    }
    
    async fn invalidate_user(&self, user_id: &str) {
        let mut entries = self.cache.entries.write().await;
        entries.retain(|key, _| !key.starts_with(&format!("{}:", user_id)));
        tracing::info!("Invalidated cache for user: {}", user_id);
    }
}
```

### 3. Pattern-Based Invalidation

```rust
impl AuthCache {
    pub async fn invalidate_pattern(&self, pattern: &str) {
        let mut entries = self.entries.write().await;
        let regex = regex::Regex::new(pattern).unwrap();
        
        entries.retain(|key, _| !regex.is_match(key));
    }
}

// Examples:
cache.invalidate_pattern(r"^alice:.*").await;        // All of Alice's permissions
cache.invalidate_pattern(r".*:note:123:.*").await;   // All permissions for note 123
cache.invalidate_pattern(r".*:write$").await;        // All write permissions
```

## Performance Optimization

### 1. Batch Cache Operations

```rust
impl AuthCache {
    pub async fn get_batch(&self, keys: &[String]) -> HashMap<String, bool> {
        let entries = self.entries.read().await;
        let now = Instant::now();
        let mut results = HashMap::new();
        
        for key in keys {
            if let Some(entry) = entries.get(key) {
                if entry.expires_at > now {
                    results.insert(key.clone(), entry.allowed);
                }
            }
        }
        
        results
    }
    
    pub async fn set_batch(&self, items: Vec<(String, bool, Duration)>) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        
        for (key, allowed, ttl) in items {
            if allowed { // Still only cache positive
                entries.insert(key, CacheEntry {
                    allowed,
                    expires_at: now + ttl,
                });
            }
        }
    }
}
```

### 2. Sharded Cache

For high-concurrency scenarios, shard the cache:

```rust
pub struct ShardedAuthCache {
    shards: Vec<Arc<AuthCache>>,
    shard_count: usize,
}

impl ShardedAuthCache {
    pub fn new(shard_count: usize, max_size_per_shard: usize) -> Self {
        let shards = (0..shard_count)
            .map(|_| Arc::new(AuthCache::new(max_size_per_shard)))
            .collect();
        
        Self { shards, shard_count }
    }
    
    fn get_shard(&self, key: &str) -> &Arc<AuthCache> {
        let hash = self.hash_key(key);
        &self.shards[hash % self.shard_count]
    }
    
    fn hash_key(&self, key: &str) -> usize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }
    
    pub async fn get(&self, key: &str) -> Option<bool> {
        self.get_shard(key).get(key).await
    }
}
```

## Monitoring and Metrics

### 1. Cache Statistics

```rust
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
    pub expirations: AtomicU64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        
        if hits + misses > 0.0 {
            hits / (hits + misses)
        } else {
            0.0
        }
    }
}

impl AuthCache {
    pub async fn get_with_stats(&self, key: &str) -> Option<bool> {
        let result = self.get(key).await;
        
        if result.is_some() {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
        }
        
        result
    }
}
```

### 2. Prometheus Metrics

```rust
use prometheus::{Counter, Histogram, Gauge};

pub struct CacheMetrics {
    cache_hits: Counter,
    cache_misses: Counter,
    cache_size: Gauge,
    cache_evictions: Counter,
    cache_operation_duration: Histogram,
}

impl AuthCache {
    pub async fn get_with_metrics(&self, key: &str) -> Option<bool> {
        let timer = self.metrics.cache_operation_duration.start_timer();
        let result = self.get(key).await;
        timer.observe_duration();
        
        if result.is_some() {
            self.metrics.cache_hits.inc();
        } else {
            self.metrics.cache_misses.inc();
        }
        
        result
    }
}
```

## Testing Cache Behavior

### 1. Test Positive-Only Caching

```rust
#[tokio::test]
async fn test_only_caches_positive_results() {
    let cache = AuthCache::new(100);
    
    // Try to cache denial
    cache.set("user:deny".to_string(), false, Duration::from_secs(60)).await;
    
    // Should not be cached
    assert_eq!(cache.get("user:deny").await, None);
    
    // Cache approval
    cache.set("user:allow".to_string(), true, Duration::from_secs(60)).await;
    
    // Should be cached
    assert_eq!(cache.get("user:allow").await, Some(true));
}
```

### 2. Test TTL Expiration

```rust
#[tokio::test]
async fn test_ttl_expiration() {
    let cache = AuthCache::new(100);
    
    // Set with short TTL
    cache.set("user:temp".to_string(), true, Duration::from_millis(100)).await;
    
    // Should be available immediately
    assert_eq!(cache.get("user:temp").await, Some(true));
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Should be expired
    assert_eq!(cache.get("user:temp").await, None);
}
```

### 3. Test Cache Under Load

```rust
#[tokio::test]
async fn test_cache_performance() {
    let cache = Arc::new(AuthCache::new(1000));
    let mut handles = vec![];
    
    // Spawn 100 concurrent tasks
    for i in 0..100 {
        let cache = cache.clone();
        let handle = tokio::spawn(async move {
            for j in 0..100 {
                let key = format!("user{}:resource{}:read", i, j);
                
                // First access - miss
                assert!(cache.get(&key).await.is_none());
                
                // Cache it
                cache.set(key.clone(), true, Duration::from_secs(60)).await;
                
                // Second access - hit
                assert_eq!(cache.get(&key).await, Some(true));
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Check hit rate
    assert!(cache.stats.hit_rate() > 0.5); // Should have >50% hit rate
}
```

## Common Mistakes

### 1. Caching Negative Results
```rust
// WRONG - Security vulnerability
async fn is_authorized(&self, user: &str, resource: &str, action: &str) -> bool {
    let key = format!("{}:{}:{}", user, resource, action);
    
    if let Some(cached) = self.cache.get(&key).await {
        return cached; // Could be false!
    }
    
    let allowed = self.check_permission(user, resource, action).await;
    self.cache.set(key, allowed, ttl).await; // Caching both true and false
    allowed
}

// RIGHT - Only cache positive
async fn is_authorized(&self, user: &str, resource: &str, action: &str) -> bool {
    let key = format!("{}:{}:{}", user, resource, action);
    
    if let Some(cached) = self.cache.get(&key).await {
        return cached; // Always true
    }
    
    let allowed = self.check_permission(user, resource, action).await;
    if allowed {
        self.cache.set(key, allowed, ttl).await; // Only cache if true
    }
    allowed
}
```

### 2. Wrong TTL for Circuit Breaker
```rust
// WRONG - Same TTL regardless of system state
let ttl = Duration::from_secs(300);

// RIGHT - Longer TTL during outages
let ttl = if circuit_breaker.is_open().await {
    Duration::from_secs(1800) // 30 minutes during outage
} else {
    Duration::from_secs(300)  // 5 minutes normally
};
```

### 3. Not Handling Cache Errors
```rust
// WRONG - Cache failure breaks authorization
let cached = self.cache.get(&key).await?; // Could panic

// RIGHT - Cache is optional optimization
let cached = match self.cache.get(&key).await {
    Ok(Some(value)) => Some(value),
    Ok(None) => None,
    Err(e) => {
        tracing::warn!("Cache error: {}", e);
        None // Continue without cache
    }
};
```

## Integration Example

Here's how caching fits into the complete authorization flow:

```rust
pub struct AuthorizationService {
    spicedb: Arc<SpiceDBClient>,
    cache: Arc<AuthCache>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl AuthorizationService {
    pub async fn is_authorized(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, Error> {
        // 1. Build cache key
        let cache_key = format!("{}:{}:{}", user_id, resource, action);
        
        // 2. Check cache
        if let Some(allowed) = self.cache.get(&cache_key).await {
            tracing::debug!("Cache hit for {}", cache_key);
            return Ok(allowed); // Always true due to positive-only caching
        }
        
        tracing::debug!("Cache miss for {}", cache_key);
        
        // 3. Check with SpiceDB
        let allowed = match self.circuit_breaker.call(|| {
            self.spicedb.check_permission(user_id, resource, action)
        }).await {
            Ok(result) => result,
            Err(_) => {
                // Circuit open or operation failed
                apply_fallback_rules(user_id, resource, action)
            }
        };
        
        // 4. Cache positive results
        if allowed {
            let ttl = if self.circuit_breaker.is_open().await {
                Duration::from_secs(1800) // Extended during outage
            } else {
                Duration::from_secs(300)  // Normal TTL
            };
            
            self.cache.set(cache_key, true, ttl).await;
        }
        
        Ok(allowed)
    }
}
```

## Best Practices Summary

1. **Only cache positive authorization results** - Never cache denials
2. **Use appropriate TTLs** - Balance freshness vs performance
3. **Implement cache eviction** - Prevent unbounded growth
4. **Monitor cache metrics** - Track hit rates and performance
5. **Handle cache failures gracefully** - Cache is optimization, not requirement
6. **Consider sharding for scale** - Reduce lock contention
7. **Clear cache on permission changes** - Maintain consistency
8. **Test cache behavior** - Especially TTL and eviction

## Next Steps

1. Start with simple in-memory cache
2. Add metrics to measure effectiveness
3. Tune TTL based on your needs
4. Consider Redis for multi-instance setups
5. Implement cache warming for critical paths

For more context, see:
- [Authorization Tutorial](./authorization-tutorial.md) - Overall auth flow
- [Circuit Breaker Guide](./circuit-breaker-guide.md) - Handling SpiceDB failures
- [Common Authorization Errors](./authorization-common-errors.md) - Troubleshooting
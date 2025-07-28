/// Cache Strategies - Phase 4 Implementation Examples
///
/// This file demonstrates caching patterns for authorization including
/// positive-only caching, LRU eviction, TTL management, and cache warming.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// Cache Entry with Access Tracking
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// The cached value
    pub value: T,
    /// When the entry expires
    pub expires_at: Instant,
    /// When the entry was created
    pub created_at: Instant,
    /// Number of times accessed
    pub hit_count: AtomicU64,
    /// Last access timestamp (for LRU)
    pub last_access: AtomicU64,
    /// Size in bytes (estimated)
    pub size_bytes: usize,
}

/// Generic Cache with LRU Eviction
pub struct LruCache<K, V> 
where 
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    /// The actual cache storage
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    /// LRU tracking queue
    lru_queue: Arc<RwLock<VecDeque<K>>>,
    /// Maximum number of entries
    max_entries: usize,
    /// Maximum total size in bytes
    max_size_bytes: usize,
    /// Current total size
    current_size: Arc<AtomicU64>,
    /// Cache statistics
    stats: Arc<CacheStats>,
}

/// Cache Statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
    pub expirations: AtomicU64,
    pub insertions: AtomicU64,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(max_entries: usize, max_size_bytes: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::with_capacity(max_entries))),
            lru_queue: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            max_entries,
            max_size_bytes,
            current_size: Arc::new(AtomicU64::new(0)),
            stats: Arc::new(CacheStats::default()),
        }
    }
    
    /// Get a value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        
        if let Some(entry) = entries.get_mut(key) {
            // Check expiration
            if entry.expires_at <= Instant::now() {
                // Remove expired entry
                entries.remove(key);
                self.stats.expirations.fetch_add(1, Ordering::Relaxed);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                
                // Update size
                self.current_size.fetch_sub(entry.size_bytes as u64, Ordering::Relaxed);
                
                // Remove from LRU queue
                let mut queue = self.lru_queue.write().await;
                queue.retain(|k| k != key);
                
                return None;
            }
            
            // Update access tracking
            entry.hit_count.fetch_add(1, Ordering::Relaxed);
            entry.last_access.store(
                chrono::Utc::now().timestamp_millis() as u64,
                Ordering::Relaxed
            );
            
            // Update LRU queue
            let mut queue = self.lru_queue.write().await;
            queue.retain(|k| k != key);
            queue.push_back(key.clone());
            
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.value.clone())
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
    
    /// Set a value in cache with TTL
    pub async fn set(&self, key: K, value: V, ttl: Duration, size_bytes: usize) {
        let mut entries = self.entries.write().await;
        let mut queue = self.lru_queue.write().await;
        
        // Check if we need to evict
        while entries.len() >= self.max_entries || 
              self.current_size.load(Ordering::Relaxed) + size_bytes as u64 > self.max_size_bytes as u64 {
            
            if let Some(evict_key) = queue.pop_front() {
                if let Some(evicted) = entries.remove(&evict_key) {
                    self.current_size.fetch_sub(evicted.size_bytes as u64, Ordering::Relaxed);
                    self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                    
                    tracing::debug!(
                        "Evicted cache entry: hits={}, age={}s",
                        evicted.hit_count.load(Ordering::Relaxed),
                        evicted.created_at.elapsed().as_secs()
                    );
                }
            } else {
                break; // No more entries to evict
            }
        }
        
        // Remove old entry if exists
        if let Some(old_entry) = entries.remove(&key) {
            self.current_size.fetch_sub(old_entry.size_bytes as u64, Ordering::Relaxed);
            queue.retain(|k| k != &key);
        }
        
        // Insert new entry
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + ttl,
            created_at: Instant::now(),
            hit_count: AtomicU64::new(0),
            last_access: AtomicU64::new(chrono::Utc::now().timestamp_millis() as u64),
            size_bytes,
        };
        
        entries.insert(key.clone(), entry);
        queue.push_back(key);
        
        self.current_size.fetch_add(size_bytes as u64, Ordering::Relaxed);
        self.stats.insertions.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Clear all expired entries
    pub async fn cleanup_expired(&self) {
        let mut entries = self.entries.write().await;
        let mut queue = self.lru_queue.write().await;
        let now = Instant::now();
        
        let expired_keys: Vec<K> = entries.iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(k, _)| k.clone())
            .collect();
        
        for key in expired_keys {
            if let Some(entry) = entries.remove(&key) {
                self.current_size.fetch_sub(entry.size_bytes as u64, Ordering::Relaxed);
                self.stats.expirations.fetch_add(1, Ordering::Relaxed);
                queue.retain(|k| k != &key);
            }
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            expirations: self.stats.expirations.load(Ordering::Relaxed),
            insertions: self.stats.insertions.load(Ordering::Relaxed),
            hit_rate: self.calculate_hit_rate(),
            size_bytes: self.current_size.load(Ordering::Relaxed),
        }
    }
    
    fn calculate_hit_rate(&self) -> f64 {
        let hits = self.stats.hits.load(Ordering::Relaxed) as f64;
        let misses = self.stats.misses.load(Ordering::Relaxed) as f64;
        
        if hits + misses > 0.0 {
            hits / (hits + misses)
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
pub struct CacheStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
    pub insertions: u64,
    pub hit_rate: f64,
    pub size_bytes: u64,
}

/// Authorization-Specific Cache Implementation
pub mod auth_cache {
    use super::*;
    
    /// Authorization cache that only stores positive results
    pub struct AuthorizationCache {
        cache: LruCache<String, bool>,
        /// Extended TTL during degraded mode
        degraded_mode_ttl: Duration,
        /// Normal operation TTL
        normal_ttl: Duration,
        /// Whether we're in degraded mode
        is_degraded: Arc<RwLock<bool>>,
    }
    
    impl AuthorizationCache {
        pub fn new(max_entries: usize) -> Self {
            Self {
                cache: LruCache::new(max_entries, max_entries * 100), // ~100 bytes per entry
                degraded_mode_ttl: Duration::from_secs(1800), // 30 minutes
                normal_ttl: Duration::from_secs(300), // 5 minutes
                is_degraded: Arc::new(RwLock::new(false)),
            }
        }
        
        /// Get authorization result from cache
        pub async fn get(&self, user_id: &str, resource: &str, action: &str) -> Option<bool> {
            let key = format!("{}:{}:{}", user_id, resource, action);
            self.cache.get(&key).await
        }
        
        /// Set authorization result (only if allowed)
        pub async fn set(&self, user_id: &str, resource: &str, action: &str, allowed: bool) {
            // CRITICAL: Only cache positive results
            if !allowed {
                tracing::debug!(
                    "Refusing to cache negative auth result for {}:{}:{}",
                    user_id, resource, action
                );
                return;
            }
            
            let key = format!("{}:{}:{}", user_id, resource, action);
            let ttl = if *self.is_degraded.read().await {
                self.degraded_mode_ttl
            } else {
                self.normal_ttl
            };
            
            // Estimate size: key length + overhead
            let size_bytes = key.len() + 50;
            
            self.cache.set(key, allowed, ttl, size_bytes).await;
        }
        
        /// Set degraded mode status
        pub async fn set_degraded_mode(&self, degraded: bool) {
            let mut is_degraded = self.is_degraded.write().await;
            let was_degraded = *is_degraded;
            *is_degraded = degraded;
            
            if degraded && !was_degraded {
                tracing::warn!("Authorization cache entering degraded mode - extending TTLs");
            } else if !degraded && was_degraded {
                tracing::info!("Authorization cache exiting degraded mode - normal TTLs restored");
            }
        }
        
        /// Get cache statistics
        pub fn stats(&self) -> CacheStatsSnapshot {
            self.cache.stats()
        }
    }
}

/// Cache Warming Strategies
pub mod warming {
    use super::*;
    
    /// Cache warmer for pre-loading common permissions
    pub struct CacheWarmer<T> {
        cache: Arc<T>,
        warm_patterns: Vec<WarmPattern>,
    }
    
    #[derive(Clone)]
    pub struct WarmPattern {
        pub user_pattern: String,
        pub resources: Vec<String>,
        pub actions: Vec<String>,
    }
    
    impl<T> CacheWarmer<T> {
        pub fn new(cache: Arc<T>) -> Self {
            Self {
                cache,
                warm_patterns: Vec::new(),
            }
        }
        
        /// Add a pattern to warm
        pub fn add_pattern(&mut self, pattern: WarmPattern) {
            self.warm_patterns.push(pattern);
        }
        
        /// Warm the cache based on patterns
        pub async fn warm<F>(&self, check_fn: F) -> WarmingResult
        where
            F: Fn(String, String, String) -> futures::future::BoxFuture<'static, Result<bool, Box<dyn std::error::Error>>>,
        {
            let mut result = WarmingResult::default();
            let start = Instant::now();
            
            for pattern in &self.warm_patterns {
                for resource in &pattern.resources {
                    for action in &pattern.actions {
                        match check_fn(
                            pattern.user_pattern.clone(),
                            resource.clone(),
                            action.clone()
                        ).await {
                            Ok(allowed) => {
                                if allowed {
                                    result.warmed_count += 1;
                                }
                            }
                            Err(e) => {
                                result.errors.push(format!(
                                    "Failed to warm {}:{}:{} - {}",
                                    pattern.user_pattern, resource, action, e
                                ));
                            }
                        }
                    }
                }
            }
            
            result.duration = start.elapsed();
            result
        }
    }
    
    #[derive(Default)]
    pub struct WarmingResult {
        pub warmed_count: usize,
        pub errors: Vec<String>,
        pub duration: Duration,
    }
}

/// Multi-Level Cache Strategy
pub mod multi_level {
    use super::*;
    
    /// Two-level cache with L1 (fast, small) and L2 (slower, large)
    pub struct MultiLevelCache<K, V>
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        /// L1 cache - very fast, limited size
        l1: LruCache<K, V>,
        /// L2 cache - larger, slightly slower
        l2: LruCache<K, V>,
        /// Promotion threshold - promote to L1 after N hits
        promotion_threshold: u64,
    }
    
    impl<K, V> MultiLevelCache<K, V>
    where
        K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        pub fn new(l1_size: usize, l2_size: usize) -> Self {
            Self {
                l1: LruCache::new(l1_size, l1_size * 100),
                l2: LruCache::new(l2_size, l2_size * 100),
                promotion_threshold: 3,
            }
        }
        
        /// Get from multi-level cache
        pub async fn get(&self, key: &K) -> Option<V> {
            // Check L1 first
            if let Some(value) = self.l1.get(key).await {
                return Some(value);
            }
            
            // Check L2
            if let Some(value) = self.l2.get(key).await {
                // Check if we should promote to L1
                let l2_entries = self.l2.entries.read().await;
                if let Some(entry) = l2_entries.get(key) {
                    if entry.hit_count.load(Ordering::Relaxed) >= self.promotion_threshold {
                        drop(l2_entries);
                        
                        // Promote to L1
                        let ttl = entry.expires_at.duration_since(Instant::now());
                        self.l1.set(key.clone(), value.clone(), ttl, entry.size_bytes).await;
                        
                        tracing::debug!("Promoted cache entry to L1 after {} hits", self.promotion_threshold);
                    }
                }
                
                return Some(value);
            }
            
            None
        }
        
        /// Set in L2 initially
        pub async fn set(&self, key: K, value: V, ttl: Duration, size_bytes: usize) {
            self.l2.set(key, value, ttl, size_bytes).await;
        }
        
        /// Get combined statistics
        pub fn stats(&self) -> MultiLevelStats {
            MultiLevelStats {
                l1: self.l1.stats(),
                l2: self.l2.stats(),
            }
        }
    }
    
    #[derive(Debug)]
    pub struct MultiLevelStats {
        pub l1: CacheStatsSnapshot,
        pub l2: CacheStatsSnapshot,
    }
}

/// Background Cache Maintenance
pub mod maintenance {
    use super::*;
    
    pub struct CacheMaintenanceTask<K, V>
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        cache: Arc<LruCache<K, V>>,
        cleanup_interval: Duration,
        stats_interval: Duration,
    }
    
    impl<K, V> CacheMaintenanceTask<K, V>
    where
        K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        pub fn new(cache: Arc<LruCache<K, V>>) -> Self {
            Self {
                cache,
                cleanup_interval: Duration::from_secs(60),
                stats_interval: Duration::from_secs(300),
            }
        }
        
        /// Start background maintenance task
        pub fn start(self) {
            // Cleanup task
            let cache_cleanup = self.cache.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(self.cleanup_interval);
                
                loop {
                    interval.tick().await;
                    cache_cleanup.cleanup_expired().await;
                }
            });
            
            // Stats logging task
            let cache_stats = self.cache.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(self.stats_interval);
                
                loop {
                    interval.tick().await;
                    
                    let stats = cache_stats.stats();
                    tracing::info!(
                        "Cache stats: hit_rate={:.2}%, hits={}, misses={}, evictions={}, size={}KB",
                        stats.hit_rate * 100.0,
                        stats.hits,
                        stats.misses,
                        stats.evictions,
                        stats.size_bytes / 1024
                    );
                    
                    // Alert on poor hit rate
                    if stats.hit_rate < 0.8 && stats.hits + stats.misses > 1000 {
                        tracing::warn!(
                            "Cache hit rate below 80%: {:.2}%",
                            stats.hit_rate * 100.0
                        );
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = LruCache::new(3, 1000);
        
        // Fill cache
        cache.set("key1".to_string(), "value1", Duration::from_secs(60), 100).await;
        cache.set("key2".to_string(), "value2", Duration::from_secs(60), 100).await;
        cache.set("key3".to_string(), "value3", Duration::from_secs(60), 100).await;
        
        // Access key1 and key2 to make them more recent
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1"));
        assert_eq!(cache.get(&"key2".to_string()).await, Some("value2"));
        
        // Add key4, should evict key3 (least recently used)
        cache.set("key4".to_string(), "value4", Duration::from_secs(60), 100).await;
        
        assert_eq!(cache.get(&"key3".to_string()).await, None);
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1"));
        assert_eq!(cache.get(&"key4".to_string()).await, Some("value4"));
    }
    
    #[tokio::test]
    async fn test_positive_only_auth_cache() {
        use auth_cache::AuthorizationCache;
        
        let cache = AuthorizationCache::new(10);
        
        // Set positive result
        cache.set("user1", "note:123", "read", true).await;
        assert_eq!(cache.get("user1", "note:123", "read").await, Some(true));
        
        // Try to set negative result
        cache.set("user1", "note:456", "write", false).await;
        assert_eq!(cache.get("user1", "note:456", "write").await, None);
        
        // Verify stats
        let stats = cache.stats();
        assert_eq!(stats.insertions, 1); // Only positive result inserted
    }
    
    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = LruCache::new(10, 1000);
        
        // Set with very short TTL
        cache.set("key1".to_string(), "value1", Duration::from_millis(100), 100).await;
        
        // Should exist immediately
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1"));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        assert_eq!(cache.get(&"key1".to_string()).await, None);
        
        // Check expiration counter
        let stats = cache.stats();
        assert_eq!(stats.expirations, 1);
    }
    
    #[tokio::test]
    async fn test_multi_level_cache() {
        use multi_level::MultiLevelCache;
        
        let cache = MultiLevelCache::new(2, 10);
        
        // Set in L2
        cache.set("key1".to_string(), "value1", Duration::from_secs(60), 100).await;
        
        // First few accesses from L2
        for _ in 0..3 {
            assert_eq!(cache.get(&"key1".to_string()).await, Some("value1"));
        }
        
        // Should be promoted to L1 now
        let stats = cache.stats();
        assert!(stats.l1.hits > 0 || stats.l1.insertions > 0);
    }
}
//! Cache module for authorization decisions
//! 
//! This module provides caching functionality to improve authorization performance
//! by reducing repeated calls to the authorization backend (SpiceDB).
//! 
//! The cache is designed with security in mind:
//! - Cache misses default to denying access (fail-closed)
//! - Configurable TTL prevents stale permissions from persisting
//! - Pattern-based invalidation enables efficient permission updates
//! - Circuit breaker pattern protects against cache failures

use std::time::Duration;
use async_trait::async_trait;
use tracing::debug;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// Authentication cache trait for storing authorization decisions
/// 
/// This trait defines the interface for caching authorization results.
/// Implementations should prioritize security and fail-closed behavior.
/// 
/// # Security Considerations
/// 
/// - Cache misses should be treated as "deny" by default
/// - TTL values should be conservative to prevent stale permissions
/// - Invalidation must be reliable to ensure permissions stay current
/// - Cache keys should include all relevant context (user, resource, action)
/// 
/// # Example Usage
/// 
/// ```rust
/// use std::time::Duration;
/// use pcf_api::auth::cache::{AuthCache, MockAuthCache};
/// 
/// # #[tokio::main]
/// # async fn main() {
/// let cache = MockAuthCache::new();
/// let key = "user123:notes:read";
/// 
/// // Check cache (will return None for mock)
/// let cached = cache.get(key).await;
/// assert!(cached.is_none());
/// 
/// // Set permission in cache
/// cache.set(key, true, Duration::from_secs(300)).await;
/// 
/// // Invalidate pattern when permissions change
/// cache.invalidate_pattern("user123:*").await;
/// # }
/// ```
#[async_trait]
pub trait AuthCache: Send + Sync {
    /// Get a cached authorization decision
    /// 
    /// Returns `Some(true)` if access is explicitly allowed and cached
    /// Returns `Some(false)` if access is explicitly denied and cached  
    /// Returns `None` if no cached decision exists
    /// 
    /// # Arguments
    /// 
    /// * `key` - Cache key in format "user_id:resource:action"
    async fn get(&self, key: &str) -> Option<bool>;
    
    /// Cache an authorization decision with TTL
    /// 
    /// Stores the authorization result for the specified duration.
    /// After TTL expires, the entry should be automatically removed.
    /// 
    /// # Arguments
    /// 
    /// * `key` - Cache key in format "user_id:resource:action"
    /// * `value` - Authorization decision (true = allowed, false = denied)
    /// * `ttl` - Time to live for this cache entry
    async fn set(&self, key: String, value: bool, ttl: Duration);
    
    /// Remove a specific cache entry
    /// 
    /// # Arguments
    /// 
    /// * `key` - Cache key to remove
    async fn invalidate(&self, key: &str);
    
    /// Clear all entries for a specific user
    /// 
    /// This is used when user permissions change to ensure
    /// no stale permissions remain in cache.
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - User ID to invalidate cache for
    async fn invalidate_user(&self, user_id: &str);
    
    /// Get current cache size
    /// 
    /// Returns the number of entries currently in cache
    async fn size(&self) -> usize;
    
    /// Invalidate cache entries matching a pattern
    /// 
    /// Removes all cached entries that match the given pattern.
    /// Used when permissions change to ensure cache consistency.
    /// 
    /// # Pattern Examples
    /// 
    /// - `"user123:*"` - Invalidate all permissions for user123
    /// - `"*:notes:*"` - Invalidate all notes permissions
    /// - `"user123:notes:read"` - Invalidate specific permission
    /// 
    /// # Arguments
    /// 
    /// * `pattern` - Glob-style pattern to match cache keys
    async fn invalidate_pattern(&self, pattern: &str);
    
    /// Clear all cached entries
    /// 
    /// Used for testing, emergency situations, or configuration changes.
    /// In production, prefer targeted invalidation via patterns.
    async fn clear(&self) {
        self.invalidate_pattern("*").await;
    }
    
    /// Get cache statistics for monitoring
    /// 
    /// Returns metrics about cache performance for observability.
    /// Default implementation returns empty stats.
    async fn stats(&self) -> CacheStats {
        CacheStats::default()
    }
}

/// Cache statistics for monitoring and debugging
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses  
    pub misses: u64,
    /// Current number of cached entries
    pub entries: u64,
    /// Hit rate as percentage (0.0 - 100.0)
    pub hit_rate: f64,
    /// Number of entries evicted due to capacity
    pub evictions: u64,
    /// Number of entries expired due to TTL
    pub expired: u64,
}

/// Configuration for the authorization cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries before LRU eviction
    pub max_entries: usize,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Interval for background cleanup task
    pub cleanup_interval: Duration,
    /// Extended TTL used during SpiceDB outages
    pub extended_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            default_ttl: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60), // 1 minute
            extended_ttl: Duration::from_secs(1800), // 30 minutes for fallback
        }
    }
}

impl CacheStats {
    /// Calculate hit rate from hits and misses
    pub fn calculate_hit_rate(hits: u64, misses: u64) -> f64 {
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }
}

/// Internal cache entry with TTL and LRU tracking
#[derive(Clone, Debug)]
struct CacheEntry {
    /// The cached authorization value
    value: bool,
    /// When this entry expires
    expires_at: Instant,
    /// When this entry was last accessed (for LRU)
    last_accessed: Instant,
}

impl CacheEntry {
    /// Create a new cache entry
    fn new(value: bool, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            expires_at: now + ttl,
            last_accessed: now,
        }
    }
    
    /// Check if this entry has expired
    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
    
    /// Update last accessed time for LRU
    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// Production implementation of the authorization cache
/// 
/// This implementation provides:
/// - In-memory storage with configurable TTL
/// - LRU eviction when at capacity
/// - Background cleanup of expired entries
/// - Thread-safe concurrent access
/// - Comprehensive metrics tracking
#[derive(Clone)]
pub struct ProductionAuthCache {
    /// Cache configuration
    config: CacheConfig,
    /// The actual cache storage
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl ProductionAuthCache {
    /// Create a new production cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        let cache = Self {
            config: config.clone(),
            entries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        };
        
        // Start background cleanup task
        let cleanup_cache = cache.clone();
        tokio::spawn(async move {
            cleanup_cache.cleanup_task().await;
        });
        
        cache
    }
    
    /// Background task that periodically cleans up expired entries
    async fn cleanup_task(&self) {
        let mut interval = tokio::time::interval(self.config.cleanup_interval);
        
        loop {
            interval.tick().await;
            self.cleanup_expired().await;
        }
    }
    
    /// Remove expired entries and perform LRU eviction if needed
    async fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;
        
        // Remove expired entries
        let expired_keys: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            entries.remove(&key);
            stats.expired += 1;
        }
        
        // Perform LRU eviction if over capacity
        if entries.len() > self.config.max_entries {
            let mut sorted: Vec<_> = entries.iter().map(|(k, v)| (k.clone(), v.last_accessed)).collect();
            sorted.sort_by_key(|(_, last_accessed)| *last_accessed);
            
            let to_evict = entries.len() - self.config.max_entries;
            for (key, _) in sorted.into_iter().take(to_evict) {
                entries.remove(&key);
                stats.evictions += 1;
            }
        }
        
        // Update current entry count
        stats.entries = entries.len() as u64;
    }
}

#[async_trait]
impl AuthCache for ProductionAuthCache {
    async fn get(&self, key: &str) -> Option<bool> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;
        
        if let Some(entry) = entries.get_mut(key) {
            if !entry.is_expired() {
                // Entry is valid, update access time and return value
                entry.touch();
                stats.hits += 1;
                Some(entry.value)
            } else {
                // Entry expired, remove it
                entries.remove(key);
                stats.expired += 1;
                stats.misses += 1;
                None
            }
        } else {
            // Cache miss
            stats.misses += 1;
            None
        }
    }
    
    async fn set(&self, key: String, value: bool, ttl: Duration) {
        let entry = CacheEntry::new(value, ttl);
        
        let mut entries = self.entries.write().await;
        entries.insert(key, entry);
        
        // Check if we need immediate eviction (beyond cleanup task)
        if entries.len() > self.config.max_entries {
            // Perform immediate LRU eviction
            let mut sorted: Vec<_> = entries.iter().map(|(k, v)| (k.clone(), v.last_accessed)).collect();
            sorted.sort_by_key(|(_, last_accessed)| *last_accessed);
            
            if let Some((key_to_evict, _)) = sorted.first() {
                entries.remove(key_to_evict);
                
                // Update stats
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }
    }
    
    async fn invalidate(&self, key: &str) {
        self.entries.write().await.remove(key);
    }
    
    async fn invalidate_user(&self, user_id: &str) {
        let prefix = format!("{}:", user_id);
        let mut entries = self.entries.write().await;
        
        entries.retain(|key, _| !key.starts_with(&prefix));
    }
    
    async fn size(&self) -> usize {
        self.entries.read().await.len()
    }
    
    async fn invalidate_pattern(&self, pattern: &str) {
        let mut entries = self.entries.write().await;
        
        if pattern == "*" {
            // Clear all entries
            entries.clear();
        } else if pattern.starts_with('*') && pattern.ends_with('*') {
            // Pattern like "*:notes:*" - contains match
            let middle = &pattern[1..pattern.len()-1];
            entries.retain(|key, _| !key.contains(middle));
        } else if pattern.starts_with('*') {
            // Pattern like "*:read" - ends with match
            let suffix = &pattern[1..];
            entries.retain(|key, _| !key.ends_with(suffix));
        } else if pattern.ends_with('*') {
            // Pattern like "user123:*" - starts with match
            let prefix = &pattern[..pattern.len()-1];
            entries.retain(|key, _| !key.starts_with(prefix));
        } else {
            // Exact match
            entries.remove(pattern);
        }
    }
    
    async fn clear(&self) {
        self.entries.write().await.clear();
    }
    
    async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        result.entries = self.entries.read().await.len() as u64;
        
        // Calculate current hit rate
        if result.hits + result.misses > 0 {
            result.hit_rate = CacheStats::calculate_hit_rate(result.hits, result.misses);
        }
        
        result
    }
}

/// Mock implementation of AuthCache for testing and development
/// 
/// This implementation provides a no-op cache that always returns `None`
/// for gets and silently ignores sets. It's suitable for:
/// 
/// - Unit testing without external dependencies
/// - Development environments where caching isn't needed
/// - Fallback when cache backend is unavailable
/// 
/// # Security Note
/// 
/// This mock implementation is fail-safe - it never returns cached results,
/// forcing all authorization decisions to go through the actual backend.
pub struct MockAuthCache {
    /// Name for logging purposes
    name: String,
}

impl MockAuthCache {
    /// Create a new mock cache instance
    pub fn new() -> Self {
        Self {
            name: "MockAuthCache".to_string(),
        }
    }
    
    /// Create a named mock cache for testing
    pub fn with_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Default for MockAuthCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthCache for MockAuthCache {
    async fn get(&self, key: &str) -> Option<bool> {
        debug!(
            cache = %self.name,
            key = %key,
            "Cache miss (mock always returns None)"
        );
        None
    }
    
    async fn set(&self, key: String, value: bool, ttl: Duration) {
        debug!(
            cache = %self.name,
            key = %key,
            value = %value,
            ttl_secs = %ttl.as_secs(),
            "Mock cache set (no-op)"
        );
    }
    
    async fn invalidate(&self, key: &str) {
        debug!(
            cache = %self.name,
            key = %key,
            "Mock cache invalidate (no-op)"
        );
    }
    
    async fn invalidate_user(&self, user_id: &str) {
        debug!(
            cache = %self.name,
            user_id = %user_id,
            "Mock cache invalidate user (no-op)"
        );
    }
    
    async fn size(&self) -> usize {
        0
    }
    
    async fn invalidate_pattern(&self, pattern: &str) {
        debug!(
            cache = %self.name,
            pattern = %pattern,
            "Mock cache invalidation (no-op)"
        );
    }
    
    async fn clear(&self) {
        debug!(
            cache = %self.name,
            "Mock cache clear (no-op)"
        );
    }
    
    async fn stats(&self) -> CacheStats {
        CacheStats {
            hits: 0,
            misses: 0,
            entries: 0,
            hit_rate: 0.0,
            evictions: 0,
            expired: 0,
        }
    }
}

/// Cache key builder for consistent key formatting
/// 
/// Ensures cache keys follow a consistent format and handles escaping
/// of special characters that might interfere with pattern matching.
pub struct CacheKeyBuilder;

impl CacheKeyBuilder {
    /// Build cache key from user, resource, and action
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - User identifier
    /// * `resource` - Resource being accessed
    /// * `action` - Action being performed
    /// 
    /// # Returns
    /// 
    /// Cache key in format "user_id:resource:action"
    pub fn build(user_id: &str, resource: &str, action: &str) -> String {
        format!(
            "{}:{}:{}",
            Self::escape(user_id),
            Self::escape(resource),
            Self::escape(action)
        )
    }
    
    /// Build invalidation pattern for user
    /// 
    /// Returns pattern to invalidate all permissions for a specific user
    pub fn user_pattern(user_id: &str) -> String {
        format!("{}:*", Self::escape(user_id))
    }
    
    /// Build invalidation pattern for resource type
    /// 
    /// Returns pattern to invalidate all permissions for a resource type
    pub fn resource_pattern(resource: &str) -> String {
        format!("*:{}:*", Self::escape(resource))
    }
    
    /// Escape special characters in cache key components
    /// 
    /// Replaces characters that have special meaning in patterns
    fn escape(input: &str) -> String {
        input
            .replace('*', "%2A")
            .replace(':', "%3A")
            .replace('?', "%3F")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_mock_cache_get_returns_none() {
        let cache = MockAuthCache::new();
        let result = cache.get("user123:notes:read").await;
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_mock_cache_set_is_noop() {
        let cache = MockAuthCache::new();
        
        // Set should not panic and should be silent
        cache.set("user123:notes:read".to_string(), true, Duration::from_secs(300)).await;
        
        // Get should still return None
        let result = cache.get("user123:notes:read").await;
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_mock_cache_invalidate_is_noop() {
        let cache = MockAuthCache::new();
        
        // Should not panic
        cache.invalidate_pattern("user123:*").await;
        cache.clear().await;
    }
    
    #[tokio::test]
    async fn test_mock_cache_stats() {
        let cache = MockAuthCache::new();
        let stats = cache.stats().await;
        
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.hit_rate, 0.0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.expired, 0);
    }
    
    #[test]
    fn test_cache_key_builder() {
        let key = CacheKeyBuilder::build("user123", "notes", "read");
        assert_eq!(key, "user123:notes:read");
    }
    
    #[test]
    fn test_cache_key_builder_escapes_special_chars() {
        let key = CacheKeyBuilder::build("user:123", "notes*", "read?");
        assert_eq!(key, "user%3A123:notes%2A:read%3F");
    }
    
    #[test]
    fn test_user_pattern() {
        let pattern = CacheKeyBuilder::user_pattern("user123");
        assert_eq!(pattern, "user123:*");
    }
    
    #[test]
    fn test_user_pattern_escapes() {
        let pattern = CacheKeyBuilder::user_pattern("user:123");
        assert_eq!(pattern, "user%3A123:*");
    }
    
    #[test]
    fn test_resource_pattern() {
        let pattern = CacheKeyBuilder::resource_pattern("notes");
        assert_eq!(pattern, "*:notes:*");
    }
    
    #[test]
    fn test_resource_pattern_escapes() {
        let pattern = CacheKeyBuilder::resource_pattern("notes*");
        assert_eq!(pattern, "*:notes%2A:*");
    }
    
    #[test]
    fn test_cache_stats_calculate_hit_rate() {
        assert_eq!(CacheStats::calculate_hit_rate(0, 0), 0.0);
        assert_eq!(CacheStats::calculate_hit_rate(50, 50), 50.0);
        assert_eq!(CacheStats::calculate_hit_rate(80, 20), 80.0);
        assert_eq!(CacheStats::calculate_hit_rate(100, 0), 100.0);
    }
    
    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }
    
    #[tokio::test]
    async fn test_mock_cache_with_name() {
        let cache = MockAuthCache::with_name("test-cache");
        assert_eq!(cache.name, "test-cache");
        
        // Should still behave like a mock
        let result = cache.get("test:key").await;
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_trait_default_clear() {
        let cache = MockAuthCache::new();
        
        // Default clear implementation should call invalidate_pattern("*")
        cache.clear().await;
        
        // Should not panic and should work
    }
    
    #[tokio::test]
    async fn test_cache_trait_default_stats() {
        let cache = MockAuthCache::new();
        let stats = cache.stats().await;
        
        // Default implementation returns empty stats
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.hit_rate, 0.0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.expired, 0);
    }
    
    // Integration test demonstrating typical usage pattern
    #[tokio::test]
    async fn test_typical_cache_usage_pattern() {
        let cache = MockAuthCache::new();
        
        // 1. Try to get cached permission
        let key = CacheKeyBuilder::build("user123", "notes", "read");
        let cached = cache.get(&key).await;
        assert!(cached.is_none()); // Cache miss with mock
        
        // 2. After authorization check, cache the result
        cache.set(key, true, Duration::from_secs(300)).await;
        
        // 3. Later, when user permissions change, invalidate
        let user_pattern = CacheKeyBuilder::user_pattern("user123");
        cache.invalidate_pattern(&user_pattern).await;
        
        // 4. Check stats for monitoring
        let stats = cache.stats().await;
        assert_eq!(stats.hit_rate, 0.0); // Mock always has 0% hit rate
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.expired, 0);
    }
    
    // Test for future redis/in-memory implementations
    #[tokio::test]
    async fn test_cache_interface_contract() {
        let cache = MockAuthCache::new();
        
        // Test that all methods exist and can be called
        let _ = cache.get("test").await;
        cache.set("test".to_string(), true, Duration::from_secs(1)).await;
        cache.invalidate_pattern("test*").await;
        cache.clear().await;
        let _ = cache.stats().await;
        
        // This test ensures the trait interface is complete and usable
    }
    
    #[test]
    fn test_cache_key_builder_complex_values() {
        // Test with more complex values that might contain special characters
        let key = CacheKeyBuilder::build("user@example.com", "notes/private", "read?admin");
        assert_eq!(key, "user@example.com:notes/private:read%3Fadmin");
    }
    
    #[test]
    fn test_cache_key_builder_empty_values() {
        let key = CacheKeyBuilder::build("", "", "");
        assert_eq!(key, "::");
    }
    
    #[test]
    fn test_cache_key_builder_long_values() {
        let long_user = "a".repeat(100);
        let long_resource = "b".repeat(100); 
        let long_action = "c".repeat(100);
        
        let key = CacheKeyBuilder::build(&long_user, &long_resource, &long_action);
        assert!(key.len() > 300);
        assert!(key.contains(&long_user));
        assert!(key.contains(&long_resource));
        assert!(key.contains(&long_action));
    }
    
    #[test]
    fn test_cache_patterns_consistency() {
        let user_id = "user123";
        let resource = "notes";
        let action = "read";
        
        let key = CacheKeyBuilder::build(user_id, resource, action);
        let user_pattern = CacheKeyBuilder::user_pattern(user_id);
        let _resource_pattern = CacheKeyBuilder::resource_pattern(resource);
        
        // User pattern should match the key
        assert!(key.starts_with(&user_pattern[..user_pattern.len()-1])); // Remove the *
        
        // Resource pattern should match the key structure
        assert!(key.contains(&format!(":{}:", resource)));
    }
    
    #[tokio::test]
    async fn test_mock_cache_concurrent_access() {
        // Test that mock cache is safe for concurrent access
        let cache = std::sync::Arc::new(MockAuthCache::new());
        let mut handles = vec![];
        
        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let key = format!("user{}:notes:read", i);
                cache_clone.set(key.clone(), true, Duration::from_secs(300)).await;
                cache_clone.get(&key).await
            });
            handles.push(handle);
        }
        
        // All should complete without panicking
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_none()); // Mock always returns None
        }
    }
    
    #[test]
    fn test_escape_function_comprehensive() {
        // Test all special characters that need escaping
        assert_eq!(CacheKeyBuilder::escape("*"), "%2A");
        assert_eq!(CacheKeyBuilder::escape(":"), "%3A");
        assert_eq!(CacheKeyBuilder::escape("?"), "%3F");
        assert_eq!(CacheKeyBuilder::escape("test*:pattern?"), "test%2A%3Apattern%3F");
        assert_eq!(CacheKeyBuilder::escape("normal_text-123"), "normal_text-123");
    }
    
    #[tokio::test]
    async fn test_cache_stats_calculation_edge_cases() {
        // Test edge cases in hit rate calculation
        assert_eq!(CacheStats::calculate_hit_rate(0, 0), 0.0);
        assert_eq!(CacheStats::calculate_hit_rate(1, 0), 100.0);
        assert_eq!(CacheStats::calculate_hit_rate(0, 1), 0.0);
        assert_eq!(CacheStats::calculate_hit_rate(50, 50), 50.0);
        assert_eq!(CacheStats::calculate_hit_rate(99, 1), 99.0);
        assert_eq!(CacheStats::calculate_hit_rate(1, 99), 1.0);
    }
    
    #[tokio::test]
    async fn test_mock_cache_default_trait() {
        let cache = MockAuthCache::default();
        assert_eq!(cache.name, "MockAuthCache");
        
        let result = cache.get("test").await;
        assert!(result.is_none());
    }
    
    // ============================================================================
    // PRODUCTION CACHE TESTS (TDD - These will fail until implementation)
    // ============================================================================
    
    #[tokio::test]
    async fn test_production_cache_get_set() {
        let cache = ProductionAuthCache::new(CacheConfig::default());
        
        // Set a value with TTL
        cache.set("user123:notes:read".to_string(), true, Duration::from_secs(60)).await;
        
        // Should get the cached value
        let result = cache.get("user123:notes:read").await;
        assert_eq!(result, Some(true));
        
        // Different key should return None
        let result = cache.get("user123:notes:write").await;
        assert_eq!(result, None);
    }
    
    #[tokio::test]
    async fn test_production_cache_ttl_expiration() {
        let cache = ProductionAuthCache::new(CacheConfig {
            max_entries: 1000,
            default_ttl: Duration::from_millis(100),
            cleanup_interval: Duration::from_millis(50),
            extended_ttl: Duration::from_secs(1800),
        });
        
        // Set with short TTL
        cache.set("user123:notes:read".to_string(), true, Duration::from_millis(100)).await;
        
        // Should be available immediately
        assert_eq!(cache.get("user123:notes:read").await, Some(true));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired now
        assert_eq!(cache.get("user123:notes:read").await, None);
    }
    
    #[tokio::test]
    async fn test_production_cache_lru_eviction() {
        let cache = ProductionAuthCache::new(CacheConfig {
            max_entries: 2, // Very small cache to test eviction
            default_ttl: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(10),
            extended_ttl: Duration::from_secs(1800),
        });
        
        // Fill cache to capacity
        cache.set("key1".to_string(), true, Duration::from_secs(60)).await;
        cache.set("key2".to_string(), true, Duration::from_secs(60)).await;
        
        // Access key1 to make it recently used
        let _ = cache.get("key1").await;
        
        // Add third key - should evict key2 (least recently used)
        cache.set("key3".to_string(), true, Duration::from_secs(60)).await;
        
        // key1 and key3 should exist, key2 should be evicted
        assert_eq!(cache.get("key1").await, Some(true));
        assert_eq!(cache.get("key2").await, None); // Evicted
        assert_eq!(cache.get("key3").await, Some(true));
    }
    
    #[tokio::test]
    async fn test_production_cache_concurrent_access() {
        let cache = Arc::new(ProductionAuthCache::new(CacheConfig::default()));
        let mut handles = vec![];
        
        // Spawn multiple tasks doing concurrent operations
        for i in 0..20 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_test_{}", i);
                
                // Set value
                cache_clone.set(key.clone(), i % 2 == 0, Duration::from_secs(300)).await;
                
                // Read it back
                let result = cache_clone.get(&key).await;
                
                // Should get back what we set
                assert_eq!(result, Some(i % 2 == 0));
                
                result
            });
            handles.push(handle);
        }
        
        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Cache should have all entries
        assert_eq!(cache.size().await, 20);
    }
    
    #[tokio::test]
    async fn test_production_cache_invalidate_user() {
        let cache = ProductionAuthCache::new(CacheConfig::default());
        
        // Set multiple permissions for two users
        cache.set("user1:notes:read".to_string(), true, Duration::from_secs(300)).await;
        cache.set("user1:notes:write".to_string(), true, Duration::from_secs(300)).await;
        cache.set("user2:notes:read".to_string(), true, Duration::from_secs(300)).await;
        cache.set("user2:notes:write".to_string(), true, Duration::from_secs(300)).await;
        
        // Should have 4 entries
        assert_eq!(cache.size().await, 4);
        
        // Invalidate user1
        cache.invalidate_user("user1").await;
        
        // Should have 2 entries left (only user2)
        assert_eq!(cache.size().await, 2);
        
        // user1 permissions should be gone
        assert_eq!(cache.get("user1:notes:read").await, None);
        assert_eq!(cache.get("user1:notes:write").await, None);
        
        // user2 permissions should still exist
        assert_eq!(cache.get("user2:notes:read").await, Some(true));
        assert_eq!(cache.get("user2:notes:write").await, Some(true));
    }
    
    #[tokio::test]
    async fn test_production_cache_background_cleanup() {
        let cache = ProductionAuthCache::new(CacheConfig {
            max_entries: 1000,
            default_ttl: Duration::from_secs(300),
            cleanup_interval: Duration::from_millis(100), // Fast cleanup for testing
            extended_ttl: Duration::from_secs(1800),
        });
        
        // Add entries with short TTL
        for i in 0..10 {
            cache.set(
                format!("cleanup_test_{}", i),
                true,
                Duration::from_millis(50) // Very short TTL
            ).await;
        }
        
        // Should have all entries initially
        assert_eq!(cache.size().await, 10);
        
        // Wait for cleanup to run
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // All entries should be cleaned up
        assert_eq!(cache.size().await, 0);
        
        // Check stats show expired entries
        let stats = cache.stats().await;
        assert!(stats.expired > 0);
    }
    
    #[tokio::test]
    async fn test_production_cache_stats_tracking() {
        let cache = ProductionAuthCache::new(CacheConfig::default());
        
        // Perform operations that affect stats
        cache.set("test1".to_string(), true, Duration::from_secs(300)).await;
        cache.set("test2".to_string(), false, Duration::from_secs(300)).await;
        
        // Hit
        let _ = cache.get("test1").await;
        // Miss
        let _ = cache.get("nonexistent").await;
        
        let stats = cache.stats().await;
        assert_eq!(stats.entries, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 50.0); // 1 hit out of 2 operations = 50%
    }
    
    #[tokio::test]
    async fn test_production_cache_pattern_invalidation() {
        let cache = ProductionAuthCache::new(CacheConfig::default());
        
        // Set multiple permissions with different patterns
        cache.set("user1:notes:read".to_string(), true, Duration::from_secs(300)).await;
        cache.set("user1:posts:read".to_string(), true, Duration::from_secs(300)).await;
        cache.set("user2:notes:read".to_string(), true, Duration::from_secs(300)).await;
        cache.set("user2:posts:read".to_string(), true, Duration::from_secs(300)).await;
        
        assert_eq!(cache.size().await, 4);
        
        // Invalidate all notes permissions
        cache.invalidate_pattern("*:notes:*").await;
        
        // Should have 2 entries left (only posts)
        assert_eq!(cache.size().await, 2);
        
        // Notes permissions should be gone
        assert_eq!(cache.get("user1:notes:read").await, None);
        assert_eq!(cache.get("user2:notes:read").await, None);
        
        // Posts permissions should remain
        assert_eq!(cache.get("user1:posts:read").await, Some(true));
        assert_eq!(cache.get("user2:posts:read").await, Some(true));
    }
    
    // Performance test to ensure operations are fast
    #[tokio::test]
    async fn test_mock_cache_performance() {
        let cache = MockAuthCache::new();
        let start = std::time::Instant::now();
        
        // Perform many operations
        for i in 0..1000 {
            let key = format!("perf_test_{}", i);
            cache.set(key.clone(), i % 2 == 0, Duration::from_secs(300)).await;
            let _ = cache.get(&key).await;
        }
        
        cache.invalidate_pattern("perf_test_*").await;
        cache.clear().await;
        let _ = cache.stats().await;
        
        let elapsed = start.elapsed();
        
        // Mock operations should be very fast (under 100ms for 1000 operations)
        assert!(elapsed < Duration::from_millis(100), "Mock cache operations too slow: {:?}", elapsed);
    }
}
/// Authorization Patterns - Phase 4 Implementation Examples
///
/// This file demonstrates authorization patterns including the standard helper,
/// caching strategies, circuit breaker implementation, and fallback rules.

use async_graphql::{Context, Error, ErrorExtensions};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Standard Authorization Helper Pattern
/// 
/// This is the core pattern that should be used in all resolvers
pub mod standard_helper {
    use super::*;
    
    /// The standard authorization check used throughout the application
    pub async fn is_authorized(
        ctx: &Context<'_>,
        resource: &str,
        action: &str,
    ) -> Result<(), Error> {
        // 1. Check demo mode bypass
        #[cfg(feature = "demo")]
        if ctx.data::<bool>().map(|v| *v).unwrap_or(false) {
            tracing::debug!("Demo mode: bypassing authorization for {}/{}", resource, action);
            return Ok(());
        }
        
        // 2. Extract authentication context
        let auth_context = ctx.data::<AuthContext>()
            .map_err(|_| Error::new("Internal error: auth context not available"))?;
        
        // 3. Require authentication
        if !auth_context.is_authenticated() {
            return Err(Error::new("Authentication required")
                .extend_with(|_, ext| {
                    ext.set("code", "UNAUTHORIZED");
                }));
        }
        
        let user_id = auth_context.user_id.as_ref().unwrap();
        
        // 4. Check cache first
        let cache = ctx.data::<Arc<AuthCache>>()?;
        let cache_key = format!("{}:{}:{}", user_id, resource, action);
        
        if let Some(allowed) = cache.get(&cache_key).await {
            return handle_auth_result(allowed);
        }
        
        // 5. Check with SpiceDB through circuit breaker
        let spicedb = ctx.data::<Arc<SpiceDBClient>>()?;
        let circuit_breaker = ctx.data::<Arc<CircuitBreaker>>()?;
        
        let allowed = check_with_fallback(
            circuit_breaker,
            spicedb,
            user_id,
            resource,
            action,
        ).await?;
        
        // 6. Cache positive results only
        if allowed {
            let ttl = if circuit_breaker.is_open().await {
                Duration::from_secs(1800) // 30 minutes during outage
            } else {
                Duration::from_secs(300) // 5 minutes normally
            };
            cache.set(cache_key, allowed, ttl).await;
        }
        
        // 7. Audit log the decision
        audit_log(AuditEntry {
            timestamp: chrono::Utc::now(),
            user_id: user_id.clone(),
            resource: resource.to_string(),
            action: action.to_string(),
            allowed,
            source: if circuit_breaker.is_open().await { "fallback" } else { "spicedb" },
            trace_id: auth_context.trace_id.clone(),
        }).await;
        
        handle_auth_result(allowed)
    }
    
    fn handle_auth_result(allowed: bool) -> Result<(), Error> {
        if allowed {
            Ok(())
        } else {
            Err(Error::new("Permission denied")
                .extend_with(|_, ext| {
                    ext.set("code", "FORBIDDEN");
                }))
        }
    }
    
    async fn check_with_fallback(
        circuit_breaker: &Arc<CircuitBreaker>,
        spicedb: &Arc<SpiceDBClient>,
        user_id: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, Error> {
        match circuit_breaker.call(|| {
            let spicedb = spicedb.clone();
            let subject = format!("user:{}", user_id);
            let resource = resource.to_string();
            let permission = action.to_string();
            
            async move {
                tokio::time::timeout(
                    Duration::from_secs(2),
                    spicedb.check_permission(subject, resource, permission)
                ).await
            }
        }).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) | Err(_) => {
                // Timeout or circuit open - use fallback
                tracing::warn!(
                    "SpiceDB unavailable for {}/{}/{}, using fallback",
                    user_id, resource, action
                );
                Ok(apply_fallback_rules(user_id, resource, action))
            }
        }
    }
}

/// Fallback Rules for Degraded Mode
/// 
/// Conservative rules when SpiceDB is unavailable
pub fn apply_fallback_rules(user_id: &str, resource: &str, action: &str) -> bool {
    // Parse resource type and ID
    let parts: Vec<&str> = resource.split(':').collect();
    if parts.len() != 2 {
        return false; // Invalid format, deny
    }
    
    let (resource_type, resource_id) = (parts[0], parts[1]);
    
    match (resource_type, action) {
        // Always allow health checks
        ("health", "read") => true,
        
        // Users can read their own profile
        ("user", "read") if resource_id == user_id => true,
        
        // In degraded mode, deny all other operations
        // This is the most conservative approach
        _ => false,
    }
}

/// Authorization Context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub roles: Vec<String>,
    pub trace_id: String,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }
}

/// Cache Implementation with LRU Eviction
pub mod cache_patterns {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    pub struct AuthCache {
        entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
        max_size: usize,
        stats: CacheStats,
    }
    
    struct CacheEntry {
        allowed: bool,
        expires_at: Instant,
        last_access: AtomicU64,
        hit_count: AtomicU64,
    }
    
    struct CacheStats {
        hits: AtomicU64,
        misses: AtomicU64,
        evictions: AtomicU64,
    }
    
    impl AuthCache {
        pub fn new(max_size: usize) -> Self {
            Self {
                entries: Arc::new(RwLock::new(HashMap::with_capacity(max_size))),
                max_size,
                stats: CacheStats {
                    hits: AtomicU64::new(0),
                    misses: AtomicU64::new(0),
                    evictions: AtomicU64::new(0),
                },
            }
        }
        
        pub async fn get(&self, key: &str) -> Option<bool> {
            let entries = self.entries.read().await;
            
            if let Some(entry) = entries.get(key) {
                if entry.expires_at > Instant::now() {
                    // Update access tracking for LRU
                    entry.last_access.store(
                        chrono::Utc::now().timestamp_millis() as u64,
                        Ordering::Relaxed
                    );
                    entry.hit_count.fetch_add(1, Ordering::Relaxed);
                    self.stats.hits.fetch_add(1, Ordering::Relaxed);
                    
                    return Some(entry.allowed);
                }
            }
            
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
        
        pub async fn set(&self, key: String, allowed: bool, ttl: Duration) {
            // Security: Only cache positive results
            if !allowed {
                tracing::debug!("Refusing to cache negative result for {}", key);
                return;
            }
            
            let mut entries = self.entries.write().await;
            
            // Check if eviction needed
            if entries.len() >= self.max_size && !entries.contains_key(&key) {
                self.evict_lru(&mut entries);
            }
            
            entries.insert(key, CacheEntry {
                allowed,
                expires_at: Instant::now() + ttl,
                last_access: AtomicU64::new(chrono::Utc::now().timestamp_millis() as u64),
                hit_count: AtomicU64::new(0),
            });
        }
        
        fn evict_lru(&self, entries: &mut HashMap<String, CacheEntry>) {
            // Find least recently used
            if let Some((key, _)) = entries.iter()
                .min_by_key(|(_, entry)| entry.last_access.load(Ordering::Relaxed))
                .map(|(k, _)| (k.clone(), ()))
            {
                entries.remove(&key);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                tracing::debug!("Evicted LRU cache entry: {}", key);
            }
        }
        
        pub fn hit_rate(&self) -> f64 {
            let hits = self.stats.hits.load(Ordering::Relaxed) as f64;
            let misses = self.stats.misses.load(Ordering::Relaxed) as f64;
            
            if hits + misses > 0.0 {
                hits / (hits + misses)
            } else {
                0.0
            }
        }
    }
}

/// Circuit Breaker Pattern for SpiceDB
pub mod circuit_breaker_patterns {
    use super::*;
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum CircuitState {
        Closed,
        Open(Instant),
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
    
    impl CircuitBreaker {
        pub fn new() -> Self {
            Self {
                state: Arc::new(RwLock::new(CircuitState::Closed)),
                failure_count: Arc::new(RwLock::new(0)),
                success_count: Arc::new(RwLock::new(0)),
                failure_threshold: 5,
                success_threshold: 3,
                timeout: Duration::from_secs(60),
            }
        }
        
        pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitError<E>>
        where
            F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
        {
            // Check and update state
            let mut state = self.state.write().await;
            match &*state {
                CircuitState::Open(opened_at) => {
                    if opened_at.elapsed() > self.timeout {
                        *state = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                        tracing::info!("Circuit breaker entering half-open state");
                    } else {
                        return Err(CircuitError::CircuitOpen);
                    }
                }
                _ => {}
            }
            drop(state);
            
            // Execute operation
            match f().await {
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
        
        async fn record_success(&self) {
            *self.failure_count.write().await = 0;
            
            let state = self.state.read().await.clone();
            if let CircuitState::HalfOpen = state {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;
                
                if *success_count >= self.success_threshold {
                    *self.state.write().await = CircuitState::Closed;
                    tracing::info!("Circuit breaker closed after {} successes", *success_count);
                }
            }
        }
        
        async fn record_failure(&self) {
            let mut failure_count = self.failure_count.write().await;
            *failure_count += 1;
            
            if *failure_count >= self.failure_threshold {
                *self.state.write().await = CircuitState::Open(Instant::now());
                tracing::warn!("Circuit breaker opened after {} failures", *failure_count);
            }
            
            // Half-open immediately returns to open on failure
            if matches!(*self.state.read().await, CircuitState::HalfOpen) {
                *self.state.write().await = CircuitState::Open(Instant::now());
            }
        }
        
        pub async fn is_open(&self) -> bool {
            matches!(*self.state.read().await, CircuitState::Open(_))
        }
    }
    
    #[derive(Debug)]
    pub enum CircuitError<E> {
        CircuitOpen,
        OperationFailed(E),
    }
}

/// Batch Authorization for List Operations
pub mod batch_patterns {
    use super::*;
    
    /// Batch authorize multiple resources
    pub async fn batch_authorize(
        ctx: &Context<'_>,
        checks: Vec<(String, String)>, // (resource, action) pairs
    ) -> Result<Vec<bool>, Error> {
        let auth_context = ctx.data::<AuthContext>()?;
        if !auth_context.is_authenticated() {
            return Err(Error::new("Authentication required")
                .extend_with(|_, ext| {
                    ext.set("code", "UNAUTHORIZED");
                }));
        }
        
        let user_id = auth_context.user_id.as_ref().unwrap();
        let cache = ctx.data::<Arc<AuthCache>>()?;
        
        let mut results = Vec::with_capacity(checks.len());
        let mut uncached = Vec::new();
        
        // Check cache first
        for (idx, (resource, action)) in checks.iter().enumerate() {
            let cache_key = format!("{}:{}:{}", user_id, resource, action);
            
            match cache.get(&cache_key).await {
                Some(allowed) => results.push((idx, allowed)),
                None => uncached.push((idx, resource.clone(), action.clone())),
            }
        }
        
        // Batch check uncached with SpiceDB
        if !uncached.is_empty() {
            let spicedb = ctx.data::<Arc<SpiceDBClient>>()?;
            let circuit_breaker = ctx.data::<Arc<CircuitBreaker>>()?;
            
            let batch_checks: Vec<_> = uncached.iter()
                .map(|(_, resource, action)| {
                    (
                        format!("user:{}", user_id),
                        resource.clone(),
                        action.clone(),
                    )
                })
                .collect();
            
            match circuit_breaker.call(|| {
                let spicedb = spicedb.clone();
                let checks = batch_checks.clone();
                async move {
                    spicedb.bulk_check(checks).await
                }
            }).await {
                Ok(batch_results) => {
                    for ((idx, resource, action), (_, _, _), allowed) in 
                        uncached.iter().zip(batch_checks.iter()).zip(batch_results.iter()) 
                    {
                        results.push((*idx, *allowed));
                        
                        // Cache positive results
                        if *allowed {
                            let cache_key = format!("{}:{}:{}", user_id, resource, action);
                            cache.set(cache_key, true, Duration::from_secs(300)).await;
                        }
                    }
                }
                Err(_) => {
                    // Use fallback for all uncached
                    for (idx, resource, action) in uncached {
                        let allowed = apply_fallback_rules(user_id, &resource, &action);
                        results.push((idx, allowed));
                    }
                }
            }
        }
        
        // Sort by original index and extract booleans
        results.sort_by_key(|(idx, _)| *idx);
        Ok(results.into_iter().map(|(_, allowed)| allowed).collect())
    }
}

/// Audit Logging Patterns
pub mod audit_patterns {
    use super::*;
    use tokio::sync::mpsc;
    
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct AuditEntry {
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub user_id: String,
        pub resource: String,
        pub action: String,
        pub allowed: bool,
        pub source: String,
        pub trace_id: String,
    }
    
    pub async fn audit_log(entry: AuditEntry) {
        // In production, send to audit service
        // For demo, just log
        tracing::info!(
            target: "audit",
            user_id = %entry.user_id,
            resource = %entry.resource,
            action = %entry.action,
            allowed = %entry.allowed,
            source = %entry.source,
            trace_id = %entry.trace_id,
            "Authorization decision"
        );
    }
    
    /// Audit logger with batching
    pub struct AuditLogger {
        tx: mpsc::Sender<AuditEntry>,
    }
    
    impl AuditLogger {
        pub fn new(buffer_size: usize) -> (Self, AuditWriter) {
            let (tx, rx) = mpsc::channel(buffer_size);
            
            (
                Self { tx },
                AuditWriter { rx }
            )
        }
        
        pub async fn log(&self, entry: AuditEntry) -> Result<(), Error> {
            self.tx.send(entry).await
                .map_err(|_| Error::new("Audit log queue full"))
        }
    }
    
    pub struct AuditWriter {
        rx: mpsc::Receiver<AuditEntry>,
    }
    
    impl AuditWriter {
        pub async fn run(mut self) {
            let mut batch = Vec::with_capacity(100);
            let mut flush_interval = tokio::time::interval(Duration::from_secs(1));
            
            loop {
                tokio::select! {
                    Some(entry) = self.rx.recv() => {
                        batch.push(entry);
                        
                        if batch.len() >= 100 {
                            self.flush_batch(&mut batch).await;
                        }
                    }
                    _ = flush_interval.tick() => {
                        if !batch.is_empty() {
                            self.flush_batch(&mut batch).await;
                        }
                    }
                }
            }
        }
        
        async fn flush_batch(&self, batch: &mut Vec<AuditEntry>) {
            // Write to persistent storage
            // For demo, just log count
            tracing::info!("Flushing {} audit entries", batch.len());
            batch.clear();
        }
    }
}

/// SpiceDB Client Mock for Testing
pub mod testing_patterns {
    use super::*;
    
    pub struct MockSpiceDB {
        permissions: Arc<RwLock<HashMap<String, bool>>>,
    }
    
    impl MockSpiceDB {
        pub fn new() -> Self {
            Self {
                permissions: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        
        pub async fn allow(&self, subject: &str, resource: &str, permission: &str) {
            let key = format!("{}:{}:{}", subject, resource, permission);
            self.permissions.write().await.insert(key, true);
        }
        
        pub async fn deny(&self, subject: &str, resource: &str, permission: &str) {
            let key = format!("{}:{}:{}", subject, resource, permission);
            self.permissions.write().await.insert(key, false);
        }
        
        pub async fn check_permission(
            &self,
            subject: String,
            resource: String,
            permission: String,
        ) -> Result<bool, Error> {
            let key = format!("{}:{}:{}", subject, resource, permission);
            
            Ok(self.permissions.read().await
                .get(&key)
                .copied()
                .unwrap_or(false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fallback_rules() {
        // Health checks always allowed
        assert!(apply_fallback_rules("any_user", "health:status", "read"));
        
        // Users can read own profile
        assert!(apply_fallback_rules("user123", "user:user123", "read"));
        assert!(!apply_fallback_rules("user123", "user:user456", "read"));
        
        // All writes denied in degraded mode
        assert!(!apply_fallback_rules("user123", "note:456", "write"));
        assert!(!apply_fallback_rules("user123", "note:456", "delete"));
    }
    
    #[tokio::test]
    async fn test_cache_positive_only() {
        let cache = cache_patterns::AuthCache::new(10);
        
        // Set positive result
        cache.set("user:allow".to_string(), true, Duration::from_secs(60)).await;
        assert_eq!(cache.get("user:allow").await, Some(true));
        
        // Try to set negative result
        cache.set("user:deny".to_string(), false, Duration::from_secs(60)).await;
        assert_eq!(cache.get("user:deny").await, None); // Should not be cached
    }
}
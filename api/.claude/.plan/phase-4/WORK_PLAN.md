# Phase 4: Authorization & Authentication - Work Plan

## Prerequisites

Before starting Phase 4, ensure you have:
- **Completed Phases 1-3**: Server foundation, database layer, and GraphQL implementation operational
- **SpiceDB Knowledge**: Understanding of Zanzibar-style permission systems and SpiceDB basics
- **Caching Strategies**: Experience with distributed caching and cache invalidation
- **Security Best Practices**: Understanding of authentication flows, JWT tokens, and OWASP guidelines
- **Circuit Breaker Pattern**: Familiarity with resilience patterns for external service dependencies

## Quick Reference - Essential Resources

### Example Files
All example files are located in `/api/.claude/.spec/examples/`:
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Comprehensive test examples following TDD
- **[Authorization Patterns](../../.spec/examples/authorization-patterns.rs)** - Complete authorization implementation patterns
- **[Cache Strategies](../../.spec/examples/cache-strategies.rs)** - Caching with TTL and cleanup examples

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[authorization.md](../../.spec/authorization.md)** - Complete authorization specification with fallback strategies
- **[SPEC.md](../../SPEC.md)** - Authorization requirements (lines 118-134)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 4 objectives (lines 96-123)
- **[error-handling.md](../../.spec/error-handling.md)** - 401 vs 403 error distinctions

### Quick Links
- **Verification Script**: `scripts/verify-phase-4.sh`
- **Auth Test Suite**: `scripts/test-auth.sh`
- **SpiceDB Setup**: `scripts/setup-spicedb.sh`

## Overview
This work plan implements secure authorization with SpiceDB integration, focusing on resilience through caching, circuit breakers, and graceful degradation. The system must never fail due to authorization service unavailability. The implementation follows TDD practices with clear checkpoint boundaries.

## Build and Test Commands

Continue using `just` as the command runner:
- `just test` - Run all tests including auth tests
- `just test-auth` - Run only authorization-related tests
- `just spicedb-up` - Start local SpiceDB for testing
- `just spicedb-down` - Stop SpiceDB container
- `just auth-demo` - Run demo with auth bypass enabled
- `just clean` - Clean up processes and build artifacts

Always use these commands instead of direct cargo commands to ensure consistency.

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Continue following TDD practices from previous phases:
1. **Write tests FIRST** - Before any implementation
2. **Run tests to see them FAIL** - Confirms test is valid
3. **Write minimal code to make tests PASS** - No more than necessary
4. **REFACTOR** - Clean up while keeping tests green
5. **Document as you go** - Add rustdoc comments and inline explanations

## Done Criteria Checklist
- [ ] All endpoints require authorization (except health checks)
- [ ] SpiceDB permission checks working correctly
- [ ] Authorization caching reduces load (5 min TTL, positive results only)
- [ ] Proper 401 (unauthenticated) vs 403 (unauthorized) responses
- [ ] Demo mode bypass functional for testing
- [ ] Circuit breaker prevents cascade failures
- [ ] Fallback rules work during SpiceDB outage
- [ ] Audit logging for all authorization decisions
- [ ] Metrics track authorization performance
- [ ] No `.unwrap()` or `.expect()` in production code paths

## Work Breakdown with Review Checkpoints

### 4.1 Authorization Framework & Helper (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Core authorization abstraction and session handling
- **Scope**: Target 600-800 lines across 5-6 files (MUST document justification if outside range)
- **Key Components**: 
  - Standard is_authorized helper function (~200 lines)
  - Authentication context extraction (~150 lines)
  - Session management from headers (~150 lines)
  - Error types and responses (~100 lines)
  - Mock authorization for testing (~100 lines)
  - Audit logging interface (~100 lines)
- **Patterns**: Context propagation, error mapping, async authorization, audit trail

#### Task 4.1.1: Write Authorization Helper Tests First
Create `src/helpers/authorization.rs` with comprehensive test module. MUST write and run tests first to see them fail before implementing:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{Context, Value};
    
    #[tokio::test]
    async fn test_is_authorized_requires_authentication() {
        let ctx = create_test_context(None); // No auth
        let result = is_authorized(&ctx, "notes:123", "read").await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.extensions().get("code"), Some(&Value::from("UNAUTHORIZED")));
    }
    
    #[tokio::test]
    async fn test_is_authorized_with_cached_permission() {
        let auth = AuthContext {
            user_id: Some("user123".to_string()),
            trace_id: "trace456".to_string(),
            is_admin: false,
        };
        let ctx = create_test_context(Some(auth));
        
        // Pre-populate cache
        let cache = ctx.data::<Arc<AuthCache>>().unwrap();
        cache.set("user123:notes:123:read", true, Duration::from_secs(300)).await;
        
        let result = is_authorized(&ctx, "notes:123", "read").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_demo_mode_bypass() {
        let ctx = create_test_context_with_demo(true);
        let result = is_authorized(&ctx, "any:resource", "any_action").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_audit_logging() {
        let audit_log = Arc::new(MockAuditLog::new());
        let ctx = create_test_context_with_audit(audit_log.clone());
        
        let _ = is_authorized(&ctx, "notes:123", "read").await;
        
        let entries = audit_log.entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].resource, "notes:123");
        assert_eq!(entries[0].action, "read");
    }
}
```

#### Task 4.1.2: Define Authentication Context
Create the authentication context that flows through all requests:
```rust
// src/auth/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub trace_id: String,
    pub is_admin: bool,
    #[serde(skip)]
    pub session_token: Option<String>,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }
    
    pub fn require_auth(&self) -> Result<&str, Error> {
        self.user_id.as_deref().ok_or_else(|| {
            Error::new("Authentication required")
                .extend_with(|_, ext| ext.set("code", "UNAUTHORIZED"))
        })
    }
}

/// Extract authentication from request headers
pub async fn extract_auth_context(headers: &HeaderMap) -> AuthContext {
    let user_id = headers
        .get("x-user-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    
    let trace_id = headers
        .get("x-trace-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    let session_token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    
    AuthContext {
        user_id,
        trace_id,
        is_admin: false, // Will be determined by SpiceDB
        session_token,
    }
}
```

#### Task 4.1.3: Implement Standard Authorization Helper
Create the core authorization function following the specification:
```rust
// src/helpers/authorization.rs
use crate::auth::{AuthContext, AuthCache};
use crate::services::spicedb::SpiceDBClient;
use async_graphql::{Context, Error};
use std::sync::Arc;

/// Standard authorization check used throughout the application
/// 
/// This function implements the authorization flow:
/// 1. Check demo mode bypass (feature flag)
/// 2. Require authentication
/// 3. Check cache for existing permission
/// 4. Query SpiceDB through circuit breaker
/// 5. Apply fallback rules if SpiceDB unavailable
/// 6. Cache positive results only
/// 7. Audit log the decision
pub async fn is_authorized(
    ctx: &Context<'_>,
    resource: &str,
    action: &str,
) -> Result<(), Error> {
    // Demo mode bypass
    #[cfg(feature = "demo")]
    if ctx.data::<DemoMode>().map(|d| d.enabled).unwrap_or(false) {
        tracing::debug!(
            resource = %resource,
            action = %action,
            "Demo mode: bypassing authorization"
        );
        return Ok(());
    }
    
    // Extract authentication context
    let auth_context = ctx.data::<AuthContext>()
        .map_err(|_| Error::new("Internal error: auth context not available"))?;
    
    // Require authentication
    let user_id = auth_context.require_auth()?;
    
    // Check cache
    let cache = ctx.data::<Arc<AuthCache>>()?;
    let cache_key = format!("{}:{}:{}", user_id, resource, action);
    
    if let Some(allowed) = cache.get(&cache_key).await {
        tracing::debug!(
            user_id = %user_id,
            resource = %resource,
            action = %action,
            source = "cache",
            "Authorization decision from cache"
        );
        return if allowed {
            Ok(())
        } else {
            Err(Error::new("Permission denied")
                .extend_with(|_, ext| ext.set("code", "FORBIDDEN")))
        };
    }
    
    // Check with SpiceDB (implementation in checkpoint 3)
    let allowed = check_permission_with_fallback(ctx, user_id, resource, action).await?;
    
    // Cache positive results
    if allowed {
        let ttl = Duration::from_secs(300); // 5 minutes
        cache.set(cache_key, allowed, ttl).await;
    }
    
    // Audit log
    audit_authorization_decision(
        auth_context,
        resource,
        action,
        allowed,
        "spicedb", // Will be "fallback" when circuit breaker is open
    ).await;
    
    if allowed {
        Ok(())
    } else {
        Err(Error::new("Permission denied")
            .extend_with(|_, ext| ext.set("code", "FORBIDDEN")))
    }
}

// Stub for checkpoint 3
async fn check_permission_with_fallback(
    ctx: &Context<'_>,
    user_id: &str,
    resource: &str,
    action: &str,
) -> Result<bool, Error> {
    // Will be implemented in checkpoint 3
    Ok(false)
}
```

#### Task 4.1.4: Create Audit Logging
Implement audit logging for all authorization decisions:
```rust
// src/auth/audit.rs
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub trace_id: String,
    pub user_id: String,
    pub resource: String,
    pub action: String,
    pub allowed: bool,
    pub source: String, // "cache", "spicedb", "fallback"
    pub duration_ms: u64,
}

pub async fn audit_authorization_decision(
    auth: &AuthContext,
    resource: &str,
    action: &str,
    allowed: bool,
    source: &str,
) {
    let entry = AuditEntry {
        timestamp: Utc::now(),
        trace_id: auth.trace_id.clone(),
        user_id: auth.user_id.clone().unwrap_or_default(),
        resource: resource.to_string(),
        action: action.to_string(),
        allowed,
        source: source.to_string(),
        duration_ms: 0, // Will be calculated with timing
    };
    
    // Log as structured JSON
    tracing::info!(
        target: "audit",
        audit_type = "authorization",
        trace_id = %entry.trace_id,
        user_id = %entry.user_id,
        resource = %entry.resource,
        action = %entry.action,
        allowed = %entry.allowed,
        source = %entry.source,
        "Authorization decision"
    );
    
    // Future: Send to audit service
}
```

---
## 🛑 CHECKPOINT 1: Authorization Framework Complete

**WORKER CHECKPOINT ACTIONS:**
1. ✅ Complete all tasks in section 4.1
2. 📝 Self-verify your work:
   - [ ] All tests written first (TDD approach)
   - [ ] Tests fail before implementation
   - [ ] Minimal code to pass tests
   - [ ] Code refactored after green
   - [ ] Public APIs documented with rustdoc
   - [ ] Demo mode bypass works correctly
   - [ ] Audit logging captures all decisions
3. 🧹 Clean up your workspace:
   - [ ] Remove all debug statements
   - [ ] Delete temporary test files
   - [ ] Remove commented-out code
   - [ ] Fix all compiler warnings
   - [ ] Run `cargo fmt` and `cargo clippy`
4. 💾 Commit your work:
   ```bash
   git add .
   git commit -m "Checkpoint 1: Authorization framework complete"
   ```
5. ❓ Document questions/blockers:
   - Write to: `api/.claude/.reviews/checkpoint-1-questions.md`
6. 🛑 **STOP AND WAIT** for review approval

**DO NOT PROCEED TO SECTION 4.2**

---

### 4.2 Authorization Cache Implementation (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Thread-safe caching with TTL and cleanup
- **Scope**: Target 500-700 lines across 3-4 files (MUST document justification if outside range)
- **Key Components**:
  - Cache trait definition (~100 lines)
  - In-memory cache implementation (~250 lines)
  - Cache entry with TTL tracking (~100 lines)
  - Background cleanup task (~150 lines)
  - Cache metrics collection (~100 lines)
- **Required Algorithms**: LRU eviction, TTL expiration, concurrent access handling

#### Task 4.2.1: Write Cache Tests First
Create comprehensive cache tests:
```rust
#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_cache_get_set() {
        let cache = AuthCache::new(CacheConfig::default());
        
        cache.set("key1", true, Duration::from_secs(60)).await;
        let result = cache.get("key1").await;
        
        assert_eq!(result, Some(true));
    }
    
    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let cache = AuthCache::new(CacheConfig {
            default_ttl: Duration::from_millis(100),
            ..Default::default()
        });
        
        cache.set("key1", true, Duration::from_millis(100)).await;
        assert_eq!(cache.get("key1").await, Some(true));
        
        sleep(Duration::from_millis(150)).await;
        assert_eq!(cache.get("key1").await, None);
    }
    
    #[tokio::test]
    async fn test_cache_max_size_eviction() {
        let cache = AuthCache::new(CacheConfig {
            max_entries: 2,
            ..Default::default()
        });
        
        cache.set("key1", true, Duration::from_secs(60)).await;
        cache.set("key2", true, Duration::from_secs(60)).await;
        cache.set("key3", true, Duration::from_secs(60)).await;
        
        // key1 should be evicted (LRU)
        assert_eq!(cache.get("key1").await, None);
        assert_eq!(cache.get("key2").await, Some(true));
        assert_eq!(cache.get("key3").await, Some(true));
    }
    
    #[tokio::test]
    async fn test_cache_cleanup_task() {
        let cache = AuthCache::new(CacheConfig::default());
        
        // Add entries with short TTL
        for i in 0..10 {
            cache.set(
                &format!("key{}", i),
                true,
                Duration::from_millis(100)
            ).await;
        }
        
        assert_eq!(cache.size().await, 10);
        
        // Wait for cleanup
        sleep(Duration::from_millis(200)).await;
        cache.cleanup_expired().await;
        
        assert_eq!(cache.size().await, 0);
    }
}
```

#### Task 4.2.2: Define Cache Trait
Create the cache abstraction:
```rust
// src/auth/cache.rs
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait AuthorizationCache: Send + Sync {
    /// Get a cached authorization result
    async fn get(&self, key: &str) -> Option<bool>;
    
    /// Set an authorization result with TTL
    async fn set(&self, key: String, allowed: bool, ttl: Duration);
    
    /// Remove a specific entry
    async fn invalidate(&self, key: &str);
    
    /// Clear all entries for a user
    async fn invalidate_user(&self, user_id: &str);
    
    /// Get cache statistics
    async fn stats(&self) -> CacheStats;
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expired: u64,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub default_ttl: Duration,
    pub cleanup_interval: Duration,
    pub extended_ttl: Duration, // For degraded mode
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            default_ttl: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60),
            extended_ttl: Duration::from_secs(1800), // 30 minutes
        }
    }
}
```

#### Task 4.2.3: Implement In-Memory Cache
Create the cache implementation with TTL and LRU eviction:
```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

#[derive(Clone)]
struct CacheEntry {
    value: bool,
    expires_at: Instant,
    last_accessed: Instant,
}

pub struct AuthCache {
    config: CacheConfig,
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
}

impl AuthCache {
    pub fn new(config: CacheConfig) -> Self {
        let cache = Self {
            config: config.clone(),
            entries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        };
        
        // Start cleanup task
        let cleanup_cache = cache.clone();
        tokio::spawn(async move {
            cleanup_cache.cleanup_task().await;
        });
        
        cache
    }
    
    async fn cleanup_task(&self) {
        let mut interval = tokio::time::interval(self.config.cleanup_interval);
        
        loop {
            interval.tick().await;
            self.cleanup_expired().await;
        }
    }
    
    async fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;
        
        let expired_keys: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            entries.remove(&key);
            stats.expired += 1;
        }
        
        // LRU eviction if over capacity
        if entries.len() > self.config.max_entries {
            let mut sorted: Vec<_> = entries.iter().collect();
            sorted.sort_by_key(|(_, entry)| entry.last_accessed);
            
            let to_evict = entries.len() - self.config.max_entries;
            for (key, _) in sorted.into_iter().take(to_evict) {
                entries.remove(key);
                stats.evictions += 1;
            }
        }
    }
}

#[async_trait]
impl AuthorizationCache for AuthCache {
    async fn get(&self, key: &str) -> Option<bool> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;
        
        if let Some(entry) = entries.get_mut(key) {
            if entry.expires_at > Instant::now() {
                entry.last_accessed = Instant::now();
                stats.hits += 1;
                Some(entry.value)
            } else {
                entries.remove(key);
                stats.expired += 1;
                stats.misses += 1;
                None
            }
        } else {
            stats.misses += 1;
            None
        }
    }
    
    async fn set(&self, key: String, allowed: bool, ttl: Duration) {
        let entry = CacheEntry {
            value: allowed,
            expires_at: Instant::now() + ttl,
            last_accessed: Instant::now(),
        };
        
        self.entries.write().await.insert(key, entry);
    }
    
    async fn invalidate(&self, key: &str) {
        self.entries.write().await.remove(key);
    }
    
    async fn invalidate_user(&self, user_id: &str) {
        let prefix = format!("{}:", user_id);
        let mut entries = self.entries.write().await;
        
        entries.retain(|key, _| !key.starts_with(&prefix));
    }
    
    async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        result.total_entries = self.entries.read().await.len();
        result
    }
}
```

#### Task 4.2.4: Add Cache Metrics
Implement metrics collection for cache performance:
```rust
// src/auth/cache/metrics.rs
use prometheus::{IntGauge, IntCounterVec, HistogramVec};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref CACHE_SIZE: IntGauge = register_int_gauge!(
        "auth_cache_entries_total",
        "Total number of entries in authorization cache"
    ).unwrap();
    
    pub static ref CACHE_OPERATIONS: IntCounterVec = register_int_counter_vec!(
        "auth_cache_operations_total",
        "Authorization cache operations by type",
        &["operation", "result"]
    ).unwrap();
    
    pub static ref CACHE_HIT_RATE: IntGauge = register_int_gauge!(
        "auth_cache_hit_rate_percent",
        "Cache hit rate percentage"
    ).unwrap();
}

impl AuthCache {
    pub async fn update_metrics(&self) {
        let stats = self.stats().await;
        
        CACHE_SIZE.set(stats.total_entries as i64);
        
        let hit_rate = if stats.hits + stats.misses > 0 {
            (stats.hits * 100) / (stats.hits + stats.misses)
        } else {
            0
        };
        
        CACHE_HIT_RATE.set(hit_rate as i64);
    }
}
```

---
## 🛑 CHECKPOINT 2: Authorization Cache Complete

**WORKER CHECKPOINT ACTIONS:**
1. ✅ Complete all tasks in section 4.2
2. 📝 Self-verify your work:
   - [ ] All cache tests written first and passing
   - [ ] TTL expiration works correctly
   - [ ] LRU eviction when at capacity
   - [ ] Background cleanup task running
   - [ ] Thread-safe concurrent access
   - [ ] Metrics collection working
3. 🧹 Clean up your workspace:
   - [ ] No debug logs in production paths
   - [ ] No test data in cache
   - [ ] All TODOs addressed or tracked
   - [ ] Memory usage acceptable
4. 💾 Commit your work:
   ```bash
   git add .
   git commit -m "Checkpoint 2: Authorization cache complete"
   ```
5. ❓ Document questions/blockers:
   - Write to: `api/.claude/.reviews/checkpoint-2-questions.md`
6. 🛑 **STOP AND WAIT** for review approval

**DO NOT PROCEED TO SECTION 4.3**

---

### 4.3 SpiceDB Integration & Circuit Breaker (3-4 work units)

**Work Unit Context:**
- **Complexity**: High - External service integration with resilience patterns
- **Scope**: Target 800-1000 lines across 5-6 files (MUST document justification if outside range)
- **Key Components**:
  - SpiceDB client wrapper (~200 lines)
  - Circuit breaker implementation (~250 lines)
  - Fallback authorization rules (~150 lines)
  - Connection pool for gRPC (~150 lines)
  - Health check integration (~100 lines)
  - Retry logic with backoff (~150 lines)
- **Required Patterns**: Circuit breaker state machine, fallback strategies, connection pooling

#### Task 4.3.1: Write SpiceDB Integration Tests
Create tests for SpiceDB client and circuit breaker:
```rust
#[cfg(test)]
mod spicedb_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_spicedb_permission_check() {
        let client = SpiceDBClient::new(test_config());
        
        // Setup test permission
        client.write_relationship(Relationship {
            resource: "notes:123",
            relation: "owner",
            subject: "user:alice",
        }).await.unwrap();
        
        let allowed = client.check_permission(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:123",
            permission: "read",
        }).await.unwrap();
        
        assert!(allowed);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let client = MockSpiceDBClient::failing();
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            half_open_timeout: Duration::from_secs(5),
        });
        
        // Trigger failures
        for _ in 0..3 {
            let result = breaker.call(|| client.check_permission_mock()).await;
            assert!(result.is_err());
        }
        
        // Circuit should be open
        assert!(breaker.is_open().await);
        
        // Should return error immediately without calling service
        let start = Instant::now();
        let result = breaker.call(|| client.check_permission_mock()).await;
        assert!(result.is_err());
        assert!(start.elapsed() < Duration::from_millis(10));
    }
    
    #[tokio::test]
    async fn test_fallback_rules() {
        let fallback = FallbackAuthorizer::new();
        
        // Owner can read their own resources
        assert!(fallback.is_authorized("user:alice", "notes:alice:123", "read"));
        
        // Cannot read others' resources
        assert!(!fallback.is_authorized("user:alice", "notes:bob:456", "read"));
        
        // All writes denied in fallback
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "write"));
    }
}
```

#### Task 4.3.2: Implement SpiceDB Client
Create the SpiceDB client wrapper:
```rust
// src/services/spicedb/mod.rs
use authzed::api::v1::*;
use tonic::transport::Channel;
use std::sync::Arc;

#[derive(Clone)]
pub struct SpiceDBClient {
    client: Arc<PermissionsServiceClient<Channel>>,
    config: SpiceDBConfig,
}

#[derive(Clone, Debug)]
pub struct SpiceDBConfig {
    pub endpoint: String,
    pub preshared_key: String,
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
    pub max_connections: usize,
}

impl SpiceDBClient {
    pub async fn new(config: SpiceDBConfig) -> Result<Self, Error> {
        let endpoint = Channel::from_shared(config.endpoint.clone())
            .map_err(|e| Error::new(format!("Invalid endpoint: {}", e)))?
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .http2_adaptive_window(true);
        
        let channel = endpoint.connect_lazy();
        
        let mut client = PermissionsServiceClient::new(channel);
        
        // Add auth interceptor
        client = client.with_interceptor(move |mut req: tonic::Request<()>| {
            req.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", config.preshared_key).parse().unwrap(),
            );
            Ok(req)
        });
        
        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }
    
    pub async fn check_permission(&self, req: CheckPermissionRequest) -> Result<bool, Error> {
        let request = authzed::CheckPermissionRequest {
            resource: Some(ObjectReference {
                object_type: req.resource.split(':').next().unwrap().to_string(),
                object_id: req.resource.split(':').nth(1).unwrap().to_string(),
            }),
            permission: req.permission.to_string(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: req.subject.split(':').next().unwrap().to_string(),
                    object_id: req.subject.split(':').nth(1).unwrap().to_string(),
                }),
                optional_relation: None,
            }),
            consistency: None,
        };
        
        let response = self.client
            .clone()
            .check_permission(request)
            .await
            .map_err(|e| Error::new(format!("SpiceDB error: {}", e)))?;
        
        Ok(response.into_inner().permission_status == PermissionStatus::HasPermission as i32)
    }
    
    pub async fn health_check(&self) -> Result<bool, Error> {
        // Simple permission check as health indicator
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            self.check_permission(CheckPermissionRequest {
                subject: "user:health",
                resource: "system:health",
                permission: "check",
            })
        ).await;
        
        match result {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) => Ok(true), // SpiceDB is responding
            Err(_) => Ok(false), // Timeout
        }
    }
}

pub struct CheckPermissionRequest {
    pub subject: &'static str,
    pub resource: &'static str,
    pub permission: &'static str,
}
```

#### Task 4.3.3: Implement Circuit Breaker
Create the circuit breaker for resilience:
```rust
// src/middleware/circuit_breaker.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_timeout: Duration,
}

struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    last_state_change: Instant,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                last_state_change: Instant::now(),
            })),
        }
    }
    
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> futures::future::BoxFuture<'static, Result<T, E>>,
        E: std::fmt::Display,
    {
        // Check if we should attempt the call
        let should_attempt = {
            let mut state = self.state.write().await;
            match state.state {
                CircuitState::Closed => true,
                CircuitState::Open => {
                    // Check if we should transition to half-open
                    if let Some(last_failure) = state.last_failure_time {
                        if last_failure.elapsed() > self.config.half_open_timeout {
                            state.state = CircuitState::HalfOpen;
                            state.last_state_change = Instant::now();
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CircuitState::HalfOpen => true,
            }
        };
        
        if !should_attempt {
            return Err("Circuit breaker is open".into());
        }
        
        // Attempt the call
        let result = tokio::time::timeout(self.config.timeout, f()).await;
        
        // Update state based on result
        let mut state = self.state.write().await;
        match result {
            Ok(Ok(value)) => {
                self.on_success(&mut state).await;
                Ok(value)
            }
            Ok(Err(e)) => {
                self.on_failure(&mut state).await;
                Err(e)
            }
            Err(_) => {
                self.on_failure(&mut state).await;
                Err("Operation timed out".into())
            }
        }
    }
    
    async fn on_success(&self, state: &mut CircuitBreakerState) {
        match state.state {
            CircuitState::Closed => {
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    state.last_state_change = Instant::now();
                    tracing::info!("Circuit breaker closed");
                }
            }
            CircuitState::Open => {} // Shouldn't happen
        }
    }
    
    async fn on_failure(&self, state: &mut CircuitBreakerState) {
        state.last_failure_time = Some(Instant::now());
        
        match state.state {
            CircuitState::Closed => {
                state.failure_count += 1;
                if state.failure_count >= self.config.failure_threshold {
                    state.state = CircuitState::Open;
                    state.last_state_change = Instant::now();
                    tracing::warn!("Circuit breaker opened after {} failures", state.failure_count);
                }
            }
            CircuitState::HalfOpen => {
                state.state = CircuitState::Open;
                state.success_count = 0;
                state.last_state_change = Instant::now();
                tracing::warn!("Circuit breaker opened from half-open");
            }
            CircuitState::Open => {} // Already open
        }
    }
    
    pub async fn is_open(&self) -> bool {
        self.state.read().await.state == CircuitState::Open
    }
    
    pub async fn state(&self) -> CircuitState {
        self.state.read().await.state
    }
}
```

#### Task 4.3.4: Implement Fallback Authorization
Create fallback rules for when SpiceDB is unavailable:
```rust
// src/auth/fallback.rs
pub struct FallbackAuthorizer;

impl FallbackAuthorizer {
    pub fn new() -> Self {
        Self
    }
    
    /// Conservative fallback rules when SpiceDB is unavailable
    /// 
    /// Allowed:
    /// - Health checks (no resource)
    /// - Users reading their own resources
    /// - Public resources (if marked)
    /// 
    /// Denied:
    /// - All write operations
    /// - Cross-user access
    /// - Admin operations
    pub fn is_authorized(&self, subject: &str, resource: &str, action: &str) -> bool {
        // Health checks always allowed
        if resource.starts_with("system:health") {
            return true;
        }
        
        // Parse subject and resource
        let subject_parts: Vec<&str> = subject.split(':').collect();
        let resource_parts: Vec<&str> = resource.split(':').collect();
        
        if subject_parts.len() != 2 || resource_parts.len() < 2 {
            return false;
        }
        
        let user_id = subject_parts[1];
        let resource_type = resource_parts[0];
        
        // Only allow read operations in fallback
        if action != "read" && action != "list" {
            tracing::warn!(
                user_id = %user_id,
                resource = %resource,
                action = %action,
                "Fallback: Denying write operation"
            );
            return false;
        }
        
        // Check resource ownership
        match resource_type {
            "notes" => {
                // notes:user_id:note_id format
                if resource_parts.len() >= 3 && resource_parts[1] == user_id {
                    tracing::info!(
                        user_id = %user_id,
                        resource = %resource,
                        "Fallback: Allowing owner read"
                    );
                    true
                } else {
                    false
                }
            }
            "public" => {
                // Public resources allowed for read
                true
            }
            _ => {
                // Deny unknown resource types
                false
            }
        }
    }
}
```

#### Task 4.3.5: Wire Everything Together
Update the authorization helper to use SpiceDB with fallback:
```rust
// Update src/helpers/authorization.rs
async fn check_permission_with_fallback(
    ctx: &Context<'_>,
    user_id: &str,
    resource: &str,
    action: &str,
) -> Result<bool, Error> {
    let spicedb = ctx.data::<Arc<SpiceDBClient>>()?;
    let circuit_breaker = ctx.data::<Arc<CircuitBreaker>>()?;
    let fallback = ctx.data::<Arc<FallbackAuthorizer>>()?;
    
    let subject = format!("user:{}", user_id);
    
    // Try SpiceDB through circuit breaker
    let result = circuit_breaker.call(|| {
        let spicedb = spicedb.clone();
        let subject = subject.clone();
        let resource = resource.to_string();
        let action = action.to_string();
        
        Box::pin(async move {
            spicedb.check_permission(CheckPermissionRequest {
                subject: &subject,
                resource: &resource,
                permission: &action,
            }).await
        })
    }).await;
    
    match result {
        Ok(allowed) => {
            tracing::debug!(
                user_id = %user_id,
                resource = %resource,
                action = %action,
                allowed = %allowed,
                source = "spicedb",
                "Authorization decision"
            );
            Ok(allowed)
        }
        Err(e) => {
            // Use fallback rules
            tracing::warn!(
                user_id = %user_id,
                resource = %resource,
                action = %action,
                error = %e,
                "SpiceDB unavailable, using fallback"
            );
            
            let allowed = fallback.is_authorized(&subject, resource, action);
            
            // Extend cache TTL during outage
            if allowed {
                let cache = ctx.data::<Arc<AuthCache>>()?;
                let cache_key = format!("{}:{}:{}", user_id, resource, action);
                cache.set(cache_key, allowed, Duration::from_secs(1800)).await; // 30 min
            }
            
            Ok(allowed)
        }
    }
}
```

---
## 🛑 CHECKPOINT 3: SpiceDB Integration Complete

**WORKER CHECKPOINT ACTIONS:**
1. ✅ Complete all tasks in section 4.3
2. 📝 Self-verify your work:
   - [ ] SpiceDB client connects and checks permissions
   - [ ] Circuit breaker opens after failures
   - [ ] Circuit breaker transitions to half-open
   - [ ] Fallback rules are conservative
   - [ ] Extended cache TTL during outages
   - [ ] Health check integration works
3. 🧹 Clean up your workspace:
   - [ ] No hardcoded endpoints or credentials
   - [ ] No test permissions in SpiceDB
   - [ ] Error messages don't leak details
   - [ ] Connection pool sized appropriately
4. 💾 Commit your work:
   ```bash
   git add .
   git commit -m "Checkpoint 3: SpiceDB integration complete"
   ```
5. ❓ Document questions/blockers:
   - Write to: `api/.claude/.reviews/checkpoint-3-questions.md`
6. 🛑 **STOP AND WAIT** for review approval

**DO NOT PROCEED TO SECTION 4.4**

---

### 4.4 Complete Integration & Testing (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Full system integration and comprehensive testing
- **Scope**: Target 600-800 lines of tests and integration
- **Key Components**:
  - GraphQL resolver integration (~200 lines)
  - End-to-end authorization tests (~300 lines)
  - Demo mode configuration (~100 lines)
  - Verification scripts (~100 lines)
  - Performance benchmarks (~100 lines)
- **Patterns**: Integration testing, performance testing, demo mode setup

#### Task 4.4.1: Integrate with GraphQL Resolvers
Update all GraphQL resolvers to use authorization:
```rust
// src/graphql/resolvers/queries.rs
use crate::helpers::authorization::is_authorized;

#[Object]
impl Query {
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        // Authorize first
        is_authorized(ctx, &format!("notes:{}", id), "read").await?;
        
        let database = ctx.data::<Arc<dyn DatabaseService>>()?;
        let note = database
            .read("notes", &id.to_string())
            .await
            .map_err(|e| e.into())?;
        
        Ok(note.map(|data| serde_json::from_value(data).unwrap()))
    }
    
    async fn notes(
        &self,
        ctx: &Context<'_>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Connection<String, Note>> {
        // List authorization
        is_authorized(ctx, "notes:*", "list").await?;
        
        // Existing pagination logic...
    }
}

// src/graphql/resolvers/mutations.rs
#[Object]
impl Mutation {
    async fn create_note(
        &self,
        ctx: &Context<'_>,
        input: CreateNoteInput,
    ) -> Result<Note> {
        // Check create permission on collection
        is_authorized(ctx, "notes:*", "create").await?;
        
        let auth = ctx.data::<AuthContext>()?;
        let user_id = auth.user_id.as_ref().unwrap();
        
        // Create note with user as owner
        let note = Note {
            id: None,
            title: input.title,
            content: input.content,
            author: user_id.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: input.tags.unwrap_or_default(),
        };
        
        // Validate and save...
    }
    
    async fn update_note(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateNoteInput,
    ) -> Result<Note> {
        // Check update permission on specific note
        is_authorized(ctx, &format!("notes:{}", id), "update").await?;
        
        // Update logic...
    }
    
    async fn delete_note(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        // Check delete permission
        is_authorized(ctx, &format!("notes:{}", id), "delete").await?;
        
        // Delete logic...
    }
}
```

#### Task 4.4.2: Create Integration Tests
Comprehensive end-to-end tests:
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_full_authorization_flow() {
        let app = create_test_app().await;
        
        // Create test user and permissions in SpiceDB
        setup_test_permissions().await;
        
        // Test authenticated request
        let response = app
            .graphql_request(
                r#"
                query {
                    note(id: "notes:123") {
                        id
                        title
                        author
                    }
                }
                "#,
            )
            .header("x-user-id", "alice")
            .send()
            .await;
        
        assert!(response.status().is_success());
        let json: serde_json::Value = response.json().await;
        assert!(json["data"]["note"].is_object());
    }
    
    #[tokio::test]
    async fn test_unauthorized_returns_401() {
        let app = create_test_app().await;
        
        // No auth header
        let response = app
            .graphql_request(
                r#"query { notes { edges { node { id } } } }"#,
            )
            .send()
            .await;
        
        let json: serde_json::Value = response.json().await;
        assert_eq!(
            json["errors"][0]["extensions"]["code"],
            "UNAUTHORIZED"
        );
    }
    
    #[tokio::test]
    async fn test_forbidden_returns_403() {
        let app = create_test_app().await;
        
        // Alice trying to update Bob's note
        let response = app
            .graphql_request(
                r#"
                mutation {
                    updateNote(id: "notes:bob:456", input: {
                        title: "Hacked!"
                    }) {
                        id
                    }
                }
                "#,
            )
            .header("x-user-id", "alice")
            .send()
            .await;
        
        let json: serde_json::Value = response.json().await;
        assert_eq!(
            json["errors"][0]["extensions"]["code"],
            "FORBIDDEN"
        );
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_fallback() {
        let app = create_test_app_without_spicedb().await;
        
        // Should use fallback rules
        let response = app
            .graphql_request(
                r#"
                query {
                    note(id: "notes:alice:123") {
                        id
                    }
                }
                "#,
            )
            .header("x-user-id", "alice")
            .send()
            .await;
        
        // Should succeed with fallback (owner reading own note)
        assert!(response.status().is_success());
    }
    
    #[tokio::test]
    async fn test_demo_mode_bypass() {
        let app = create_test_app_demo_mode().await;
        
        // Any request should work in demo mode
        let response = app
            .graphql_request(
                r#"
                mutation {
                    createNote(input: {
                        title: "Demo Note"
                        content: "No auth needed"
                    }) {
                        id
                    }
                }
                "#,
            )
            .send()
            .await;
        
        assert!(response.status().is_success());
    }
}
```

#### Task 4.4.3: Add Demo Mode Configuration
Implement demo mode bypass:
```rust
// src/config.rs
#[derive(Debug, Deserialize, Validate)]
pub struct AuthConfig {
    #[cfg(feature = "demo")]
    pub demo_mode: bool,
    
    pub cache: CacheConfig,
    pub spicedb: SpiceDBConfig,
    pub circuit_breaker: CircuitBreakerConfig,
}

// src/main.rs - in server setup
#[cfg(feature = "demo")]
if config.auth.demo_mode {
    tracing::warn!("🚨 DEMO MODE ENABLED - Authorization bypassed!");
    schema = schema.data(DemoMode { enabled: true });
}

// Demo mode check
#[cfg(not(feature = "demo"))]
compile_error!("Demo mode must be enabled with --features demo for development");
```

#### Task 4.4.4: Create Verification Script
Add comprehensive verification:
```bash
#!/bin/bash
# scripts/verify-phase-4.sh
set -e

echo "=== Phase 4 Authorization Verification ==="

# Check compilation
echo "✓ Checking compilation..."
cargo check --no-default-features
cargo check --features demo

# Start SpiceDB
echo "✓ Starting SpiceDB..."
just spicedb-up

# Wait for SpiceDB
sleep 5

# Run unit tests
echo "✓ Running unit tests..."
cargo test auth::

# Run integration tests
echo "✓ Running integration tests..."
cargo test --test authorization_integration

# Test circuit breaker
echo "✓ Testing circuit breaker..."
./scripts/test-circuit-breaker.sh

# Test cache performance
echo "✓ Testing cache performance..."
cargo bench auth_cache

# Test demo mode
echo "✓ Testing demo mode..."
cargo run --features demo --example demo_auth

# Cleanup
just spicedb-down

echo "=== All Phase 4 verification passed! ==="
```

#### Task 4.4.5: Performance Benchmarks
Add authorization performance tests:
```rust
// benches/auth_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_authorization(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("auth_cache_hit", |b| {
        b.to_async(&runtime).iter(|| async {
            let ctx = create_bench_context_with_cache();
            is_authorized(&ctx, black_box("notes:123"), black_box("read")).await
        });
    });
    
    c.bench_function("auth_spicedb_check", |b| {
        b.to_async(&runtime).iter(|| async {
            let ctx = create_bench_context_no_cache();
            is_authorized(&ctx, black_box("notes:456"), black_box("write")).await
        });
    });
    
    c.bench_function("auth_fallback", |b| {
        b.iter(|| {
            let fallback = FallbackAuthorizer::new();
            fallback.is_authorized(
                black_box("user:alice"),
                black_box("notes:alice:789"),
                black_box("read")
            )
        });
    });
}

criterion_group!(benches, benchmark_authorization);
criterion_main!(benches);
```

---
## 🛑 CHECKPOINT 4: Complete Integration Verified

**WORKER CHECKPOINT ACTIONS:**
1. ✅ Complete all tasks in section 4.4
2. 📝 Self-verify your work:
   - [ ] All GraphQL resolvers use authorization
   - [ ] Integration tests cover all scenarios
   - [ ] Demo mode works as expected
   - [ ] Performance benchmarks acceptable
   - [ ] Verification script passes
   - [ ] No authorization bypasses
3. 🧹 Final cleanup:
   - [ ] Remove all test data
   - [ ] No debug configurations
   - [ ] Documentation updated
   - [ ] All features tested
   - [ ] Production ready
4. 💾 Commit your work:
   ```bash
   git add .
   git commit -m "Checkpoint 4: Phase 4 complete - Authorization implemented"
   ```
5. ❓ Document questions/blockers:
   - Write to: `api/.claude/.reviews/checkpoint-4-questions.md`
6. 🛑 **STOP AND WAIT** for final review

**Phase 4 Complete!**

---

## Summary

Phase 4 implements a robust authorization system with:
- Standard `is_authorized` helper used everywhere
- Positive-only caching with 5-minute TTL
- SpiceDB integration with circuit breaker
- Conservative fallback rules during outages
- Comprehensive audit logging
- Demo mode for development
- Full test coverage and benchmarks

The system ensures the API never fails due to authorization service unavailability while maintaining security through conservative fallback rules.
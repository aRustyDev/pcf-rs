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
- **[Authorization Patterns](../../.spec/examples/authorization-patterns.rs)** - Authorization implementation patterns (to be created)
- **[Cache Strategies](../../.spec/examples/cache-strategies.rs)** - Caching with fallback examples (to be created)

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[authorization.md](../../.spec/authorization.md)** - Complete authorization specification
- **[SPEC.md](../../SPEC.md)** - Authorization requirements (lines 44-53)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 4 objectives (lines 98-124)

### Quick Links
- **Verification Script**: `scripts/verify-phase-4.sh` (to be created)
- **Auth Test Suite**: `scripts/test-auth.sh` (to be created)
- **SpiceDB Setup**: `scripts/setup-spicedb.sh` (to be created)

## Overview
This work plan implements secure authorization with SpiceDB integration, focusing on resilience through caching, circuit breakers, and graceful degradation. The system must never fail due to authorization service unavailability. Each checkpoint represents a natural boundary for review.

## Build and Test Commands

Continue using `just` as the command runner:
- `just test` - Run all tests including auth tests
- `just test-auth` - Run only authorization-related tests
- `just spicedb-up` - Start local SpiceDB for testing
- `just spicedb-down` - Stop SpiceDB container

Always use these commands instead of direct cargo commands to ensure consistency.

## IMPORTANT: Review Process

**This plan includes 4 mandatory review checkpoints where work MUST stop for external review.**

At each checkpoint:
1. **STOP all work** and commit your code
2. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
3. **Wait for approval** before continuing to next section

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
- [ ] Authorization caching reduces load
- [ ] Proper 401 (unauthenticated) vs 403 (unauthorized) responses
- [ ] Demo mode bypass functional for testing
- [ ] Circuit breaker prevents cascade failures
- [ ] Fallback rules work during SpiceDB outage
- [ ] Audit logging for all authorization decisions
- [ ] Metrics track authorization performance

## Work Breakdown with Review Checkpoints

### 4.1 Authorization Framework & Helper (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Core authorization abstraction
- **Scope**: ~500 lines across 4-5 files
- **Key Components**: 
  - Standard is_authorized helper function (~200 lines)
  - Authorization context types (~100 lines)
  - Error types and responses (~100 lines)
  - Session extraction from headers (~100 lines)
  - Mock authorization for testing (~100 lines)
- **Patterns**: Context propagation, error mapping, async authorization

#### Task 4.1.1: Write Authorization Helper Tests First
Create `src/helpers/authorization.rs` with comprehensive test module:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Context;
    
    #[tokio::test]
    async fn test_is_authorized_requires_authentication() {
        let ctx = create_test_context(None); // No auth
        
        let result = is_authorized(&ctx, "note:123", "read").await;
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert_eq!(err.extensions.get("code"), Some(&"UNAUTHORIZED".into()));
    }
    
    #[tokio::test]
    async fn test_is_authorized_with_valid_user() {
        let ctx = create_test_context(Some("user123"));
        let mock_spicedb = MockSpiceDB::new();
        mock_spicedb.allow("user:user123", "note:123", "read");
        ctx.insert(mock_spicedb);
        
        let result = is_authorized(&ctx, "note:123", "read").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_is_authorized_denies_unauthorized() {
        let ctx = create_test_context(Some("user123"));
        let mock_spicedb = MockSpiceDB::new();
        mock_spicedb.deny("user:user123", "note:123", "write");
        ctx.insert(mock_spicedb);
        
        let result = is_authorized(&ctx, "note:123", "write").await;
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert_eq!(err.extensions.get("code"), Some(&"FORBIDDEN".into()));
    }
    
    #[tokio::test]
    async fn test_demo_mode_bypass() {
        std::env::set_var("DEMO_MODE", "true");
        let ctx = create_test_context(None); // No auth in demo
        
        let result = is_authorized(&ctx, "note:123", "write").await;
        assert!(result.is_ok()); // Should pass in demo mode
        
        std::env::remove_var("DEMO_MODE");
    }
}
```

#### Task 4.1.2: Define Authorization Context
Create types for authorization state:
```rust
// src/auth/context.rs
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub roles: Vec<String>,
    pub trace_id: String,
}

impl AuthContext {
    pub fn anonymous(trace_id: String) -> Self {
        Self {
            user_id: None,
            session_id: None,
            roles: vec![],
            trace_id,
        }
    }
    
    pub fn authenticated(user_id: String, session_id: String, trace_id: String) -> Self {
        Self {
            user_id: Some(user_id),
            session_id: Some(session_id),
            roles: vec!["authenticated".to_string()],
            trace_id,
        }
    }
    
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }
}

// Resource formatting utilities
pub fn format_resource(resource_type: &str, resource_id: &str) -> String {
    format!("{}:{}", resource_type, resource_id)
}

pub fn parse_resource(resource: &str) -> Result<(&str, &str), AuthError> {
    let parts: Vec<&str> = resource.split(':').collect();
    if parts.len() != 2 {
        return Err(AuthError::InvalidResourceFormat(resource.to_string()));
    }
    Ok((parts[0], parts[1]))
}
```

#### Task 4.1.3: Implement Standard Authorization Helper
Create the core authorization function:
```rust
// src/helpers/authorization.rs
use crate::auth::{AuthContext, AuthError};
use crate::services::spicedb::SpiceDBClient;
use async_graphql::{Context, Error, ErrorExtensions};
use std::time::Duration;
use tokio::time::timeout;

/// Standard authorization check for all operations
pub async fn is_authorized(
    ctx: &Context<'_>,
    resource: &str,
    action: &str,
) -> Result<(), Error> {
    // Demo mode bypass
    #[cfg(feature = "demo")]
    if std::env::var("DEMO_MODE").unwrap_or_default() == "true" {
        tracing::debug!("Demo mode: bypassing authorization");
        return Ok(());
    }
    
    // Extract auth context
    let auth = ctx.data::<AuthContext>()
        .map_err(|_| Error::new("Authorization context not available"))?;
    
    // Require authentication
    if !auth.is_authenticated() {
        return Err(Error::new("Authentication required")
            .extend_with(|_, ext| {
                ext.set("code", "UNAUTHORIZED");
            }));
    }
    
    let user_id = auth.user_id.as_ref().unwrap();
    
    // Check cache first
    let cache = ctx.data::<Arc<AuthCache>>()?;
    let cache_key = format!("{}:{}:{}", user_id, resource, action);
    
    if let Some(allowed) = cache.get(&cache_key).await {
        tracing::debug!(
            user_id = %user_id,
            resource = %resource,
            action = %action,
            allowed = %allowed,
            "Authorization result from cache"
        );
        
        return if allowed {
            Ok(())
        } else {
            Err(Error::new("Permission denied")
                .extend_with(|_, ext| {
                    ext.set("code", "FORBIDDEN");
                }))
        };
    }
    
    // Query SpiceDB
    let spicedb = ctx.data::<Arc<SpiceDBClient>>()?;
    let circuit_breaker = ctx.data::<Arc<CircuitBreaker>>()?;
    
    let allowed = match circuit_breaker.call(|| {
        let spicedb = spicedb.clone();
        let subject = format!("user:{}", user_id);
        let resource = resource.to_string();
        let permission = action.to_string();
        
        async move {
            timeout(
                Duration::from_secs(2),
                spicedb.check_permission(subject, resource, permission)
            ).await
        }
    }).await {
        Ok(Ok(result)) => {
            metrics::AUTH_CHECKS.with_label_values(&["success"]).inc();
            result
        }
        Ok(Err(_)) => {
            // Timeout - use fallback
            tracing::warn!(
                user_id = %user_id,
                resource = %resource,
                action = %action,
                "SpiceDB timeout, using fallback rules"
            );
            metrics::AUTH_CHECKS.with_label_values(&["timeout"]).inc();
            apply_fallback_rules(user_id, resource, action)
        }
        Err(_) => {
            // Circuit breaker open - use fallback
            tracing::warn!(
                user_id = %user_id,
                resource = %resource,
                action = %action,
                "SpiceDB unavailable, using fallback rules"
            );
            metrics::AUTH_CHECKS.with_label_values(&["circuit_open"]).inc();
            apply_fallback_rules(user_id, resource, action)
        }
    };
    
    // Cache positive results only
    if allowed {
        let ttl = if circuit_breaker.is_open() {
            Duration::from_secs(1800) // 30 minutes during outage
        } else {
            Duration::from_secs(300) // 5 minutes normally
        };
        
        cache.set(cache_key, allowed, ttl).await;
    }
    
    // Audit log
    audit_log(AuditEntry {
        timestamp: chrono::Utc::now(),
        user_id: user_id.clone(),
        resource: resource.to_string(),
        action: action.to_string(),
        allowed,
        source: if circuit_breaker.is_open() { "fallback" } else { "spicedb" },
        trace_id: auth.trace_id.clone(),
    }).await;
    
    // Return result
    if allowed {
        Ok(())
    } else {
        Err(Error::new("Permission denied")
            .extend_with(|_, ext| {
                ext.set("code", "FORBIDDEN");
            }))
    }
}

/// Conservative fallback rules for degraded mode
fn apply_fallback_rules(user_id: &str, resource: &str, action: &str) -> bool {
    match parse_resource(resource) {
        Ok(("health", _)) if action == "read" => true, // Always allow health checks
        Ok(("user", id)) if id == user_id && action == "read" => true, // Users can read own profile
        Ok((_, _)) if action == "read" => false, // Deny other reads without cache evidence
        _ => false, // Deny all writes and unknown operations
    }
}
```

#### Task 4.1.4: Extract Session from Headers
Implement session extraction middleware:
```rust
// src/auth/session.rs
use axum::http::HeaderMap;
use jsonwebtoken::{decode, DecodingKey, Validation};

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub sid: String, // session_id
    pub exp: i64,    // expiration
    pub roles: Vec<String>,
}

pub async fn extract_session(headers: &HeaderMap) -> Result<AuthContext, AuthError> {
    // Check for Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingAuthHeader)?;
    
    // Extract Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidAuthHeader)?;
    
    // Decode JWT (in production, verify signature)
    #[cfg(not(feature = "demo"))]
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(get_jwt_secret().as_ref()),
        &Validation::default()
    )
    .map_err(|e| AuthError::InvalidToken(e.to_string()))?
    .claims;
    
    #[cfg(feature = "demo")]
    let claims = decode_demo_token(token)?;
    
    // Build auth context
    Ok(AuthContext {
        user_id: Some(claims.sub),
        session_id: Some(claims.sid),
        roles: claims.roles,
        trace_id: generate_trace_id(),
    })
}

#[cfg(feature = "demo")]
fn decode_demo_token(token: &str) -> Result<Claims, AuthError> {
    // Simple demo token format: "demo_user123"
    if let Some(user_id) = token.strip_prefix("demo_") {
        Ok(Claims {
            sub: user_id.to_string(),
            sid: "demo_session".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            roles: vec!["authenticated".to_string()],
        })
    } else {
        Err(AuthError::InvalidToken("Invalid demo token".to_string()))
    }
}
```

---
## 🛑 CHECKPOINT 1: Authorization Framework Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Created standard is_authorized helper function
2. Implemented auth context with session extraction
3. Added demo mode bypass for testing
4. Created fallback rules for degraded mode
5. Written comprehensive tests for all auth scenarios
6. Documented authorization flow and usage
7. Verified `just test-auth` passes
8. Committed all work with message: "Checkpoint 1: Authorization framework complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 1
- Your git commit hash

**DO NOT PROCEED** until you receive explicit approval.

---

### 4.2 Authorization Cache Implementation (2-3 work units)

**Work Unit Context:**
- **Complexity**: High - Distributed caching with LRU eviction
- **Scope**: ~600 lines across 3-4 files
- **Key Components**:
  - Cache structure with TTL support (~200 lines)
  - LRU eviction strategy (~150 lines)
  - Cache metrics and monitoring (~100 lines)
  - Cache warming and invalidation (~150 lines)
  - Comprehensive cache tests (~200 lines)
- **Algorithms**: LRU eviction, TTL management, atomic operations

#### Task 4.2.1: Write Cache Tests First
Create comprehensive cache tests:
```rust
#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_cache_get_set() {
        let cache = AuthCache::new(100);
        
        // Set value
        cache.set("user1:note:123:read", true, Duration::from_secs(60)).await;
        
        // Get value
        let result = cache.get("user1:note:123:read").await;
        assert_eq!(result, Some(true));
        
        // Non-existent key
        let result = cache.get("user1:note:456:write").await;
        assert_eq!(result, None);
    }
    
    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = AuthCache::new(100);
        
        // Set with short TTL
        cache.set("key1", true, Duration::from_millis(100)).await;
        
        // Should exist immediately
        assert_eq!(cache.get("key1").await, Some(true));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        assert_eq!(cache.get("key1").await, None);
    }
    
    #[tokio::test]
    async fn test_cache_only_positive_results() {
        let cache = AuthCache::new(100);
        
        // Try to set negative result
        cache.set("denied", false, Duration::from_secs(60)).await;
        
        // Should not be cached
        assert_eq!(cache.get("denied").await, None);
    }
    
    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = AuthCache::new(3); // Small cache
        
        // Fill cache
        cache.set("key1", true, Duration::from_secs(60)).await;
        cache.set("key2", true, Duration::from_secs(60)).await;
        cache.set("key3", true, Duration::from_secs(60)).await;
        
        // Access key1 and key2 to make them recently used
        cache.get("key1").await;
        cache.get("key2").await;
        
        // Add new key, should evict key3 (least recently used)
        cache.set("key4", true, Duration::from_secs(60)).await;
        
        assert_eq!(cache.get("key3").await, None); // Evicted
        assert_eq!(cache.get("key1").await, Some(true)); // Still there
    }
}
```

#### Task 4.2.2: Implement Authorization Cache
Create high-performance cache with LRU eviction:
```rust
// src/auth/cache.rs
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;
use prometheus::{Counter, Gauge};

pub struct AuthCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    metrics: CacheMetrics,
}

#[derive(Debug)]
struct CacheEntry {
    allowed: bool,
    expires_at: Instant,
    created_at: Instant,
    hit_count: AtomicU64,
    last_access: AtomicU64, // Epoch millis for LRU
}

struct CacheMetrics {
    hits: Counter,
    misses: Counter,
    evictions: Counter,
    size: Gauge,
}

impl AuthCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(max_size))),
            max_size,
            metrics: CacheMetrics {
                hits: register_counter!("auth_cache_hits_total"),
                misses: register_counter!("auth_cache_misses_total"),
                evictions: register_counter!("auth_cache_evictions_total"),
                size: register_gauge!("auth_cache_size"),
            },
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<bool> {
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(key) {
            // Check expiration
            if entry.expires_at > Instant::now() {
                // Update access tracking
                entry.hit_count.fetch_add(1, Ordering::Relaxed);
                entry.last_access.store(
                    chrono::Utc::now().timestamp_millis() as u64,
                    Ordering::Relaxed
                );
                
                self.metrics.hits.inc();
                return Some(entry.allowed);
            }
        }
        
        self.metrics.misses.inc();
        None
    }
    
    pub async fn set(&self, key: &str, allowed: bool, ttl: Duration) {
        // Only cache positive results per security requirements
        if !allowed {
            return;
        }
        
        let mut cache = self.cache.write().await;
        
        // Check if we need to evict
        if cache.len() >= self.max_size && !cache.contains_key(key) {
            self.evict_lru(&mut cache);
        }
        
        cache.insert(key.to_string(), CacheEntry {
            allowed,
            expires_at: Instant::now() + ttl,
            created_at: Instant::now(),
            hit_count: AtomicU64::new(0),
            last_access: AtomicU64::new(chrono::Utc::now().timestamp_millis() as u64),
        });
        
        self.metrics.size.set(cache.len() as f64);
    }
    
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        // Find least recently used entry
        let lru_key = cache.iter()
            .min_by_key(|(_, entry)| entry.last_access.load(Ordering::Relaxed))
            .map(|(key, _)| key.clone());
        
        if let Some(key) = lru_key {
            cache.remove(&key);
            self.metrics.evictions.inc();
            
            tracing::debug!(
                key = %key,
                cache_size = %cache.len(),
                "Evicted LRU cache entry"
            );
        }
    }
    
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        self.metrics.size.set(0.0);
    }
    
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        
        let total_hits: u64 = cache.values()
            .map(|e| e.hit_count.load(Ordering::Relaxed))
            .sum();
        
        CacheStats {
            size: cache.len(),
            capacity: self.max_size,
            total_hits,
            hit_rate: if total_hits > 0 {
                self.metrics.hits.get() / (self.metrics.hits.get() + self.metrics.misses.get())
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub total_hits: u64,
    pub hit_rate: f64,
}

// Background task to clean expired entries
impl AuthCache {
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let mut cache = self.cache.write().await;
                let now = Instant::now();
                
                // Remove expired entries
                cache.retain(|key, entry| {
                    let keep = entry.expires_at > now;
                    if !keep {
                        tracing::debug!(key = %key, "Removing expired cache entry");
                    }
                    keep
                });
                
                self.metrics.size.set(cache.len() as f64);
            }
        });
    }
}
```

#### Task 4.2.3: Add Cache Warming
Implement cache pre-warming for common permissions:
```rust
// src/auth/cache_warmer.rs
pub struct CacheWarmer {
    cache: Arc<AuthCache>,
    spicedb: Arc<SpiceDBClient>,
}

impl CacheWarmer {
    pub async fn warm_user_permissions(&self, user_id: &str) -> Result<()> {
        // Get user's recent resources
        let resources = self.get_user_resources(user_id).await?;
        
        // Common actions to pre-check
        let actions = vec!["read", "write", "delete"];
        
        // Batch check permissions
        let mut checks = Vec::new();
        for resource in &resources {
            for action in &actions {
                checks.push((
                    format!("user:{}", user_id),
                    resource.clone(),
                    action.to_string(),
                ));
            }
        }
        
        let results = self.spicedb.bulk_check(checks).await?;
        
        // Cache positive results
        for ((subject, resource, action), allowed) in results {
            if allowed {
                let key = format!("{}:{}:{}", 
                    subject.strip_prefix("user:").unwrap(),
                    resource,
                    action
                );
                self.cache.set(&key, true, Duration::from_secs(300)).await;
            }
        }
        
        Ok(())
    }
}
```

### 4.3 SpiceDB Integration (3-4 work units)

**Work Unit Context:**
- **Complexity**: High - External service integration with resilience
- **Scope**: ~800 lines across 4-5 files
- **Key Components**:
  - SpiceDB client wrapper (~300 lines)
  - Permission check methods (~200 lines)
  - Circuit breaker implementation (~200 lines)
  - Connection pool and retry logic (~150 lines)
  - Integration tests (~200 lines)
- **Patterns**: gRPC client, circuit breaker, connection pooling

#### Task 4.3.1: Write SpiceDB Client Tests
Test client behavior including failures:
```rust
#[cfg(test)]
mod spicedb_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_check_permission() {
        let client = SpiceDBClient::new("localhost:50051").await.unwrap();
        
        // Setup test data in SpiceDB
        client.write_relationships(vec![
            Relationship::new("note:123", "owner", "user:alice"),
        ]).await.unwrap();
        
        // Check permission
        let allowed = client.check_permission(
            "user:alice",
            "note:123",
            "write"
        ).await.unwrap();
        
        assert!(allowed);
    }
    
    #[tokio::test]
    async fn test_bulk_check_permissions() {
        let client = SpiceDBClient::new("localhost:50051").await.unwrap();
        
        let checks = vec![
            ("user:alice", "note:123", "read"),
            ("user:alice", "note:123", "write"),
            ("user:bob", "note:123", "read"),
        ];
        
        let results = client.bulk_check(checks).await.unwrap();
        
        assert_eq!(results.len(), 3);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let client = SpiceDBClient::new("invalid:50051").await.unwrap();
        let breaker = CircuitBreaker::new();
        
        // Fail multiple times
        for _ in 0..5 {
            let _ = breaker.call(|| client.check_permission(
                "user:test",
                "note:123",
                "read"
            )).await;
        }
        
        assert!(breaker.is_open());
    }
}
```

#### Task 4.3.2: Implement SpiceDB Client
Create client with connection pooling:
```rust
// src/services/spicedb/client.rs
use authzed::api::v1::*;
use tonic::transport::Channel;
use std::time::Duration;

pub struct SpiceDBClient {
    client: PermissionsServiceClient<Channel>,
    token: String,
}

impl SpiceDBClient {
    pub async fn new(endpoint: &str, token: &str) -> Result<Self> {
        let channel = Channel::from_shared(format!("http://{}", endpoint))?
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .http2_keep_alive_interval(Duration::from_secs(10))
            .connect()
            .await?;
        
        let client = PermissionsServiceClient::new(channel);
        
        Ok(Self {
            client,
            token: token.to_string(),
        })
    }
    
    pub async fn check_permission(
        &self,
        subject: String,
        resource: String,
        permission: String,
    ) -> Result<bool> {
        let request = CheckPermissionRequest {
            resource: Some(ObjectReference {
                object_type: resource.split(':').next().unwrap().to_string(),
                object_id: resource.split(':').nth(1).unwrap().to_string(),
            }),
            permission: permission.clone(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: subject.split(':').next().unwrap().to_string(),
                    object_id: subject.split(':').nth(1).unwrap().to_string(),
                }),
                optional_relation: None,
            }),
            consistency: None,
        };
        
        let mut request = tonic::Request::new(request);
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", self.token).parse()?,
        );
        
        let response = self.client
            .clone()
            .check_permission(request)
            .await?
            .into_inner();
        
        Ok(response.permissionship() == Permissionship::HasPermission)
    }
    
    pub async fn bulk_check(
        &self,
        checks: Vec<(String, String, String)>,
    ) -> Result<Vec<((String, String, String), bool)>> {
        let items = checks.iter().map(|(subject, resource, permission)| {
            BulkCheckPermissionRequestItem {
                resource: Some(ObjectReference {
                    object_type: resource.split(':').next().unwrap().to_string(),
                    object_id: resource.split(':').nth(1).unwrap().to_string(),
                }),
                permission: permission.clone(),
                subject: Some(SubjectReference {
                    object: Some(ObjectReference {
                        object_type: subject.split(':').next().unwrap().to_string(),
                        object_id: subject.split(':').nth(1).unwrap().to_string(),
                    }),
                    optional_relation: None,
                }),
            }
        }).collect();
        
        let request = BulkCheckPermissionRequest { items };
        
        let mut request = tonic::Request::new(request);
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", self.token).parse()?,
        );
        
        let response = self.client
            .clone()
            .bulk_check_permission(request)
            .await?
            .into_inner();
        
        let results = checks.into_iter()
            .zip(response.pairs)
            .map(|(check, pair)| {
                let allowed = pair.item
                    .and_then(|i| i.permissionship)
                    .map(|p| p == Permissionship::HasPermission as i32)
                    .unwrap_or(false);
                (check, allowed)
            })
            .collect();
        
        Ok(results)
    }
}
```

#### Task 4.3.3: Implement Circuit Breaker
Add resilience with circuit breaker pattern:
```rust
// src/auth/circuit_breaker.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open(Instant), // When opened
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    config: CircuitConfig,
}

#[derive(Clone)]
pub struct CircuitConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_timeout: Duration,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            half_open_timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self::with_config(CircuitConfig::default())
    }
    
    pub fn with_config(config: CircuitConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            config,
        }
    }
    
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitError<E>>
    where
        F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
    {
        // Check state and potentially transition
        let mut state = self.state.write().await;
        match &*state {
            CircuitState::Open(opened_at) => {
                if opened_at.elapsed() > self.config.timeout {
                    *state = CircuitState::HalfOpen;
                    *self.success_count.write().await = 0;
                } else {
                    return Err(CircuitError::CircuitOpen);
                }
            }
            _ => {}
        }
        drop(state);
        
        // Check if we can proceed
        let current_state = self.state.read().await.clone();
        match current_state {
            CircuitState::Open(_) => Err(CircuitError::CircuitOpen),
            CircuitState::Closed | CircuitState::HalfOpen => {
                // Try the operation
                match f().await {
                    Ok(result) => {
                        self.on_success().await;
                        Ok(result)
                    }
                    Err(error) => {
                        self.on_failure().await;
                        Err(CircuitError::OperationFailed(error))
                    }
                }
            }
        }
    }
    
    async fn on_success(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count = 0;
        
        let state = self.state.read().await.clone();
        if let CircuitState::HalfOpen = state {
            let mut success_count = self.success_count.write().await;
            *success_count += 1;
            
            if *success_count >= self.config.success_threshold {
                let mut state = self.state.write().await;
                *state = CircuitState::Closed;
                tracing::info!("Circuit breaker closed after {} successes", *success_count);
            }
        }
    }
    
    async fn on_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;
        
        if *failure_count >= self.config.failure_threshold {
            let mut state = self.state.write().await;
            *state = CircuitState::Open(Instant::now());
            tracing::warn!("Circuit breaker opened after {} failures", *failure_count);
            
            metrics::CIRCUIT_BREAKER_STATE
                .with_label_values(&["spicedb", "open"])
                .set(1.0);
        }
        
        // If in half-open state, immediately open
        let state = self.state.read().await.clone();
        if let CircuitState::HalfOpen = state {
            let mut state = self.state.write().await;
            *state = CircuitState::Open(Instant::now());
            *self.success_count.write().await = 0;
        }
    }
    
    pub async fn is_open(&self) -> bool {
        matches!(*self.state.read().await, CircuitState::Open(_))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CircuitError<E> {
    #[error("Circuit breaker is open")]
    CircuitOpen,
    
    #[error("Operation failed: {0}")]
    OperationFailed(E),
}
```

---
## 🛑 CHECKPOINT 2: Cache and SpiceDB Integration Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Implemented authorization cache with LRU eviction
2. Created SpiceDB client with connection pooling
3. Implemented circuit breaker for resilience
4. Added cache warming capabilities
5. Only positive results are cached
6. Written comprehensive tests for cache and client
7. Added metrics for monitoring
8. Documented SpiceDB schema and setup
9. Committed all work with message: "Checkpoint 2: Cache and SpiceDB integration complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 2
- Your git commit hash
- Evidence of cache and circuit breaker working

**DO NOT PROCEED** until you receive explicit approval.

---

### 4.4 Audit Logging & Monitoring (2 work units)

**Work Unit Context:**
- **Complexity**: Medium - Structured logging and metrics
- **Scope**: ~400 lines across 3-4 files
- **Key Components**:
  - Audit log structure and writer (~150 lines)
  - Async audit logging (~100 lines)
  - Prometheus metrics (~100 lines)
  - Audit log queries (~50 lines)
  - Monitoring dashboards (configuration)
- **No complex algorithms** - Just structured logging and metrics

#### Task 4.4.1: Write Audit Log Tests
Test audit logging functionality:
```rust
#[cfg(test)]
mod audit_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_audit_log_creation() {
        let logger = AuditLogger::new();
        
        let entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            user_id: "user123".to_string(),
            resource: "note:456".to_string(),
            action: "write".to_string(),
            allowed: true,
            source: "spicedb".to_string(),
            trace_id: "trace789".to_string(),
        };
        
        logger.log(entry.clone()).await.unwrap();
        
        // Verify log was written
        let logs = logger.query_by_user("user123", 10).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].resource, "note:456");
    }
    
    #[tokio::test]
    async fn test_audit_metrics() {
        let logger = AuditLogger::new();
        
        // Log some decisions
        for allowed in [true, false, true] {
            logger.log(AuditEntry {
                timestamp: chrono::Utc::now(),
                user_id: "test".to_string(),
                resource: "test:1".to_string(),
                action: "read".to_string(),
                allowed,
                source: "cache".to_string(),
                trace_id: "test".to_string(),
            }).await.unwrap();
        }
        
        // Check metrics
        assert_eq!(
            metrics::AUTH_DECISIONS_TOTAL
                .with_label_values(&["allowed"])
                .get(),
            2
        );
    }
}
```

#### Task 4.4.2: Implement Audit Logger
Create comprehensive audit logging:
```rust
// src/auth/audit.rs
use tokio::sync::mpsc;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: String,
    pub resource: String,
    pub action: String,
    pub allowed: bool,
    pub source: String, // "spicedb" | "cache" | "fallback"
    pub trace_id: String,
}

pub struct AuditLogger {
    sender: mpsc::Sender<AuditEntry>,
}

impl AuditLogger {
    pub fn new(buffer_size: usize) -> (Self, AuditLogWriter) {
        let (sender, receiver) = mpsc::channel(buffer_size);
        
        let logger = Self { sender };
        let writer = AuditLogWriter { receiver };
        
        (logger, writer)
    }
    
    pub async fn log(&self, entry: AuditEntry) -> Result<()> {
        // Update metrics
        let result_label = if entry.allowed { "allowed" } else { "denied" };
        metrics::AUTH_DECISIONS_TOTAL
            .with_label_values(&[result_label, &entry.source])
            .inc();
        
        // Send to writer
        self.sender.send(entry).await
            .map_err(|_| anyhow!("Audit log channel closed"))?;
        
        Ok(())
    }
}

pub struct AuditLogWriter {
    receiver: mpsc::Receiver<AuditEntry>,
}

impl AuditLogWriter {
    pub async fn run(mut self, storage: Arc<dyn AuditStorage>) {
        let mut batch = Vec::with_capacity(100);
        let mut flush_interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            tokio::select! {
                Some(entry) = self.receiver.recv() => {
                    batch.push(entry);
                    
                    // Flush if batch is full
                    if batch.len() >= 100 {
                        if let Err(e) = storage.write_batch(&batch).await {
                            tracing::error!("Failed to write audit batch: {}", e);
                        }
                        batch.clear();
                    }
                }
                _ = flush_interval.tick() => {
                    // Periodic flush
                    if !batch.is_empty() {
                        if let Err(e) = storage.write_batch(&batch).await {
                            tracing::error!("Failed to write audit batch: {}", e);
                        }
                        batch.clear();
                    }
                }
                else => break,
            }
        }
    }
}

// Storage trait for flexibility
#[async_trait]
pub trait AuditStorage: Send + Sync {
    async fn write_batch(&self, entries: &[AuditEntry]) -> Result<()>;
    async fn query_by_user(&self, user_id: &str, limit: usize) -> Result<Vec<AuditEntry>>;
    async fn query_by_resource(&self, resource: &str, limit: usize) -> Result<Vec<AuditEntry>>;
}

// File-based implementation for demo
pub struct FileAuditStorage {
    path: PathBuf,
}

#[async_trait]
impl AuditStorage for FileAuditStorage {
    async fn write_batch(&self, entries: &[AuditEntry]) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        
        for entry in entries {
            let line = serde_json::to_string(entry)? + "\n";
            file.write_all(line.as_bytes()).await?;
        }
        
        Ok(())
    }
    
    async fn query_by_user(&self, user_id: &str, limit: usize) -> Result<Vec<AuditEntry>> {
        // Simple implementation - in production use proper database
        let content = tokio::fs::read_to_string(&self.path).await?;
        
        let entries: Vec<AuditEntry> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .filter(|e: &AuditEntry| e.user_id == user_id)
            .take(limit)
            .collect();
        
        Ok(entries)
    }
    
    async fn query_by_resource(&self, resource: &str, limit: usize) -> Result<Vec<AuditEntry>> {
        let content = tokio::fs::read_to_string(&self.path).await?;
        
        let entries: Vec<AuditEntry> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .filter(|e: &AuditEntry| e.resource == resource)
            .take(limit)
            .collect();
        
        Ok(entries)
    }
}
```

#### Task 4.4.3: Add Authorization Metrics
Comprehensive metrics for monitoring:
```rust
// src/auth/metrics.rs
use prometheus::{
    register_counter_vec, register_histogram_vec, register_gauge_vec,
    CounterVec, HistogramVec, GaugeVec,
};

lazy_static! {
    pub static ref AUTH_CHECKS: CounterVec = register_counter_vec!(
        "auth_checks_total",
        "Total authorization checks",
        &["result"] // success, timeout, circuit_open
    ).unwrap();
    
    pub static ref AUTH_DECISIONS_TOTAL: CounterVec = register_counter_vec!(
        "auth_decisions_total",
        "Authorization decisions",
        &["result", "source"] // allowed/denied, spicedb/cache/fallback
    ).unwrap();
    
    pub static ref AUTH_CHECK_DURATION: HistogramVec = register_histogram_vec!(
        "auth_check_duration_seconds",
        "Authorization check duration",
        &["source"], // spicedb, cache
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();
    
    pub static ref AUTH_CACHE_SIZE: Gauge = register_gauge!(
        "auth_cache_size",
        "Current size of authorization cache"
    ).unwrap();
    
    pub static ref AUTH_CACHE_HIT_RATE: Gauge = register_gauge!(
        "auth_cache_hit_rate",
        "Authorization cache hit rate"
    ).unwrap();
    
    pub static ref CIRCUIT_BREAKER_STATE: GaugeVec = register_gauge_vec!(
        "circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=open)",
        &["service", "state"]
    ).unwrap();
    
    pub static ref AUTH_FALLBACK_DECISIONS: CounterVec = register_counter_vec!(
        "auth_fallback_decisions_total",
        "Fallback authorization decisions",
        &["action", "decision"]
    ).unwrap();
}

// Helper to time operations
pub async fn time_auth_check<F, T>(source: &str, f: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let timer = AUTH_CHECK_DURATION
        .with_label_values(&[source])
        .start_timer();
    
    let result = f.await;
    timer.observe_duration();
    
    result
}
```

### 4.5 Integration & Testing (2 work units)

**Work Unit Context:**
- **Complexity**: Medium - Integration with GraphQL resolvers
- **Scope**: ~500 lines of tests and integration
- **Key Components**:
  - GraphQL resolver integration (~150 lines)
  - End-to-end authorization tests (~200 lines)
  - Performance benchmarks (~100 lines)
  - Verification scripts (~50 lines)
- **Patterns**: Integration testing, benchmarking

#### Task 4.5.1: Write Integration Tests
Test complete authorization flow:
```rust
// tests/auth_integration.rs
#[tokio::test]
async fn test_graphql_with_authorization() {
    let app = create_test_app().await;
    
    // Test without auth - should fail
    let response = app
        .post("/graphql")
        .json(&json!({
            "query": r#"
                mutation {
                    updateNote(id: "123", input: { title: "New" }) {
                        id
                    }
                }
            "#
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await;
    assert!(body["errors"][0]["extensions"]["code"] == "UNAUTHORIZED");
    
    // Test with valid auth
    let response = app
        .post("/graphql")
        .header("Authorization", "Bearer valid_token")
        .json(&json!({
            "query": r#"
                mutation {
                    updateNote(id: "123", input: { title: "New" }) {
                        id
                    }
                }
            "#
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), 200);
    // Should succeed if user has permission
}

#[tokio::test]
async fn test_authorization_caching() {
    let app = create_test_app().await;
    let token = "Bearer test_token";
    
    // First request - hits SpiceDB
    let start = Instant::now();
    let response1 = app
        .post("/graphql")
        .header("Authorization", token)
        .json(&json!({
            "query": r#"{ note(id: "123") { title } }"#
        }))
        .send()
        .await;
    let duration1 = start.elapsed();
    
    // Second request - should hit cache
    let start = Instant::now();
    let response2 = app
        .post("/graphql")
        .header("Authorization", token)
        .json(&json!({
            "query": r#"{ note(id: "123") { title } }"#
        }))
        .send()
        .await;
    let duration2 = start.elapsed();
    
    // Cache should be much faster
    assert!(duration2 < duration1 / 2);
}

#[tokio::test]
async fn test_fallback_during_outage() {
    let app = create_test_app().await;
    
    // Simulate SpiceDB outage
    app.spicedb_client.force_circuit_open().await;
    
    // Health checks should still work
    let response = app
        .post("/graphql")
        .json(&json!({
            "query": r#"{ health { status } }"#
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), 200);
    
    // Write operations should be denied
    let response = app
        .post("/graphql")
        .header("Authorization", "Bearer test_token")
        .json(&json!({
            "query": r#"
                mutation {
                    createNote(input: { title: "Test" }) {
                        id
                    }
                }
            "#
        }))
        .send()
        .await;
    
    let body: serde_json::Value = response.json().await;
    assert!(body["errors"][0]["extensions"]["code"] == "FORBIDDEN");
}
```

#### Task 4.5.2: Update GraphQL Resolvers
Integrate authorization into all resolvers:
```rust
// Update src/graphql/resolvers/mutations.rs
impl Mutation {
    async fn create_note(
        &self,
        ctx: &Context<'_>,
        input: CreateNoteInput,
    ) -> Result<Note> {
        // Validate input first
        input.validate()?;
        
        // Check authorization - create permission on parent
        is_authorized(ctx, "notes:collection", "create").await?;
        
        // Create note...
        let note = /* ... */;
        
        Ok(note)
    }
    
    async fn update_note(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateNoteInput,
    ) -> Result<Note> {
        // Check authorization for specific note
        is_authorized(ctx, &format!("note:{}", id), "write").await?;
        
        // Update note...
        let note = /* ... */;
        
        Ok(note)
    }
    
    async fn delete_note(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<bool> {
        // Check delete permission
        is_authorized(ctx, &format!("note:{}", id), "delete").await?;
        
        // Delete note...
        
        Ok(true)
    }
}

// Update queries similarly
impl Query {
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        // Check read permission
        is_authorized(ctx, &format!("note:{}", id), "read").await?;
        
        // Fetch note...
    }
    
    async fn notes(&self, ctx: &Context<'_>, limit: Option<i32>) -> Result<Vec<Note>> {
        // For list queries, filter by permissions
        let notes = /* fetch notes */;
        
        // Batch check permissions
        let checks: Vec<_> = notes.iter()
            .map(|n| (format!("note:{}", n.id), "read"))
            .collect();
        
        let results = batch_authorize(ctx, checks).await?;
        
        // Filter to only authorized notes
        let authorized_notes = notes.into_iter()
            .zip(results)
            .filter_map(|(note, allowed)| if allowed { Some(note) } else { None })
            .collect();
        
        Ok(authorized_notes)
    }
}
```

#### Task 4.5.3: Create Verification Script
Add Phase 4 verification:
```bash
#!/bin/bash
# scripts/verify-phase-4.sh
set -e

echo "=== Phase 4 Verification ==="

# 1. Check compilation
echo "✓ Checking compilation..."
just build

# 2. Start SpiceDB
echo "✓ Starting SpiceDB..."
just spicedb-up
sleep 5

# 3. Run auth tests
echo "✓ Running authorization tests..."
just test-auth

# 4. Start server
echo "✓ Starting server..."
cargo run --features demo &
SERVER_PID=$!
sleep 5

# 5. Test unauthorized request
echo "✓ Testing unauthorized access..."
RESPONSE=$(curl -s -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { createNote(input:{title:\"Test\"}) { id } }"}')
echo $RESPONSE | jq -e '.errors[0].extensions.code == "UNAUTHORIZED"'

# 6. Test authorized request (demo mode)
echo "✓ Testing authorized access..."
RESPONSE=$(curl -s -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo_user123" \
  -d '{"query":"mutation { createNote(input:{title:\"Test\",content:\"Test\",author:\"user123\"}) { id } }"}')
echo $RESPONSE | jq -e '.data.createNote.id'

# 7. Check metrics
echo "✓ Checking authorization metrics..."
curl -s http://localhost:8080/metrics | grep -E "auth_checks_total|auth_cache_"

# 8. Test circuit breaker
echo "✓ Testing circuit breaker..."
# Stop SpiceDB
just spicedb-down
sleep 2

# Should still handle health checks
curl -f http://localhost:8080/health

# 9. Cleanup
kill $SERVER_PID
just spicedb-down || true

echo "=== All Phase 4 checks passed! ==="
```

---
## 🛑 CHECKPOINT 3: Complete Phase 4 System Review

**STOP HERE FOR FINAL EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Integrated authorization into all GraphQL resolvers
2. Implemented comprehensive audit logging
3. Added all required metrics
4. Created integration tests for auth flows
5. Tested circuit breaker and fallback behavior
6. Verified cache performance improvements
7. Demo mode bypass works for testing
8. All authorization decisions are logged
9. Documentation complete
10. Committed all work with message: "Checkpoint 3: Phase 4 complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 3
- Your git commit hash
- Output from `scripts/verify-phase-4.sh`
- Metrics showing auth performance
- Sample audit logs

**Review Checklist for Reviewer**:

### Authorization Implementation
- [ ] Standard is_authorized helper used consistently
- [ ] All resolvers check permissions
- [ ] Proper 401 vs 403 responses
- [ ] Demo mode bypass works
- [ ] Batch authorization for lists

### Resilience & Performance
- [ ] SpiceDB circuit breaker works
- [ ] Fallback rules apply correctly
- [ ] Cache improves performance
- [ ] Only positive results cached
- [ ] Cache TTL extends during outages

### Monitoring & Compliance
- [ ] All decisions audit logged
- [ ] Metrics track performance
- [ ] Circuit breaker state visible
- [ ] Cache hit rate tracked
- [ ] No sensitive data in logs

### Integration
- [ ] Works with Phase 1-3 systems
- [ ] Session extraction from headers
- [ ] Error responses consistent
- [ ] Health checks bypass auth

### Testing & Documentation
- [ ] Unit tests comprehensive
- [ ] Integration tests cover flows
- [ ] Fallback behavior tested
- [ ] Performance benchmarks done
- [ ] Documentation complete

### Operational Readiness
- [ ] Verification script passes
- [ ] SpiceDB setup documented
- [ ] Cache tuning guide provided
- [ ] Monitoring alerts configured
- [ ] All Phase 4 "Done Criteria" met

**Final Approval Required**: The reviewer must explicitly approve before Phase 5 can begin.

---

## Final Phase 4 Deliverables

Before marking Phase 4 complete, ensure these artifacts exist:

1. **Documentation**
   - [ ] Authorization architecture guide
   - [ ] SpiceDB schema documentation
   - [ ] Cache tuning recommendations
   - [ ] Audit log query examples

2. **Tests**
   - [ ] Unit tests for all auth components
   - [ ] Integration tests with SpiceDB
   - [ ] Circuit breaker tests
   - [ ] Performance benchmarks

3. **Scripts**
   - [ ] `scripts/verify-phase-4.sh` - Automated verification
   - [ ] `scripts/setup-spicedb.sh` - SpiceDB setup
   - [ ] `scripts/test-auth.sh` - Auth-specific tests

4. **Metrics**
   - [ ] Authorization check counts
   - [ ] Cache hit/miss rates
   - [ ] Circuit breaker state
   - [ ] Audit decision counts

## Next Steps

Once all checkpoints pass:
1. Commit with message: "Complete Phase 4: Authorization & Authentication"
2. Tag as `v0.4.0-phase4`
3. Create PR for review if working in team
4. Document any deviations from original plan
5. Begin Phase 5 planning (Observability & Monitoring)

## Important Notes

- **DO NOT PROCEED** past a checkpoint until all verification steps pass
- **MAINTAIN** fail-secure approach - deny by default
- **DOCUMENT** SpiceDB schema and relationships
- **TEST** degraded mode thoroughly - system must remain available
- **MONITOR** cache effectiveness and adjust TTLs as needed

## Troubleshooting Guide

### Common Issues and Solutions

#### Authorization Issues

**Issue**: All requests return 403 Forbidden
**Solution**: 
- Check SpiceDB is running and accessible
- Verify relationships are properly created
- Check circuit breaker state
- Review audit logs for actual vs expected permissions

**Issue**: Cache hit rate is low
**Solution**:
- Increase cache size if at capacity
- Check TTL settings
- Verify positive-only caching
- Look for cache key mismatches

#### SpiceDB Issues

**Issue**: Circuit breaker keeps opening
**Solution**:
- Check SpiceDB health
- Verify network connectivity
- Review timeout settings
- Check SpiceDB logs for errors

**Issue**: Slow permission checks
**Solution**:
- Verify SpiceDB indexes
- Check relationship complexity
- Consider batch checks
- Review SpiceDB performance docs

#### Integration Issues

**Issue**: Session extraction fails
**Solution**:
- Check JWT format and claims
- Verify Authorization header format
- Test with demo tokens
- Review session extraction logs

### Debugging Tips

1. **Enable debug logging**: `RUST_LOG=pcf_api::auth=debug`
2. **Check audit logs**: Look for patterns in denials
3. **Monitor metrics**: Watch cache hit rate and latencies
4. **Test with curl**: Isolate auth from GraphQL
5. **Use demo mode**: Bypass auth to test other issues

### Useful Resources

- [SpiceDB Documentation](https://docs.authzed.com/)
- [Circuit Breaker Pattern](https://martinfowler.com/bliki/CircuitBreaker.html)
- [JWT Best Practices](https://tools.ietf.org/html/rfc8725)
- [OWASP Authorization Guide](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)

---
*This work plan follows the same structure and practices as Phases 1-3, adapted for authorization implementation.*
# Authorization Specification

## Core Principles

1. **Zero Trust**: Every request MUST be authorized, no implicit permissions
2. **Fail Secure**: When uncertain, MUST deny access
3. **Cache with Care**: Cache positive results only, never cache denials
4. **Graceful Degradation**: MUST handle SpiceDB unavailability without service outage
5. **Audit Everything**: All authorization decisions MUST be logged for compliance

## Authorization Architecture

The API server acts as an authorization enforcement point, delegating permission decisions to SpiceDB while maintaining resilience through caching and fallback strategies.

## Authorization Flow

1. **Extract Authentication**: Get user identity from request headers
2. **Check Cache**: Look for cached authorization result
3. **Check Circuit Breaker**: Verify SpiceDB availability
4. **Query SpiceDB**: If cache miss and service available
5. **Fallback Strategy**: Apply degraded mode rules if SpiceDB unavailable
6. **Cache Result**: Store positive results with TTL
7. **Audit Log**: Record decision with context
8. **Enforce Decision**: Allow or deny based on final determination

## Service Degradation Strategy

**When SpiceDB is Available:**
- Query SpiceDB for all decisions
- Cache positive results for 5 minutes
- Never cache negative results

**When SpiceDB is Degraded (slow response):**
- Use cached results if available (extend TTL)
- For cache misses, apply timeout of 2 seconds
- If timeout, use fallback rules

**When SpiceDB is Unavailable:**
- Use cached results if available (extend TTL to 30 minutes)
- For cache misses, apply conservative fallback rules:
  - Allow: Health checks, read-only operations by resource owner
  - Deny: All write operations, cross-user access, admin operations
- Log all fallback decisions with WARNING level

## Standard Authorization Function

```rust
// helpers/authorization.rs
pub async fn is_authorized(
    ctx: &Context<'_>,
    resource: &str,
    action: &str,
) -> Result<()> {
    // Extract user context
    let auth = ctx.data::<AuthContext>()?;
    if auth.user_id.is_none() {
        return Err(FieldError::new("Authentication required")
            .extend_with(|_, e| e.set("code", "UNAUTHORIZED")));
    }
    let user_id = auth.user_id.as_ref().unwrap();
    
    // Check cache
    let cache = ctx.data::<AuthzCache>()?;
    let cache_key = format!("{}:{}:{}", user_id, resource, action);
    
    if let Some(allowed) = cache.get(&cache_key).await {
        return if allowed {
            Ok(())
        } else {
            Err(FieldError::new("Permission denied")
                .extend_with(|_, e| e.set("code", "FORBIDDEN")))
        };
    }
    
    // Check circuit breaker status
    let spicedb = ctx.data::<SpiceDBClient>()?;
    let circuit_breaker = ctx.data::<CircuitBreaker>()?;
    
    let allowed = match circuit_breaker.call(|| async {
        // Query SpiceDB with timeout
        timeout(
            Duration::from_secs(2),
            spicedb.check_permission(CheckPermissionRequest {
                subject: &format!("user:{}", user_id),
                resource,
                permission: action,
            })
        ).await
    }).await {
        Ok(Ok(result)) => {
            // Successful SpiceDB response
            metrics::AUTH_CHECKS.with_label_values(&["success"]).inc();
            result
        }
        Ok(Err(_)) => {
            // Timeout - use fallback
            warn!(
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
            warn!(
                user_id = %user_id,
                resource = %resource,
                action = %action,
                "SpiceDB unavailable, using fallback rules"
            );
            metrics::AUTH_CHECKS.with_label_values(&["circuit_open"]).inc();
            apply_fallback_rules(user_id, resource, action)
        }
    };
    
    // Cache result - only cache positive results
    if allowed {
        let cache_ttl = if circuit_breaker.is_open() {
            // Extended TTL during outage
            Duration::from_secs(1800) // 30 minutes
        } else {
            #[cfg(feature = "demo")]
            let ttl = ctx.data::<Config>()?.auth.cache_ttl_seconds;
            #[cfg(not(feature = "demo"))]
            let ttl = 300; // 5 minutes
            Duration::from_secs(ttl)
        };
        
        cache.set(&cache_key, allowed, cache_ttl).await;
    }
    
    // Audit log
    audit_log(AuditEntry {
        timestamp: Utc::now(),
        user_id: user_id.clone(),
        resource: resource.to_string(),
        action: action.to_string(),
        allowed,
        source: if circuit_breaker.is_open() { "fallback" } else { "spicedb" },
        trace_id: ctx.trace_id(),
    }).await;
    
    // Return result
    if allowed {
        Ok(())
    } else {
        Err(FieldError::new("Permission denied")
            .extend_with(|_, e| e.set("code", "FORBIDDEN")))
    }
}
```

## Usage in Resolvers

```rust
// In every resolver that needs authorization
async fn update_note(
    &self,
    ctx: &Context<'_>,
    id: String,
    input: UpdateNoteInput,
) -> Result<Note> {
    // Check authorization first
    is_authorized(ctx, &format!("note:{}", id), "write").await?;
    
    // Proceed with business logic
    let db = ctx.data::<DatabaseService>()?;
    db.update_note(&id, input).await
}

async fn delete_note(
    &self,
    ctx: &Context<'_>,
    id: String,
) -> Result<bool> {
    // Different permission for delete
    is_authorized(ctx, &format!("note:{}", id), "delete").await?;
    
    let db = ctx.data::<DatabaseService>()?;
    db.delete_note(&id).await
}
```

## SpiceDB Integration

### Resource Format
```
type:id
```
Examples:
- `note:123` - Note with ID 123
- `organization:acme` - Organization named acme
- `user:user_456` - User with ID user_456

### Permission Names
Standard CRUD permissions:
- `read` - View resource
- `write` - Modify resource
- `delete` - Remove resource
- `create` - Create new resources (checked on parent)

Domain-specific permissions:
- `share` - Share with others
- `publish` - Make public
- `admin` - Administrative access

### SpiceDB Schema Example
```zed
definition user {}

definition organization {
    relation admin: user
    relation member: user
    
    permission manage = admin
    permission view = member + admin
}

definition note {
    relation owner: user
    relation viewer: user
    relation parent: organization
    
    permission read = viewer + owner + parent->view
    permission write = owner + parent->admin
    permission delete = owner + parent->admin
    permission share = owner
}
```

## Fallback Rules Implementation

```rust
fn apply_fallback_rules(
    user_id: &str,
    resource: &str,
    action: &str,
) -> bool {
    // Parse resource type and ID
    let parts: Vec<&str> = resource.split(':').collect();
    if parts.len() != 2 {
        return false; // Invalid resource format, deny
    }
    let (resource_type, resource_id) = (parts[0], parts[1]);
    
    // Conservative fallback rules
    match (resource_type, action) {
        // Always allow health checks
        ("health", "read") => true,
        
        // Allow users to read their own resources
        ("user", "read") if resource_id == user_id => true,
        
        // Allow read-only operations during degraded mode
        (_, "read") => {
            // Only if we have evidence of prior access (check cache)
            // This prevents information disclosure
            false
        }
        
        // Deny all write operations during degraded mode
        (_, "write" | "delete" | "create" | "admin") => false,
        
        // Default deny
        _ => false,
    }
}
```

## Enhanced Cache Implementation

```rust
pub struct AuthzCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    metrics: CacheMetrics,
}

struct CacheEntry {
    allowed: bool,
    expires_at: Instant,
    created_at: Instant,
    hit_count: AtomicU64,
    last_access: AtomicU64,
}

struct CacheMetrics {
    hits: Counter,
    misses: Counter,
    evictions: Counter,
    size: Gauge,
}

impl AuthzCache {
    pub async fn get(&self, key: &str) -> Option<bool> {
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                // Update metrics
                entry.hit_count.fetch_add(1, Ordering::Relaxed);
                entry.last_access.store(
                    Instant::now().elapsed().as_secs(),
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
        // Only cache positive results
        if !allowed {
            return;
        }
        
        let mut cache = self.cache.write().await;
        
        // Enforce cache size limit with LRU eviction
        if cache.len() >= 10000 {
            self.evict_least_recently_used(&mut cache).await;
        }
        
        cache.insert(key.to_string(), CacheEntry {
            allowed,
            expires_at: Instant::now() + ttl,
            created_at: Instant::now(),
            hit_count: AtomicU64::new(0),
            last_access: AtomicU64::new(0),
        });
        
        self.metrics.size.set(cache.len() as f64);
    }
    
    async fn evict_least_recently_used(
        &self,
        cache: &mut HashMap<String, CacheEntry>
    ) {
        // Find and remove 10% of least recently used entries
        let mut entries: Vec<(String, u64)> = cache.iter()
            .map(|(k, v)| (k.clone(), v.last_access.load(Ordering::Relaxed)))
            .collect();
        
        entries.sort_by_key(|e| e.1);
        
        let evict_count = cache.len() / 10;
        for (key, _) in entries.iter().take(evict_count) {
            cache.remove(key);
            self.metrics.evictions.inc();
        }
    }
}
```

## Bypass for Demo Mode

In demo mode, authorization can be bypassed for testing:

```rust
#[cfg(feature = "demo")]
if ctx.data::<Config>()?.demo.bypass_auth {
    return Ok(());
}
```

## Error Handling

Authorization errors must be clear but not leak information:

**Authenticated but not authorized**:
- Code: `FORBIDDEN`
- Message: "Permission denied"
- HTTP: 403

**Not authenticated**:
- Code: `UNAUTHORIZED`  
- Message: "Authentication required"
- HTTP: 401

**SpiceDB unavailable (fallback allowed)**:
- Code: `FORBIDDEN` or allow based on fallback rules
- Message: "Permission denied" or success
- HTTP: 403 or 200
- Log: WARNING level with fallback decision

**SpiceDB unavailable (no fallback possible)**:
- Code: `SERVICE_UNAVAILABLE`
- Message: "Authorization service temporarily unavailable"
- HTTP: 503
- Action: Return immediately, do not attempt operation

## Monitoring and Alerting

**Required Metrics:**
```prometheus
# Authorization check results
auth_checks_total{result="success|denied|error|timeout|fallback"}

# Cache performance
auth_cache_hits_total
auth_cache_misses_total
auth_cache_size
auth_cache_evictions_total

# SpiceDB circuit breaker
circuit_breaker_state{service="spicedb",state="closed|open|half_open"}

# Fallback usage
auth_fallback_decisions_total{action="read|write",decision="allow|deny"}
```

**Alert Conditions:**
- SpiceDB error rate > 10% for 5 minutes
- Circuit breaker open > 5 minutes
- Cache hit rate < 80% (possible cache poisoning)
- Fallback usage > 100 requests/minute

## Future Enhancements

### Batch Authorization
For list queries, check permissions in batch:
```rust
let resources: Vec<String> = notes.iter()
    .map(|n| format!("note:{}", n.id))
    .collect();

let permissions = spicedb.bulk_check(user_id, resources, "read").await?;
```

### Permission Hints
Include what permissions user has in responses:
```graphql
type Note {
  id: String!
  title: String!
  _permissions: Permissions!
}

type Permissions {
  canEdit: Boolean!
  canDelete: Boolean!
  canShare: Boolean!
}
```
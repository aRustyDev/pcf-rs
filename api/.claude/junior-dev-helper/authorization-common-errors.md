# Common Authorization Errors Guide

## Overview

Authorization errors can be tricky to debug. This guide covers the most common issues you'll encounter and how to fix them.

## Error Types: 401 vs 403

### 401 Unauthorized - "Who are you?"
- User is not authenticated (not logged in)
- Missing or invalid authentication token
- Session expired

### 403 Forbidden - "You can't do that!"
- User is authenticated but lacks permission
- Valid token but insufficient privileges
- Resource exists but user can't access it

```rust
// Correct error usage
if auth_context.user_id.is_none() {
    return Err(Error::new("Authentication required")
        .extend_with(|_, e| e.set("code", "UNAUTHORIZED"))); // 401
}

if !has_permission {
    return Err(Error::new("Permission denied")
        .extend_with(|_, e| e.set("code", "FORBIDDEN"))); // 403
}
```

## Common Compilation Errors

### 1. Missing Async in Trait Implementation

**Error:**
```
error[E0195]: lifetime parameters or bounds on method `call` do not match the trait declaration
```

**Problem:**
```rust
// Wrong - missing async_trait
impl CircuitBreaker {
    async fn call<F>(&self, f: F) -> Result<T, E> {
        // ...
    }
}
```

**Solution:**
```rust
use async_trait::async_trait;

#[async_trait]
impl CircuitBreaker {
    async fn call<F>(&self, f: F) -> Result<T, E> {
        // ...
    }
}
```

### 2. Lifetime Issues with Closures

**Error:**
```
error[E0373]: closure may outlive the current function, but it borrows `spicedb`
```

**Problem:**
```rust
circuit_breaker.call(|| {
    spicedb.check_permission(user, resource, action) // Borrows spicedb
}).await
```

**Solution:**
```rust
circuit_breaker.call(|| {
    let spicedb = spicedb.clone(); // Clone Arc
    Box::pin(async move {
        spicedb.check_permission(user, resource, action).await
    })
}).await
```

### 3. Type Mismatch in Error Handling

**Error:**
```
error[E0308]: mismatched types
expected enum `async_graphql::Error`
found enum `std::result::Result`
```

**Problem:**
```rust
let auth = ctx.data::<AuthContext>()?; // Returns wrong error type
```

**Solution:**
```rust
let auth = ctx.data::<AuthContext>()
    .map_err(|_| Error::new("Internal error: auth context missing"))?;
```

## Common Runtime Errors

### 1. Cache Never Hits

**Symptom:** Cache hit rate is 0%, every request goes to SpiceDB

**Common Causes:**
1. **Only caching positive results (correct) but testing with failures**
   ```rust
   // If user lacks permission, cache.get() returns None
   // This is correct behavior!
   ```

2. **Cache key mismatch**
   ```rust
   // Setting with one format
   cache.set("alice:note:123:read", true, ttl).await;
   
   // Getting with different format
   cache.get("user:alice:note:123:read").await; // Won't find it!
   ```

3. **TTL too short**
   ```rust
   // 1 second TTL - expires before next request
   cache.set(key, true, Duration::from_secs(1)).await;
   ```

**Solution:**
```rust
// Consistent key format
fn cache_key(user_id: &str, resource: &str, action: &str) -> String {
    format!("{}:{}:{}", user_id, resource, action)
}

// Reasonable TTL
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes
```

### 2. Circuit Breaker Never Opens

**Symptom:** SpiceDB is down but circuit breaker stays closed

**Common Causes:**
1. **Not counting timeouts as failures**
   ```rust
   // Wrong - timeout returns Ok(Err(_)), not Err(_)
   match timeout(duration, operation).await {
       Ok(Ok(result)) => Ok(result),
       Ok(Err(e)) => Err(e), // Should count as failure!
       Err(_) => Err("timeout"), // Timeout not counted
   }
   ```

2. **Failure threshold too high**
   ```rust
   CircuitBreaker::new()
       .failure_threshold(100) // Takes 100 failures to open!
   ```

**Solution:**
```rust
match timeout(duration, operation).await {
    Ok(Ok(result)) => {
        self.record_success();
        Ok(result)
    }
    Ok(Err(e)) => {
        self.record_failure(); // Count operation errors
        Err(e)
    }
    Err(_) => {
        self.record_failure(); // Count timeouts
        Err("Operation timed out")
    }
}
```

### 3. Fallback Rules Too Permissive

**Symptom:** Users can access resources they shouldn't during SpiceDB outage

**Problem:**
```rust
// Dangerous fallback
fn apply_fallback_rules(user: &str, resource: &str, action: &str) -> bool {
    match action {
        "read" => true, // Allows reading EVERYTHING!
        _ => false,
    }
}
```

**Solution:**
```rust
// Conservative fallback
fn apply_fallback_rules(user: &str, resource: &str, action: &str) -> bool {
    let (resource_type, resource_id) = parse_resource(resource);
    
    match (resource_type, action) {
        // Only allow critical operations
        ("health", "read") => true,
        ("user", "read") if resource_id == user => true, // Own profile only
        _ => false, // Deny everything else
    }
}
```

## GraphQL-Specific Issues

### 1. Authorization Check in Wrong Place

**Problem:**
```rust
#[Object]
impl Query {
    async fn notes(&self, ctx: &Context<'_>) -> Result<Vec<Note>> {
        // Fetch ALL notes first
        let all_notes = db.get_all_notes().await?;
        
        // Then filter - inefficient!
        let mut allowed = vec![];
        for note in all_notes {
            if is_authorized(ctx, &format!("note:{}", note.id), "read").await.is_ok() {
                allowed.push(note);
            }
        }
        Ok(allowed)
    }
}
```

**Solution:**
```rust
#[Object]
impl Query {
    async fn notes(&self, ctx: &Context<'_>) -> Result<Vec<Note>> {
        let auth = ctx.data::<AuthContext>()?;
        let user_id = auth.require_auth()?;
        
        // Only fetch notes user can access
        let notes = db.get_accessible_notes(user_id).await?;
        Ok(notes)
    }
}
```

### 2. Missing Authorization on Mutations

**Problem:**
```rust
async fn update_note(&self, ctx: &Context<'_>, id: ID, input: UpdateInput) -> Result<Note> {
    // No authorization check!
    db.update_note(id, input).await
}
```

**Solution:**
```rust
async fn update_note(&self, ctx: &Context<'_>, id: ID, input: UpdateInput) -> Result<Note> {
    // Always check first
    is_authorized(ctx, &format!("note:{}", id), "write").await?;
    db.update_note(id, input).await
}
```

## SpiceDB Integration Issues

### 1. Wrong Resource Format

**Problem:**
```rust
// SpiceDB expects "type:id" format
is_authorized(ctx, "note-123", "read").await // Wrong format!
```

**Solution:**
```rust
is_authorized(ctx, "note:123", "read").await // Correct format
```

### 2. Missing Relationship Creation

**Problem:**
```rust
async fn create_note(&self, input: CreateInput) -> Result<Note> {
    let note = db.create_note(input).await?;
    // Forgot to create SpiceDB relationship!
    Ok(note)
}
```

**Solution:**
```rust
async fn create_note(&self, ctx: &Context<'_>, input: CreateInput) -> Result<Note> {
    let auth = ctx.data::<AuthContext>()?;
    let user_id = auth.require_auth()?;
    
    let note = db.create_note(input).await?;
    
    // Create ownership relationship
    let spicedb = ctx.data::<SpiceDBClient>()?;
    spicedb.create_relationship(
        &format!("note:{}", note.id),
        "owner",
        &format!("user:{}", user_id),
    ).await?;
    
    Ok(note)
}
```

## Debugging Techniques

### 1. Enable Detailed Logging

```rust
// Add tracing to authorization
pub async fn is_authorized(ctx: &Context<'_>, resource: &str, action: &str) -> Result<(), Error> {
    let span = tracing::info_span!(
        "authorization",
        resource = %resource,
        action = %action,
    );
    let _enter = span.enter();
    
    let auth = ctx.data::<AuthContext>()?;
    tracing::debug!("Auth context: {:?}", auth);
    
    // ... rest of authorization logic
}
```

### 2. Add Debug Endpoints

```rust
// Development only!
#[cfg(feature = "debug")]
async fn debug_auth(&self, ctx: &Context<'_>) -> Result<DebugInfo> {
    let auth = ctx.data::<AuthContext>()?;
    let cache = ctx.data::<Arc<AuthCache>>()?;
    let circuit = ctx.data::<Arc<CircuitBreaker>>()?;
    
    Ok(DebugInfo {
        authenticated: auth.is_authenticated(),
        user_id: auth.user_id.clone(),
        cache_size: cache.size().await,
        cache_hit_rate: cache.hit_rate(),
        circuit_state: circuit.state().await,
    })
}
```

### 3. Test Individual Components

```rust
#[tokio::test]
async fn test_authorization_components() {
    // Test cache independently
    let cache = AuthCache::new(100);
    cache.set("test:key".into(), true, Duration::from_secs(60)).await;
    assert_eq!(cache.get("test:key").await, Some(true));
    
    // Test circuit breaker independently
    let breaker = CircuitBreaker::new();
    let failing_op = || Box::pin(async { Err::<(), _>("error") });
    
    for _ in 0..3 {
        let _ = breaker.call(failing_op).await;
    }
    assert!(breaker.is_open().await);
    
    // Test fallback rules independently
    assert!(apply_fallback_rules("alice", "health:status", "read"));
    assert!(!apply_fallback_rules("alice", "note:123", "write"));
}
```

## Common Patterns That Cause Issues

### 1. Async Context Loss

**Problem:**
```rust
// Context not available in spawned task
tokio::spawn(async move {
    is_authorized(ctx, resource, action).await // ctx not Send!
});
```

**Solution:**
```rust
// Extract what you need first
let auth_service = ctx.data::<Arc<AuthorizationService>>()?;
let auth_context = ctx.data::<AuthContext>()?.clone();

tokio::spawn(async move {
    auth_service.check_with_context(&auth_context, resource, action).await
});
```

### 2. Incorrect Error Propagation

**Problem:**
```rust
// Swallowing specific error info
match is_authorized(ctx, resource, action).await {
    Ok(_) => { /* proceed */ }
    Err(_) => return Err(Error::new("Error")), // Lost details!
}
```

**Solution:**
```rust
// Preserve error context
match is_authorized(ctx, resource, action).await {
    Ok(_) => { /* proceed */ }
    Err(e) => return Err(e), // Propagate full error
}

// Or add context
is_authorized(ctx, resource, action)
    .await
    .map_err(|e| e.extend_with(|_, ext| {
        ext.set("resource", resource);
        ext.set("action", action);
    }))?;
```

## Performance Issues

### 1. N+1 Authorization Queries

**Problem:**
```rust
// Checking permission for each item separately
for note in notes {
    if is_authorized(ctx, &format!("note:{}", note.id), "read").await.is_ok() {
        results.push(note);
    }
}
```

**Solution:**
```rust
// Batch authorization checks
let checks: Vec<_> = notes.iter()
    .map(|note| (format!("note:{}", note.id), "read".to_string()))
    .collect();

let permissions = batch_authorize(ctx, checks).await?;

let allowed_notes: Vec<_> = notes.into_iter()
    .zip(permissions)
    .filter_map(|(note, allowed)| if allowed { Some(note) } else { None })
    .collect();
```

### 2. Cache Key Explosion

**Problem:**
```rust
// Including timestamp in cache key
let key = format!("{}:{}:{}:{}", user_id, resource, action, Utc::now());
// Every request creates new key!
```

**Solution:**
```rust
// Only include what affects authorization
let key = format!("{}:{}:{}", user_id, resource, action);
```

## Testing Authorization

### 1. Test Matrix

```rust
#[tokio::test]
async fn test_authorization_matrix() {
    let scenarios = vec![
        // (user, resource, action, expected)
        ("alice", "note:alice-1", "read", true),
        ("alice", "note:alice-1", "write", true),
        ("bob", "note:alice-1", "read", false),
        ("bob", "note:alice-1", "write", false),
    ];
    
    for (user, resource, action, expected) in scenarios {
        let ctx = create_test_context(user);
        let result = is_authorized(&ctx, resource, action).await;
        
        assert_eq!(
            result.is_ok(),
            expected,
            "Failed for {} {} {}",
            user, resource, action
        );
    }
}
```

### 2. Test Error Cases

```rust
#[tokio::test]
async fn test_error_handling() {
    // No auth context
    let ctx = Context::new();
    let result = is_authorized(&ctx, "note:123", "read").await;
    assert!(result.is_err());
    
    // Not authenticated
    let ctx = create_test_context(None);
    let result = is_authorized(&ctx, "note:123", "read").await;
    assert_eq!(
        result.unwrap_err().extensions().get("code"),
        Some(&Value::from("UNAUTHORIZED"))
    );
}
```

## Quick Debugging Checklist

When authorization isn't working:

1. **Check authentication first**
   - Is user_id present in context?
   - Is the session valid?

2. **Verify cache behavior**
   - Are you seeing cache hits?
   - Is the cache key format consistent?
   - Remember: only positive results are cached

3. **Check circuit breaker state**
   - Is it open when it should be?
   - Are timeouts being counted as failures?

4. **Validate SpiceDB format**
   - Resource format: "type:id"
   - Subject format: "user:id"
   - Permission names match schema

5. **Review fallback rules**
   - Are they appropriately conservative?
   - Do they handle all resource types?

6. **Check error types**
   - 401 for no authentication
   - 403 for no authorization

## Common Solutions Summary

1. **"It always returns 403"** → Check if user is authenticated first
2. **"Cache never works"** → Verify you're only caching positive results
3. **"Circuit breaker won't open"** → Count timeouts as failures
4. **"Fallback allows too much"** → Make rules more conservative
5. **"SpiceDB calls fail"** → Check resource format is "type:id"
6. **"Tests are flaky"** → Mock SpiceDB, don't use real service

For more details, see:
- [Authorization Tutorial](./authorization-tutorial.md) - Complete implementation
- [Circuit Breaker Guide](./circuit-breaker-guide.md) - Resilience patterns
- [Cache Strategies Guide](./cache-strategies-guide.md) - Caching patterns
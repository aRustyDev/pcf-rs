# Authorization TDD Examples for Phase 4

## Test-Driven Development Review

Remember the TDD cycle:
1. **RED** - Write a failing test
2. **GREEN** - Write minimal code to pass
3. **REFACTOR** - Clean up while keeping tests green

For authorization, this means writing security tests FIRST!

## Starting Simple: Basic Authorization Tests

### 1. Test Authentication Requirement

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{Context, Error};
    
    #[tokio::test]
    async fn test_requires_authentication() {
        // Arrange - Context without authentication
        let ctx = Context::new();
        
        // Act - Try to authorize
        let result = is_authorized(&ctx, "note:123", "read").await;
        
        // Assert - Should fail with UNAUTHORIZED
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.extensions().get("code"),
            Some(&serde_json::Value::from("UNAUTHORIZED"))
        );
    }
}
```

### 2. Implement Minimal Code to Pass

```rust
pub async fn is_authorized(
    ctx: &Context<'_>,
    resource: &str,
    action: &str,
) -> Result<(), Error> {
    // Just check for auth context
    let auth = ctx.data::<AuthContext>()
        .map_err(|_| Error::new("Internal error: auth context missing"))?;
    
    if !auth.is_authenticated() {
        return Err(Error::new("Authentication required")
            .extend_with(|_, e| e.set("code", "UNAUTHORIZED")));
    }
    
    Ok(()) // For now, authenticated = authorized
}
```

### 3. Add More Specific Tests

```rust
#[tokio::test]
async fn test_owner_can_write() {
    // Arrange
    let mut ctx = Context::new();
    ctx.insert_data(AuthContext {
        user_id: Some("alice".to_string()),
        trace_id: "test".to_string(),
        session_token: None,
    });
    
    // Mock SpiceDB to return true for alice:note:123:write
    let mock_spicedb = Arc::new(MockSpiceDB::new());
    mock_spicedb.allow("user:alice", "note:123", "write").await;
    ctx.insert_data(mock_spicedb);
    
    // Act
    let result = is_authorized(&ctx, "note:123", "write").await;
    
    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_non_owner_cannot_write() {
    // Arrange
    let mut ctx = Context::new();
    ctx.insert_data(AuthContext {
        user_id: Some("bob".to_string()),
        trace_id: "test".to_string(),
        session_token: None,
    });
    
    let mock_spicedb = Arc::new(MockSpiceDB::new());
    // Don't add permission for Bob
    ctx.insert_data(mock_spicedb);
    
    // Act
    let result = is_authorized(&ctx, "note:123", "write").await;
    
    // Assert
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().extensions().get("code"),
        Some(&serde_json::Value::from("FORBIDDEN"))
    );
}
```

## Testing Cache Behavior

### 1. Test Positive-Only Caching

```rust
#[tokio::test]
async fn test_only_caches_positive_authorization() {
    // Arrange
    let cache = Arc::new(AuthCache::new(100));
    let mock_spicedb = Arc::new(MockSpiceDB::new());
    
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    ctx.insert_data(cache.clone());
    ctx.insert_data(mock_spicedb.clone());
    
    // First check - denied
    let result1 = is_authorized(&ctx, "note:forbidden", "write").await;
    assert!(result1.is_err());
    
    // Cache should be empty (negative not cached)
    assert!(cache.get("alice:note:forbidden:write").await.is_none());
    
    // Allow permission
    mock_spicedb.allow("user:alice", "note:allowed", "write").await;
    
    // Second check - allowed
    let result2 = is_authorized(&ctx, "note:allowed", "write").await;
    assert!(result2.is_ok());
    
    // Cache should contain positive result
    assert_eq!(cache.get("alice:note:allowed:write").await, Some(true));
}
```

### 2. Test Cache Hit Performance

```rust
#[tokio::test]
async fn test_cache_improves_performance() {
    // Arrange
    let cache = Arc::new(AuthCache::new(100));
    let mock_spicedb = Arc::new(SlowMockSpiceDB::new(Duration::from_millis(50)));
    mock_spicedb.allow("user:alice", "note:123", "read").await;
    
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    ctx.insert_data(cache);
    ctx.insert_data(mock_spicedb);
    
    // First call - slow (hits SpiceDB)
    let start1 = Instant::now();
    let result1 = is_authorized(&ctx, "note:123", "read").await;
    let duration1 = start1.elapsed();
    assert!(result1.is_ok());
    assert!(duration1 >= Duration::from_millis(50));
    
    // Second call - fast (cache hit)
    let start2 = Instant::now();
    let result2 = is_authorized(&ctx, "note:123", "read").await;
    let duration2 = start2.elapsed();
    assert!(result2.is_ok());
    assert!(duration2 < Duration::from_millis(5));
}
```

## Testing Circuit Breaker

### 1. Test Circuit Opens on Failures

```rust
#[tokio::test]
async fn test_circuit_breaker_opens_after_failures() {
    // Arrange
    let circuit_breaker = Arc::new(CircuitBreaker::with_config(
        3,  // failure_threshold
        2,  // success_threshold
        Duration::from_millis(100), // timeout
    ));
    
    let failing_spicedb = Arc::new(FailingSpiceDB::new());
    
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    ctx.insert_data(circuit_breaker.clone());
    ctx.insert_data(failing_spicedb);
    
    // Act - Make requests until circuit opens
    for i in 0..4 {
        let _ = is_authorized(&ctx, "note:123", "read").await;
        
        // After 3rd failure, circuit should be open
        if i >= 2 {
            assert!(circuit_breaker.is_open().await);
        }
    }
    
    // Assert - Circuit is open and using fallback
    let result = is_authorized(&ctx, "health:status", "read").await;
    assert!(result.is_ok()); // Fallback allows health checks
    
    let result = is_authorized(&ctx, "note:123", "write").await;
    assert!(result.is_err()); // Fallback denies writes
}
```

### 2. Test Circuit Recovery

```rust
#[tokio::test]
async fn test_circuit_breaker_recovers() {
    // Arrange
    let circuit_breaker = Arc::new(CircuitBreaker::with_config(2, 2, Duration::from_millis(50)));
    let mock_spicedb = Arc::new(MockSpiceDB::new());
    
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    ctx.insert_data(circuit_breaker.clone());
    ctx.insert_data(mock_spicedb.clone());
    
    // Open the circuit with failures
    mock_spicedb.set_failing(true).await;
    for _ in 0..2 {
        let _ = is_authorized(&ctx, "note:123", "read").await;
    }
    assert!(circuit_breaker.is_open().await);
    
    // Fix the service
    mock_spicedb.set_failing(false).await;
    mock_spicedb.allow("user:alice", "note:123", "read").await;
    
    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(60)).await;
    
    // Make successful requests to close circuit
    for _ in 0..2 {
        let result = is_authorized(&ctx, "note:123", "read").await;
        assert!(result.is_ok());
    }
    
    // Circuit should be closed
    assert!(!circuit_breaker.is_open().await);
}
```

## Testing GraphQL Integration

### 1. Test Query Authorization

```rust
#[tokio::test]
async fn test_graphql_query_requires_authorization() {
    // Arrange
    let schema = create_test_schema();
    
    // Test without authentication
    let query = r#"
        query {
            note(id: "123") {
                id
                title
            }
        }
    "#;
    
    let response = schema.execute(query).await;
    
    // Should have authorization error
    assert!(!response.errors.is_empty());
    assert_eq!(
        response.errors[0].extensions.get("code"),
        Some(&serde_json::Value::from("UNAUTHORIZED"))
    );
}

#[tokio::test]
async fn test_graphql_query_with_valid_authorization() {
    // Arrange
    let schema = create_test_schema();
    let mock_spicedb = schema.data::<Arc<MockSpiceDB>>().unwrap();
    mock_spicedb.allow("user:alice", "note:123", "read").await;
    
    // Create request with auth context
    let request = Request::new(r#"
        query {
            note(id: "123") {
                id
                title
            }
        }
    "#)
    .data(auth_context("alice"));
    
    let response = schema.execute(request).await;
    
    // Should succeed
    assert!(response.errors.is_empty());
    assert!(response.data.get("note").is_some());
}
```

### 2. Test Mutation Authorization

```rust
#[tokio::test]
async fn test_mutation_requires_write_permission() {
    // Arrange
    let schema = create_test_schema();
    let mock_spicedb = schema.data::<Arc<MockSpiceDB>>().unwrap();
    
    // Alice can read but not write
    mock_spicedb.allow("user:alice", "note:123", "read").await;
    
    let request = Request::new(r#"
        mutation {
            updateNote(id: "123", input: { title: "New Title" }) {
                id
                title
            }
        }
    "#)
    .data(auth_context("alice"));
    
    let response = schema.execute(request).await;
    
    // Should fail with FORBIDDEN
    assert!(!response.errors.is_empty());
    assert_eq!(
        response.errors[0].extensions.get("code"),
        Some(&serde_json::Value::from("FORBIDDEN"))
    );
}
```

## Testing Fallback Rules

### 1. Test Conservative Fallback

```rust
#[tokio::test]
async fn test_fallback_rules_are_conservative() {
    // Test matrix of fallback scenarios
    let test_cases = vec![
        // (user, resource, action, expected)
        ("alice", "health:status", "read", true),   // Always allow health
        ("alice", "user:alice", "read", true),      // Own profile
        ("alice", "user:bob", "read", false),       // Other's profile
        ("alice", "note:123", "read", false),       // No note access
        ("alice", "note:123", "write", false),      // No write access
        ("alice", "org:acme", "admin", false),      // No admin access
    ];
    
    for (user, resource, action, expected) in test_cases {
        let result = apply_fallback_rules(user, resource, action);
        assert_eq!(
            result, expected,
            "Fallback failed for {} {} {}",
            user, resource, action
        );
    }
}
```

### 2. Test Fallback During Outage

```rust
#[tokio::test]
async fn test_fallback_during_spicedb_outage() {
    // Arrange - SpiceDB is completely down
    let circuit_breaker = Arc::new(CircuitBreaker::new());
    circuit_breaker.force_open().await; // Simulate outage
    
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    ctx.insert_data(circuit_breaker);
    ctx.insert_data(Arc::new(FailingSpiceDB::new()));
    
    // Health checks should still work
    let health_result = is_authorized(&ctx, "health:status", "read").await;
    assert!(health_result.is_ok());
    
    // User can read own profile
    let profile_result = is_authorized(&ctx, "user:alice", "read").await;
    assert!(profile_result.is_ok());
    
    // But not others' data
    let other_result = is_authorized(&ctx, "user:bob", "read").await;
    assert!(other_result.is_err());
}
```

## Testing Batch Authorization

### 1. Test Batch Permission Checks

```rust
#[tokio::test]
async fn test_batch_authorization() {
    // Arrange
    let mock_spicedb = Arc::new(MockSpiceDB::new());
    mock_spicedb.allow("user:alice", "note:1", "read").await;
    mock_spicedb.allow("user:alice", "note:3", "read").await;
    // Note 2 not allowed
    
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    ctx.insert_data(mock_spicedb);
    
    // Act - Check multiple resources
    let checks = vec![
        ("note:1".to_string(), "read".to_string()),
        ("note:2".to_string(), "read".to_string()),
        ("note:3".to_string(), "read".to_string()),
    ];
    
    let results = batch_authorize(&ctx, checks).await.unwrap();
    
    // Assert
    assert_eq!(results, vec![true, false, true]);
}
```

## Testing Error Scenarios

### 1. Test Timeout Handling

```rust
#[tokio::test]
async fn test_spicedb_timeout_uses_fallback() {
    // Arrange - SpiceDB that's very slow
    let slow_spicedb = Arc::new(SlowMockSpiceDB::with_delay(Duration::from_secs(10)));
    
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    ctx.insert_data(Arc::new(CircuitBreaker::new()));
    ctx.insert_data(slow_spicedb);
    
    // Act - Should timeout and use fallback
    let start = Instant::now();
    let result = is_authorized(&ctx, "note:123", "read").await;
    let duration = start.elapsed();
    
    // Assert - Timed out quickly, not after 10 seconds
    assert!(duration < Duration::from_secs(3));
    assert!(result.is_err()); // Fallback denies note access
}
```

### 2. Test Missing Context Data

```rust
#[tokio::test]
async fn test_missing_auth_context() {
    let ctx = Context::new();
    // No auth context inserted
    
    let result = is_authorized(&ctx, "note:123", "read").await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("auth context"));
}

#[tokio::test]
async fn test_missing_spicedb_client() {
    let mut ctx = Context::new();
    ctx.insert_data(auth_context("alice"));
    // No SpiceDB client inserted
    
    let result = is_authorized(&ctx, "note:123", "read").await;
    
    assert!(result.is_err());
}
```

## Mock Implementations for Testing

### 1. MockSpiceDB

```rust
pub struct MockSpiceDB {
    permissions: Arc<RwLock<HashMap<String, bool>>>,
    failing: Arc<RwLock<bool>>,
}

impl MockSpiceDB {
    pub fn new() -> Self {
        Self {
            permissions: Arc::new(RwLock::new(HashMap::new())),
            failing: Arc::new(RwLock::new(false)),
        }
    }
    
    pub async fn allow(&self, subject: &str, resource: &str, permission: &str) {
        let key = format!("{}:{}:{}", subject, resource, permission);
        self.permissions.write().await.insert(key, true);
    }
    
    pub async fn set_failing(&self, failing: bool) {
        *self.failing.write().await = failing;
    }
    
    pub async fn check_permission(
        &self,
        subject: String,
        resource: String,
        permission: String,
    ) -> Result<bool, Error> {
        if *self.failing.read().await {
            return Err(Error::new("SpiceDB unavailable"));
        }
        
        let key = format!("{}:{}:{}", subject, resource, permission);
        Ok(self.permissions.read().await.get(&key).copied().unwrap_or(false))
    }
}
```

### 2. Test Helpers

```rust
fn auth_context(user_id: &str) -> AuthContext {
    AuthContext {
        user_id: Some(user_id.to_string()),
        trace_id: "test-trace".to_string(),
        session_token: Some("test-token".to_string()),
    }
}

fn create_test_schema() -> Schema<Query, Mutation, EmptySubscription> {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(Arc::new(MockSpiceDB::new()))
        .data(Arc::new(AuthCache::new(100)))
        .data(Arc::new(CircuitBreaker::new()))
        .finish()
}
```

## Best Practices for Authorization Tests

### 1. Test Security Boundaries
```rust
// Always test both success and failure cases
#[test]
fn test_authorization_boundaries() {
    // Can do what they should
    assert!(authorized("owner", "their_resource", "delete"));
    
    // Can't do what they shouldn't
    assert!(!authorized("viewer", "any_resource", "delete"));
    assert!(!authorized("anonymous", "any_resource", "read"));
}
```

### 2. Test Edge Cases
```rust
#[test]
fn test_edge_cases() {
    // Empty strings
    assert!(!authorized("", "resource", "read"));
    assert!(!authorized("user", "", "read"));
    
    // Invalid formats
    assert!(!authorized("user", "invalid-format", "read"));
    assert!(!authorized("user", "note:123:extra", "read"));
}
```

### 3. Test Performance Requirements
```rust
#[test]
fn test_authorization_performance() {
    // Authorization should be fast
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = authorized("user", "resource", "read");
    }
    assert!(start.elapsed() < Duration::from_millis(100));
}
```

## Common TDD Mistakes to Avoid

### 1. Testing Implementation, Not Behavior
```rust
// BAD - Tests internal details
#[test]
fn test_cache_key_format() {
    assert_eq!(build_cache_key("a", "b", "c"), "a:b:c");
}

// GOOD - Tests behavior
#[test]
fn test_caches_authorization_result() {
    authorize("alice", "note:1", "read");
    assert!(was_cached("alice", "note:1", "read"));
}
```

### 2. Not Testing Security First
```rust
// BAD - Feature first, security later
fn create_note() -> Note {
    // Implementation without auth check
}

// GOOD - Security test first
#[test]
fn test_create_note_requires_authentication() {
    let result = create_note_without_auth();
    assert!(result.is_err());
}
```

### 3. Over-Mocking
```rust
// BAD - Mocking everything
let mock_cache = MockCache::new();
let mock_circuit = MockCircuitBreaker::new();
let mock_logger = MockLogger::new();

// GOOD - Mock external services only
let mock_spicedb = MockSpiceDB::new();
let real_cache = AuthCache::new(100);
let real_circuit = CircuitBreaker::new();
```

## Summary

When doing TDD for authorization:

1. **Start with security tests** - Write failing auth tests first
2. **Test boundaries** - Both allowed and denied cases
3. **Mock external services** - But use real components when possible
4. **Test error conditions** - Timeouts, failures, missing data
5. **Verify performance** - Authorization should be fast
6. **Test the full stack** - From GraphQL to SpiceDB

Remember: Every authorization bug is a potential security vulnerability. Test thoroughly!

For implementation details, see:
- [Authorization Tutorial](./authorization-tutorial.md) - Complete implementation
- [Circuit Breaker Guide](./circuit-breaker-guide.md) - Resilience testing
- [SpiceDB Setup Guide](./spicedb-setup-guide.md) - Test environment setup
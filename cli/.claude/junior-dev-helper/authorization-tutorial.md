# Authorization Tutorial for Phase 4

## Introduction

Authorization determines what authenticated users can do. Unlike authentication (who you are), authorization controls what you can access (what you can do).

## Core Concepts

### Authentication vs Authorization

- **Authentication**: Verifying identity (login with username/password)
- **Authorization**: Verifying permissions (can user X do action Y on resource Z?)

```
Authentication: "Are you John?"     → 401 Unauthorized if not
Authorization:  "Can John edit note 123?" → 403 Forbidden if not allowed
```

### The Authorization Pattern

Our system uses a standard pattern across all endpoints:

```rust
// Every resolver starts with this
is_authorized(ctx, resource, action).await?;
```

This single line:
1. Checks if user is authenticated
2. Looks up permissions in cache
3. Queries SpiceDB if needed
4. Falls back to safe rules if SpiceDB is down
5. Logs the decision for audit
6. Returns appropriate error if denied

## Understanding SpiceDB/Zanzibar

SpiceDB implements Google's Zanzibar model for permissions. Think of it as a graph database for permissions.

### Basic Concepts

1. **Resources**: Things to protect (notes, users, organizations)
2. **Subjects**: Who wants access (users, services)
3. **Relations**: How subjects relate to resources (owner, viewer, member)
4. **Permissions**: What actions are allowed (read, write, delete)

### Resource Format

Resources follow a consistent format:
```
type:id
```

Examples:
- `note:123` - Note with ID 123
- `user:alice` - User with ID alice
- `org:acme` - Organization named acme

### SpiceDB Schema Example

```zed
definition user {}

definition note {
    relation owner: user
    relation viewer: user
    
    permission read = viewer + owner
    permission write = owner
    permission delete = owner
}
```

This means:
- Owners can read, write, and delete
- Viewers can only read
- Relations are additive (owner is also a viewer)

## Implementation Steps

### Step 1: Authentication Context

First, extract who the user is:

```rust
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub trace_id: String,
    pub session_token: Option<String>,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }
    
    pub fn require_auth(&self) -> Result<&str, Error> {
        self.user_id.as_deref()
            .ok_or_else(|| Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHORIZED")))
    }
}
```

### Step 2: The is_authorized Helper

This is the heart of authorization:

```rust
pub async fn is_authorized(
    ctx: &Context<'_>,
    resource: &str,
    action: &str,
) -> Result<(), Error> {
    // 1. Check demo mode (development only)
    #[cfg(feature = "demo")]
    if ctx.data::<DemoMode>()?.enabled {
        return Ok(()); // Bypass in demo
    }
    
    // 2. Require authentication
    let auth = ctx.data::<AuthContext>()?;
    let user_id = auth.require_auth()?;
    
    // 3. Check cache first (fast path)
    let cache = ctx.data::<Arc<AuthCache>>()?;
    let cache_key = format!("{}:{}:{}", user_id, resource, action);
    
    if let Some(allowed) = cache.get(&cache_key).await {
        return if allowed {
            Ok(())
        } else {
            Err(Error::new("Permission denied")
                .extend_with(|_, e| e.set("code", "FORBIDDEN")))
        };
    }
    
    // 4. Check with SpiceDB (slow path)
    let allowed = check_with_spicedb(ctx, user_id, resource, action).await?;
    
    // 5. Cache positive results only
    if allowed {
        cache.set(cache_key, true, Duration::from_secs(300)).await;
    }
    
    // 6. Return result
    if allowed {
        Ok(())
    } else {
        Err(Error::new("Permission denied")
            .extend_with(|_, e| e.set("code", "FORBIDDEN")))
    }
}
```

### Step 3: Using in Resolvers

Every GraphQL resolver should check authorization:

```rust
#[Object]
impl Query {
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        // Check authorization FIRST
        is_authorized(ctx, &format!("note:{}", id), "read").await?;
        
        // Then fetch data
        let db = ctx.data::<DatabaseService>()?;
        db.get_note(id.as_str()).await
    }
}

#[Object]
impl Mutation {
    async fn update_note(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateNoteInput,
    ) -> Result<Note> {
        // Different permission for mutations
        is_authorized(ctx, &format!("note:{}", id), "write").await?;
        
        // Update the note
        let db = ctx.data::<DatabaseService>()?;
        db.update_note(id.as_str(), input).await
    }
    
    async fn delete_note(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        // Delete requires specific permission
        is_authorized(ctx, &format!("note:{}", id), "delete").await?;
        
        let db = ctx.data::<DatabaseService>()?;
        db.delete_note(id.as_str()).await
    }
}
```

## Caching Strategy

### Why Cache?

- SpiceDB queries take 10-50ms
- Many repeated permission checks
- Reduce load on SpiceDB
- Improve response times

### Cache Rules

1. **Cache positive results only** - Never cache denials
2. **5-minute TTL** - Balance between performance and freshness
3. **LRU eviction** - Remove least recently used when full
4. **User invalidation** - Clear cache when permissions change

```rust
impl AuthCache {
    pub async fn set(&self, key: String, allowed: bool, ttl: Duration) {
        // SECURITY: Only cache positive results
        if !allowed {
            tracing::debug!("Refusing to cache negative result");
            return;
        }
        
        // Store with expiration
        self.entries.write().await.insert(key, CacheEntry {
            allowed,
            expires_at: Instant::now() + ttl,
        });
    }
}
```

## Resilience Patterns

### Circuit Breaker

Prevents cascading failures when SpiceDB is down:

```rust
let circuit_breaker = CircuitBreaker::new()
    .failure_threshold(3)     // Open after 3 failures
    .success_threshold(2)     // Close after 2 successes
    .timeout(Duration::from_secs(60)); // Try again after 60s

match circuit_breaker.call(spicedb_check).await {
    Ok(result) => result,
    Err(_) => {
        // Circuit open - use fallback
        apply_fallback_rules(user_id, resource, action)
    }
}
```

### Fallback Rules

When SpiceDB is unavailable, apply conservative rules:

```rust
fn apply_fallback_rules(user_id: &str, resource: &str, action: &str) -> bool {
    let (resource_type, resource_id) = parse_resource(resource);
    
    match (resource_type, action) {
        // Always allow health checks
        ("health", "read") => true,
        
        // Users can read their own profile
        ("user", "read") if resource_id == user_id => true,
        
        // Deny everything else (fail secure)
        _ => false,
    }
}
```

## Common Patterns

### 1. Checking Multiple Permissions

```rust
// Check if user can perform any of several actions
async fn can_modify(&self, ctx: &Context<'_>, id: &str) -> bool {
    let resource = format!("note:{}", id);
    
    // Try write permission
    if is_authorized(ctx, &resource, "write").await.is_ok() {
        return true;
    }
    
    // Try admin permission
    if is_authorized(ctx, &resource, "admin").await.is_ok() {
        return true;
    }
    
    false
}
```

### 2. Batch Authorization

For lists, check permissions efficiently:

```rust
async fn notes(&self, ctx: &Context<'_>) -> Result<Vec<Note>> {
    let auth = ctx.data::<AuthContext>()?;
    let user_id = auth.require_auth()?;
    
    // Get all notes
    let all_notes = db.get_all_notes().await?;
    
    // Filter by permission (this could be optimized with batch checks)
    let mut authorized_notes = Vec::new();
    for note in all_notes {
        let resource = format!("note:{}", note.id);
        if is_authorized(ctx, &resource, "read").await.is_ok() {
            authorized_notes.push(note);
        }
    }
    
    Ok(authorized_notes)
}
```

### 3. Creating Resources with Permissions

```rust
async fn create_note(&self, ctx: &Context<'_>, input: CreateNoteInput) -> Result<Note> {
    let auth = ctx.data::<AuthContext>()?;
    let user_id = auth.require_auth()?;
    
    // Create the note
    let note = db.create_note(input).await?;
    
    // Set up permissions in SpiceDB
    let spicedb = ctx.data::<SpiceDBClient>()?;
    spicedb.create_relationship(
        &format!("note:{}", note.id),
        "owner",
        &format!("user:{}", user_id),
    ).await?;
    
    Ok(note)
}
```

## Testing Authorization

### Unit Tests

```rust
#[tokio::test]
async fn test_authorization_denied_without_auth() {
    let ctx = create_test_context(None); // No auth
    
    let result = is_authorized(&ctx, "note:123", "read").await;
    
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().extensions().get("code"),
        Some(&Value::from("UNAUTHORIZED"))
    );
}

#[tokio::test]
async fn test_authorization_uses_cache() {
    let ctx = create_test_context_with_auth("alice");
    let cache = ctx.data::<Arc<AuthCache>>().unwrap();
    
    // Prime cache
    cache.set("alice:note:123:read".into(), true, Duration::from_secs(300)).await;
    
    // Should not hit SpiceDB
    let result = is_authorized(&ctx, "note:123", "read").await;
    assert!(result.is_ok());
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_authorization_flow() {
    let app = create_test_app().await;
    
    // Create a note as Alice
    let create_response = app.graphql_request(r#"
        mutation {
            createNote(input: { title: "Test", content: "Content" }) {
                id
            }
        }
    "#)
    .header("x-user-id", "alice")
    .send()
    .await;
    
    let note_id = create_response.json()["data"]["createNote"]["id"].as_str().unwrap();
    
    // Alice can read her own note
    let read_response = app.graphql_request(&format!(r#"
        query {{
            note(id: "{}") {{
                title
            }}
        }}
    "#, note_id))
    .header("x-user-id", "alice")
    .send()
    .await;
    
    assert!(read_response.status().is_success());
    
    // Bob cannot read Alice's note
    let forbidden_response = app.graphql_request(&format!(r#"
        query {{
            note(id: "{}") {{
                title
            }}
        }}
    "#, note_id))
    .header("x-user-id", "bob")
    .send()
    .await;
    
    assert_eq!(
        forbidden_response.json()["errors"][0]["extensions"]["code"],
        "FORBIDDEN"
    );
}
```

## Best Practices

1. **Always authorize first** - Before any data access
2. **Use consistent resource naming** - `type:id` format
3. **Log authorization decisions** - For audit trail
4. **Cache conservatively** - Only positive results
5. **Fail secure** - Deny when uncertain
6. **Test extensively** - Both success and failure paths
7. **Monitor performance** - Track cache hit rates

## Common Mistakes

1. **Caching negative results** - Security vulnerability!
2. **Forgetting to authorize** - Always check first
3. **Wrong error codes** - 401 vs 403
4. **Trusting client data** - Always verify server-side
5. **Complex fallback rules** - Keep them simple and conservative

## Next Steps

1. Study the [SpiceDB documentation](https://authzed.com/docs)
2. Practice with the [SpiceDB Playground](https://play.authzed.com/)
3. Review our [Circuit Breaker Guide](./circuit-breaker-guide.md)
4. Understand [Cache Strategies](./cache-strategies-guide.md)

Remember: Authorization is critical for security. When in doubt, deny access!
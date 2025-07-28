# Database Layer Tutorial for Phase 2

## Introduction

The database layer is the foundation of data persistence in our API. Phase 2 implements a robust database connection system with SurrealDB, focusing on reliability through retry logic, connection pooling, and graceful error handling.

## Core Concepts

### What is a Database Service Trait?

In Rust, we use traits to define interfaces. Our `DatabaseService` trait defines what ANY database implementation must provide:

```rust
#[async_trait]
pub trait DatabaseService: Send + Sync {
    async fn connect(&self) -> Result<(), DatabaseError>;
    async fn health_check(&self) -> DatabaseHealth;
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError>;
    // ... more methods
}
```

This allows us to:
- Swap databases without changing application code
- Create mock implementations for testing
- Ensure consistent behavior across different databases

### Connection Pooling Explained

Instead of creating a new database connection for each request (slow!), we maintain a pool of reusable connections:

```
Request 1 ─┐
Request 2 ─┼─> Connection Pool ─> Database
Request 3 ─┘   (10 connections)
```

Benefits:
- **Performance**: Reuse existing connections (saves ~100ms per request)
- **Resource Management**: Limit total connections to database
- **Resilience**: Handle connection failures gracefully

## Understanding Retry Logic

### Why Retry?

Databases can be temporarily unavailable due to:
- Network hiccups
- Database restarts
- Temporary overload
- Container startup delays

Our retry strategy ensures the API eventually connects rather than failing immediately.

### Exponential Backoff with Jitter

We use exponential backoff to avoid overwhelming a recovering database:

```
Attempt 1: Wait 1 second
Attempt 2: Wait 2 seconds  
Attempt 3: Wait 4 seconds
Attempt 4: Wait 8 seconds
...
Maximum: Wait 60 seconds
```

**Jitter** adds randomness (0-1000ms) to prevent "thundering herd" - where all clients retry simultaneously:

```rust
// Without jitter: All clients retry at exactly 2 seconds
// With jitter: Clients retry between 2.0 - 3.0 seconds
let delay = base_delay + random_milliseconds(0..1000);
```

## SurrealDB Basics

### What Makes SurrealDB Different?

1. **Thing IDs**: Unique identifiers in format `table:id`
   ```
   note:01234567-89ab-cdef
   user:alice
   ```

2. **Schemaless**: No need to define tables upfront
   
3. **Built-in Relations**: Can link records directly

### Basic Operations

```rust
// Create a note
let id = db.create("note", json!({
    "title": "My Note",
    "content": "Hello World"
})).await?;
// Returns: "note:01234567-89ab-cdef"

// Read by ID
let note = db.read("note", "01234567-89ab-cdef").await?;

// Query with conditions
let notes = db.query("SELECT * FROM note WHERE author = $author", 
    json!({ "author": "alice" })
).await?;
```

## Implementing the Database Layer

### Step 1: Define Your Service Trait

Start with the interface your application needs:

```rust
#[async_trait]
pub trait DatabaseService: Send + Sync {
    // Connection management
    async fn connect(&self) -> Result<(), DatabaseError>;
    async fn disconnect(&self) -> Result<(), DatabaseError>;
    async fn health_check(&self) -> DatabaseHealth;
    
    // CRUD operations
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError>;
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError>;
    async fn update(&self, collection: &str, id: &str, data: Value) -> Result<(), DatabaseError>;
    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError>;
    
    // Queries
    async fn query(&self, query: &str, params: Value) -> Result<Vec<Value>, DatabaseError>;
}
```

### Step 2: Create a Mock Implementation

For testing, create a mock that doesn't need a real database:

```rust
pub struct MockDatabase {
    connected: Arc<RwLock<bool>>,
    data: Arc<RwLock<HashMap<String, HashMap<String, Value>>>>,
}

#[async_trait]
impl DatabaseService for MockDatabase {
    async fn connect(&self) -> Result<(), DatabaseError> {
        *self.connected.write().await = true;
        Ok(())
    }
    
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError> {
        let id = Uuid::new_v4().to_string();
        let mut store = self.data.write().await;
        
        store.entry(collection.to_string())
            .or_insert_with(HashMap::new)
            .insert(id.clone(), data);
            
        Ok(format!("{}:{}", collection, id))
    }
    
    // ... implement other methods
}
```

### Step 3: Implement Connection Pool

A basic connection pool structure:

```rust
pub struct ConnectionPool<T> {
    connections: Arc<RwLock<Vec<PooledConnection<T>>>>,
    semaphore: Arc<Semaphore>,
    config: PoolConfig,
}

pub struct PooledConnection<T> {
    conn: T,
    created_at: Instant,
    last_used: Arc<RwLock<Instant>>,
    id: Uuid,
}

impl<T> ConnectionPool<T> {
    pub async fn acquire(&self) -> Result<PooledConnection<T>, PoolError> {
        // 1. Try to get existing connection
        let mut connections = self.connections.write().await;
        
        if let Some(conn) = connections.pop() {
            // Update last_used time
            *conn.last_used.write().await = Instant::now();
            return Ok(conn);
        }
        
        // 2. Create new connection if under limit
        if connections.len() < self.config.max_connections {
            let conn = self.create_connection().await?;
            return Ok(conn);
        }
        
        // 3. Wait for available connection
        drop(connections); // Release lock while waiting
        
        let permit = self.semaphore.acquire().await?;
        // ... get connection with permit
    }
}
```

### Step 4: Add Retry Logic

Implement retry with exponential backoff:

```rust
pub async fn connect_with_retry(
    db: &dyn DatabaseService,
    max_wait: Duration,
) -> Result<(), DatabaseError> {
    let start = Instant::now();
    let mut backoff = ExponentialBackoff::new();
    
    loop {
        match db.connect().await {
            Ok(()) => {
                tracing::info!("Database connected successfully");
                return Ok(());
            }
            Err(e) => {
                let elapsed = start.elapsed();
                
                if elapsed >= max_wait {
                    tracing::error!("Database connection timeout after {:?}", elapsed);
                    return Err(e);
                }
                
                let delay = backoff.next_delay();
                tracing::warn!(
                    "Database connection failed, retrying in {:?} (attempt {})",
                    delay,
                    backoff.attempt
                );
                
                tokio::time::sleep(delay).await;
            }
        }
    }
}
```

## Error Handling

### Database-Specific Errors

Always create specific error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query timeout after {0}s")]
    QueryTimeout(u64),
    
    #[error("Record not found: {collection}/{id}")]
    NotFound { 
        collection: String, 
        id: String 
    },
    
    #[error("Invalid Thing ID format: {0}")]
    InvalidThingId(String),
    
    #[error("Transaction rolled back: {0}")]
    TransactionRollback(String),
}
```

### Handling Errors Gracefully

```rust
// Don't panic!
let result = db.read("note", id).await.unwrap(); // BAD!

// Handle errors properly
let result = match db.read("note", id).await {
    Ok(Some(note)) => note,
    Ok(None) => {
        // Note doesn't exist - that's OK
        return Ok(None);
    }
    Err(DatabaseError::ConnectionFailed(_)) => {
        // Return 503 Service Unavailable
        return Err(ServiceError::DatabaseUnavailable);
    }
    Err(e) => {
        // Log and return generic error
        tracing::error!("Database error: {}", e);
        return Err(ServiceError::Internal);
    }
};
```

## Testing Your Database Layer

### Unit Tests with Mocks

```rust
#[tokio::test]
async fn test_create_note() {
    // Arrange
    let db = MockDatabase::new();
    db.connect().await.unwrap();
    
    let note_data = json!({
        "title": "Test Note",
        "content": "Test Content"
    });
    
    // Act
    let id = db.create("note", note_data.clone()).await.unwrap();
    
    // Assert
    assert!(id.starts_with("note:"));
    
    let saved = db.read("note", &id.replace("note:", "")).await.unwrap();
    assert_eq!(saved, Some(note_data));
}
```

### Integration Tests with TestContainers

```rust
#[tokio::test]
async fn test_real_database_connection() {
    // Start SurrealDB in container
    let container = SurrealDbContainer::new()
        .with_user("root")
        .with_password("root")
        .start()
        .await;
    
    let port = container.get_port().await;
    let db = SurrealDbAdapter::new(&format!("localhost:{}", port));
    
    // Test connection with retry
    let result = connect_with_retry(&db, Duration::from_secs(30)).await;
    assert!(result.is_ok());
    
    // Cleanup happens automatically when container drops
}
```

## Common Patterns

### 1. Health Checks

```rust
pub async fn database_health_check(db: &dyn DatabaseService) -> HealthStatus {
    match tokio::time::timeout(Duration::from_secs(5), db.health_check()).await {
        Ok(DatabaseHealth::Healthy) => HealthStatus::Healthy,
        Ok(DatabaseHealth::Degraded(reason)) => HealthStatus::Degraded(reason),
        Ok(DatabaseHealth::Unhealthy(reason)) => HealthStatus::Unhealthy(reason),
        Err(_) => HealthStatus::Unhealthy("Health check timeout".to_string()),
    }
}
```

### 2. Transaction Wrapper

```rust
pub async fn with_transaction<F, T>(
    db: &dyn DatabaseService,
    operation: F,
) -> Result<T, DatabaseError>
where
    F: FnOnce() -> Future<Output = Result<T, DatabaseError>>,
{
    db.begin_transaction().await?;
    
    match operation().await {
        Ok(result) => {
            db.commit_transaction().await?;
            Ok(result)
        }
        Err(e) => {
            db.rollback_transaction().await?;
            Err(e)
        }
    }
}
```

### 3. Bulk Operations

```rust
pub async fn bulk_create(
    db: &dyn DatabaseService,
    collection: &str,
    items: Vec<Value>,
) -> Result<Vec<String>, DatabaseError> {
    let mut ids = Vec::with_capacity(items.len());
    
    for item in items {
        match db.create(collection, item).await {
            Ok(id) => ids.push(id),
            Err(e) => {
                // Log error but continue
                tracing::warn!("Failed to create item: {}", e);
            }
        }
    }
    
    Ok(ids)
}
```

## Best Practices

1. **Always Use Timeouts**
   ```rust
   let result = tokio::time::timeout(
       Duration::from_secs(30),
       db.query(complex_query, params)
   ).await??;
   ```

2. **Log Connection Metrics**
   ```rust
   tracing::info!(
       pool_size = %pool.size(),
       active_connections = %pool.active(),
       idle_connections = %pool.idle(),
       "Connection pool status"
   );
   ```

3. **Validate Data Before Saving**
   ```rust
   #[derive(Validate)]
   struct Note {
       #[validate(length(min = 1, max = 100))]
       title: String,
       #[validate(length(max = 10000))]
       content: String,
   }
   
   note.validate()?; // Check before database call
   ```

4. **Use Prepared Statements**
   ```rust
   // Prevents SQL injection
   db.query(
       "SELECT * FROM note WHERE author = $author",
       json!({ "author": user_input })
   ).await?;
   ```

## Troubleshooting

### Connection Failures
- Check database is running: `docker ps`
- Verify connection string: `ws://localhost:8000`
- Check firewall/network settings
- Look for retry attempts in logs

### Performance Issues
- Monitor connection pool metrics
- Check for N+1 queries
- Add database indexes
- Use query explain plans

### Data Inconsistencies
- Ensure proper transaction usage
- Check for race conditions
- Validate data before saving
- Use optimistic locking

## Summary

The database layer is critical for application reliability. Focus on:
- Proper error handling (no unwraps!)
- Connection pooling for performance
- Retry logic for resilience
- Comprehensive testing
- Clear separation of concerns

Remember: The database will fail at some point. Design your system to handle it gracefully!
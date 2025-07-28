# Common Database Errors Guide

## Overview

Database errors can be frustrating, especially when working with async Rust and SurrealDB. This guide covers the most common errors you'll encounter and how to fix them.

## Compilation Errors

### 1. Async Trait Without async-trait Crate

**Error:**
```
error[E0706]: functions in traits cannot be declared `async`
```

**Problem:**
```rust
// Wrong - Rust doesn't support async in traits natively yet
pub trait DatabaseService {
    async fn connect(&self) -> Result<(), Error>;
}
```

**Solution:**
```rust
use async_trait::async_trait;

#[async_trait]
pub trait DatabaseService {
    async fn connect(&self) -> Result<(), Error>;
}
```

### 2. Send + Sync Bounds Missing

**Error:**
```
error: future cannot be sent between threads safely
```

**Problem:**
```rust
// Using non-Send types in async context
let data = Rc::new(RefCell::new(HashMap::new()));
tokio::spawn(async move {
    data.borrow_mut().insert("key", "value"); // Rc is not Send!
});
```

**Solution:**
```rust
// Use Arc + RwLock for thread-safe sharing
let data = Arc::new(RwLock::new(HashMap::new()));
tokio::spawn(async move {
    data.write().await.insert("key", "value");
});
```

### 3. Lifetime Issues with Async

**Error:**
```
error[E0597]: borrowed value does not live long enough
```

**Problem:**
```rust
async fn query_notes(db: &DatabaseService, author: &str) {
    let query = format!("SELECT * FROM note WHERE author = {}", author);
    db.query(&query).await; // query borrowed across await point
}
```

**Solution:**
```rust
async fn query_notes(db: &DatabaseService, author: &str) {
    let query = format!("SELECT * FROM note WHERE author = {}", author);
    db.query(query).await; // Pass owned String
    
    // Or use parameters (better!)
    db.query_with_params(
        "SELECT * FROM note WHERE author = $author",
        json!({ "author": author })
    ).await;
}
```

## Runtime Errors

### 1. Connection Refused

**Error:**
```
Error: Connection refused (os error 111)
```

**Common Causes:**
- Database not running
- Wrong connection URL
- Firewall blocking connection

**Debugging Steps:**
```rust
// Add detailed logging
tracing::info!("Attempting to connect to: {}", connection_url);

// Check if service is running
match TcpStream::connect("localhost:8000").await {
    Ok(_) => tracing::info!("Port is open"),
    Err(e) => tracing::error!("Port check failed: {}", e),
}

// Try with explicit timeout
let result = tokio::time::timeout(
    Duration::from_secs(5),
    db.connect()
).await;

match result {
    Ok(Ok(())) => tracing::info!("Connected successfully"),
    Ok(Err(e)) => tracing::error!("Connection error: {}", e),
    Err(_) => tracing::error!("Connection timeout"),
}
```

### 2. Thing ID Format Errors

**Error:**
```
Error: Invalid Thing ID format: "123"
```

**Problem:**
```rust
// Missing table prefix
let note = db.select("123").await?;

// Wrong separator
let note = db.select("note-123").await?;
```

**Solution:**
```rust
// Correct format: table:id
let note = db.select(("note", "123")).await?;

// Or as string
let note = db.select("note:123").await?;

// Parse and validate
fn parse_thing_id(input: &str) -> Result<(String, String), Error> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 2 {
        return Err(Error::InvalidThingId(input.to_string()));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}
```

### 3. Query Timeout

**Error:**
```
Error: Query timeout after 30 seconds
```

**Problem:**
```rust
// Long-running query without timeout
let results = db.query("SELECT * FROM massive_table").await?;
```

**Solution:**
```rust
// Add query timeout
let results = tokio::time::timeout(
    Duration::from_secs(30),
    db.query("SELECT * FROM massive_table LIMIT 1000")
).await??;

// Better: Add pagination
async fn paginated_query(db: &DatabaseService, page_size: usize) {
    let mut start = 0;
    loop {
        let query = format!(
            "SELECT * FROM massive_table LIMIT {} START {}",
            page_size, start
        );
        
        let results = db.query(&query).await?;
        if results.is_empty() {
            break;
        }
        
        // Process batch
        process_batch(results).await?;
        start += page_size;
    }
}
```

### 4. Connection Pool Exhausted

**Error:**
```
Error: Connection pool timeout - no connections available
```

**Problem:**
```rust
// Not returning connections to pool
for i in 0..100 {
    let conn = pool.acquire().await?;
    // Forgot to drop connection!
    do_work(&conn).await?;
}
```

**Solution:**
```rust
// Connections returned when dropped
for i in 0..100 {
    let conn = pool.acquire().await?;
    do_work(&conn).await?;
    // conn dropped here, returns to pool
}

// Or use a guard pattern
struct PooledConnection<T> {
    conn: Option<T>,
    pool: Arc<ConnectionPool<T>>,
}

impl<T> Drop for PooledConnection<T> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.return_connection(conn);
        }
    }
}
```

## Transaction Errors

### 1. Deadlocks

**Error:**
```
Error: Transaction deadlock detected
```

**Problem:**
```rust
// Transaction 1
let tx1 = db.begin().await?;
tx1.update("user:alice", data).await?;
tx1.update("user:bob", data).await?;

// Transaction 2 (concurrent)
let tx2 = db.begin().await?;
tx2.update("user:bob", data).await?;  // Locks bob
tx2.update("user:alice", data).await?; // Waits for alice (deadlock!)
```

**Solution:**
```rust
// Always lock resources in consistent order
async fn transfer_funds(from: &str, to: &str, amount: f64) {
    // Sort IDs to ensure consistent lock order
    let (first, second) = if from < to {
        (from, to)
    } else {
        (to, from)
    };
    
    let tx = db.begin().await?;
    tx.update(first, data).await?;
    tx.update(second, data).await?;
    tx.commit().await?;
}
```

### 2. Transaction Already Committed/Rolled Back

**Error:**
```
Error: Transaction already completed
```

**Problem:**
```rust
let tx = db.begin().await?;
tx.create("note", data).await?;
tx.commit().await?;
tx.create("note", more_data).await?; // Error - tx already committed!
```

**Solution:**
```rust
// Use transaction guard pattern
async fn with_transaction<F, T>(db: &Database, f: F) -> Result<T, Error>
where
    F: FnOnce(&Transaction) -> Future<Output = Result<T, Error>>,
{
    let tx = db.begin().await?;
    
    match f(&tx).await {
        Ok(result) => {
            tx.commit().await?;
            Ok(result)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

// Usage
let result = with_transaction(&db, |tx| async move {
    tx.create("note", data).await?;
    tx.create("note", more_data).await?;
    Ok(())
}).await?;
```

## Validation Errors

### 1. Missing Required Fields

**Error:**
```
Error: Field 'title' is required but not provided
```

**Problem:**
```rust
let note = json!({
    "content": "My content"
    // Missing title!
});
db.create("note", note).await?;
```

**Solution:**
```rust
// Validate before sending to database
#[derive(Validate, Serialize, Deserialize)]
struct Note {
    #[validate(length(min = 1, max = 100))]
    title: String,
    
    #[validate(length(max = 10000))]
    content: String,
}

// Check validation
let note = Note {
    title: String::new(), // Empty!
    content: "Content".to_string(),
};

match note.validate() {
    Ok(_) => db.create("note", note).await?,
    Err(e) => return Err(Error::ValidationFailed(e)),
}
```

### 2. Type Mismatches

**Error:**
```
Error: Expected string, got number for field 'created_at'
```

**Solution:**
```rust
// Use proper types and serialization
#[derive(Serialize, Deserialize)]
struct Note {
    title: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

// Or use type conversion
let data = json!({
    "title": "My Note",
    "created_at": Utc::now().to_rfc3339(), // Convert to string
});
```

## Performance Issues

### 1. N+1 Queries

**Problem:**
```rust
// Fetch all notes
let notes = db.query("SELECT * FROM note").await?;

// Then fetch author for each (N queries!)
for note in notes {
    let author = db.select(note.author_id).await?;
    // Process...
}
```

**Solution:**
```rust
// Fetch with join in single query
let notes_with_authors = db.query(r#"
    SELECT *, author.* as author_data 
    FROM note
"#).await?;

// Or batch fetch authors
let author_ids: Vec<_> = notes.iter()
    .map(|n| &n.author_id)
    .collect();

let authors = db.query(r#"
    SELECT * FROM user 
    WHERE id IN $author_ids
"#)
.bind(("author_ids", author_ids))
.await?;
```

### 2. Missing Indexes

**Symptom:** Queries getting slower as data grows

**Solution:**
```sql
-- Add indexes for common queries
DEFINE INDEX created_idx ON TABLE note COLUMNS created_at;
DEFINE INDEX author_idx ON TABLE note COLUMNS author;
DEFINE INDEX email_idx ON TABLE user COLUMNS email UNIQUE;

-- Compound index for complex queries
DEFINE INDEX author_created_idx ON TABLE note COLUMNS author, created_at;
```

## Debugging Techniques

### 1. Enable Query Logging

```rust
// Log all queries
pub struct LoggingDatabase<T> {
    inner: T,
}

#[async_trait]
impl<T: DatabaseService> DatabaseService for LoggingDatabase<T> {
    async fn query(&self, query: &str) -> Result<Vec<Value>, Error> {
        let start = Instant::now();
        tracing::debug!("Executing query: {}", query);
        
        let result = self.inner.query(query).await;
        
        tracing::debug!(
            "Query completed in {:?}", 
            start.elapsed()
        );
        
        result
    }
}
```

### 2. Connection Pool Monitoring

```rust
// Add metrics to pool
impl ConnectionPool {
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            total: self.connections.read().len(),
            available: self.available_count(),
            in_use: self.in_use_count(),
            wait_time: self.avg_wait_time(),
        }
    }
}

// Log periodically
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let stats = pool.stats();
        tracing::info!(?stats, "Connection pool status");
    }
});
```

### 3. Test Helpers

```rust
// Helper to test error conditions
async fn test_connection_failure() {
    let db = MockDatabase::new();
    db.set_failing(true);
    
    let result = connect_with_retry(&db, Duration::from_secs(1)).await;
    assert!(matches!(result, Err(DatabaseError::ConnectionFailed(_))));
}

// Helper to simulate slow queries
impl MockDatabase {
    pub fn with_latency(&mut self, latency: Duration) {
        self.latency = Some(latency);
    }
    
    async fn query(&self, query: &str) -> Result<Vec<Value>, Error> {
        if let Some(latency) = self.latency {
            tokio::time::sleep(latency).await;
        }
        // ... rest of implementation
    }
}
```

## Best Practices to Avoid Errors

1. **Always use timeouts**
   ```rust
   tokio::time::timeout(Duration::from_secs(30), operation).await??
   ```

2. **Validate inputs early**
   ```rust
   data.validate().map_err(|e| Error::ValidationFailed(e))?;
   ```

3. **Use transactions for consistency**
   ```rust
   with_transaction(&db, |tx| async { /* ... */ }).await?
   ```

4. **Handle None cases explicitly**
   ```rust
   match db.select(id).await? {
       Some(record) => process(record),
       None => return Err(Error::NotFound),
   }
   ```

5. **Log errors with context**
   ```rust
   tracing::error!(
       error = %e,
       query = %query,
       duration = ?elapsed,
       "Query failed"
   );
   ```

Remember: Most database errors are either configuration issues (connection strings, missing services) or logic errors (wrong queries, missing error handling). Take time to understand the error message - it usually points directly to the problem!
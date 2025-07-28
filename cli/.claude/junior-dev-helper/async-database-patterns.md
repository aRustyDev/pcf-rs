# Async Database Patterns Guide

## Understanding Async in Rust

Async Rust can be confusing at first, especially with databases. This guide covers common patterns and pitfalls.

## The Basics

### What Makes a Function Async?

```rust
// Synchronous function - blocks thread
fn get_data() -> String {
    thread::sleep(Duration::from_secs(1)); // Blocks!
    "data".to_string()
}

// Asynchronous function - doesn't block
async fn get_data_async() -> String {
    tokio::time::sleep(Duration::from_secs(1)).await; // Yields!
    "data".to_string()
}
```

Key differences:
- Async functions return a `Future`
- Must use `.await` to get the result
- Can yield control to other tasks

### async-trait for Database Interfaces

Rust doesn't natively support async in traits yet, so we use the `async-trait` crate:

```rust
use async_trait::async_trait;

// Without async-trait (doesn't compile!)
trait DatabaseService {
    async fn connect(&self) -> Result<(), Error>; // Error!
}

// With async-trait (works!)
#[async_trait]
trait DatabaseService {
    async fn connect(&self) -> Result<(), Error>;
}

// Implementation also needs the attribute
#[async_trait]
impl DatabaseService for MyDatabase {
    async fn connect(&self) -> Result<(), Error> {
        // Implementation
    }
}
```

## Common Async Patterns

### 1. Sequential vs Concurrent Operations

```rust
// SEQUENTIAL - Slow (waits for each)
async fn get_multiple_notes_sequential(db: &dyn DatabaseService) -> Vec<Note> {
    let mut notes = Vec::new();
    
    for id in &["1", "2", "3"] {
        if let Ok(Some(note)) = db.read("note", id).await {
            notes.push(note);
        }
    }
    
    notes // Takes 3x as long
}

// CONCURRENT - Fast (runs in parallel)
async fn get_multiple_notes_concurrent(db: &dyn DatabaseService) -> Vec<Note> {
    let futures: Vec<_> = ["1", "2", "3"]
        .iter()
        .map(|id| db.read("note", id))
        .collect();
    
    let results = futures::future::join_all(futures).await;
    
    results.into_iter()
        .filter_map(|r| r.ok().flatten())
        .collect()
}
```

### 2. Timeout Pattern

Always add timeouts to database operations:

```rust
use tokio::time::timeout;

async fn query_with_timeout(
    db: &dyn DatabaseService,
    query: &str,
) -> Result<Vec<Value>, Error> {
    match timeout(Duration::from_secs(30), db.query(query)).await {
        Ok(Ok(results)) => Ok(results),
        Ok(Err(e)) => Err(e), // Database error
        Err(_) => Err(Error::QueryTimeout), // Timeout elapsed
    }
}

// Even better - configurable timeout
async fn query_with_config(
    db: &dyn DatabaseService,
    query: &str,
    timeout_duration: Duration,
) -> Result<Vec<Value>, Error> {
    timeout(timeout_duration, db.query(query))
        .await
        .map_err(|_| Error::QueryTimeout)?
}
```

### 3. Retry with Async

```rust
async fn retry_async<F, Fut, T, E>(
    mut operation: F,
    max_attempts: u32,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    
    loop {
        attempt += 1;
        
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= max_attempts => return Err(e),
            Err(e) => {
                tracing::warn!(
                    "Attempt {} failed: {}, retrying...",
                    attempt, e
                );
                tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
            }
        }
    }
}

// Usage
let result = retry_async(
    || async { db.connect().await },
    3
).await?;
```

## Arc and RwLock Patterns

### When to Use Arc<RwLock<T>>

```rust
// Shared state between async tasks
pub struct ConnectionPool {
    // Arc allows multiple owners
    // RwLock allows multiple readers OR one writer
    connections: Arc<RwLock<Vec<Connection>>>,
}

impl ConnectionPool {
    pub async fn acquire(&self) -> Option<Connection> {
        // Multiple tasks can read simultaneously
        let mut conns = self.connections.write().await;
        conns.pop()
    }
    
    pub async fn return_connection(&self, conn: Connection) {
        // Only one task can write at a time
        let mut conns = self.connections.write().await;
        conns.push(conn);
    }
    
    pub async fn size(&self) -> usize {
        // Many tasks can check size concurrently
        self.connections.read().await.len()
    }
}
```

### Common Arc Pitfalls

```rust
// BAD - Holding lock across await point
async fn bad_pattern(pool: &ConnectionPool) {
    let conns = pool.connections.read().await;
    
    // DON'T DO THIS - holds lock during async operation!
    expensive_async_operation().await;
    
    println!("Connections: {}", conns.len());
} // Lock finally released here

// GOOD - Release lock before await
async fn good_pattern(pool: &ConnectionPool) {
    let size = {
        let conns = pool.connections.read().await;
        conns.len()
    }; // Lock released here
    
    expensive_async_operation().await;
    
    println!("Connections: {}", size);
}
```

## Send + Sync Requirements

### Understanding the Bounds

```rust
// This trait requires Send + Sync
#[async_trait]
pub trait DatabaseService: Send + Sync {
    async fn query(&self, q: &str) -> Result<Vec<Value>, Error>;
}

// Why? Because it might be used across threads:
async fn spawn_query(db: Arc<dyn DatabaseService>) {
    tokio::spawn(async move {
        db.query("SELECT * FROM notes").await;
    }); // db must be Send to move into spawned task
}
```

### Common Send + Sync Issues

```rust
// BAD - Rc is not Send
struct BadDatabase {
    cache: Rc<RefCell<HashMap<String, Value>>>, // Not Send!
}

// GOOD - Arc is Send + Sync
struct GoodDatabase {
    cache: Arc<RwLock<HashMap<String, Value>>>, // Send + Sync!
}

// BAD - Raw pointers are not Send
struct UnsafeDatabase {
    ptr: *mut SomeStruct, // Not Send!
}

// GOOD - Use safe abstractions
struct SafeDatabase {
    data: Arc<Mutex<SomeStruct>>, // Send + Sync!
}
```

## Stream Patterns

### Processing Large Result Sets

```rust
use futures::stream::{self, StreamExt};

// Instead of loading everything into memory
async fn get_all_notes_bad(db: &dyn DatabaseService) -> Vec<Note> {
    db.query("SELECT * FROM notes").await.unwrap() // Could be huge!
}

// Stream results as they arrive
async fn get_all_notes_stream(
    db: &dyn DatabaseService,
) -> impl Stream<Item = Result<Note, Error>> {
    let page_size = 100;
    
    stream::unfold(0, move |offset| async move {
        let query = format!(
            "SELECT * FROM notes LIMIT {} OFFSET {}",
            page_size, offset
        );
        
        match db.query(&query).await {
            Ok(notes) if notes.is_empty() => None,
            Ok(notes) => {
                let next_offset = offset + notes.len();
                Some((stream::iter(notes.into_iter().map(Ok)), next_offset))
            }
            Err(e) => Some((stream::once(async { Err(e) }), offset)),
        }
    })
    .flatten()
}

// Process stream
async fn process_notes_stream(db: &dyn DatabaseService) {
    let mut stream = Box::pin(get_all_notes_stream(db));
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(note) => process_note(note).await,
            Err(e) => tracing::error!("Stream error: {}", e),
        }
    }
}
```

## Error Handling in Async

### Propagating Errors

```rust
// Simple error propagation
async fn get_user_notes(
    db: &dyn DatabaseService,
    user_id: &str,
) -> Result<Vec<Note>, Error> {
    let query = format!("SELECT * FROM notes WHERE author = '{}'", user_id);
    db.query(&query).await // ? operator works in async
}

// Multiple error types
async fn complex_operation(
    db: &dyn DatabaseService,
) -> Result<String, Box<dyn std::error::Error>> {
    // Parse config
    let config = std::fs::read_to_string("config.json")?;
    
    // Connect to database
    db.connect().await?;
    
    // Query data
    let data = db.query("SELECT * FROM config").await?;
    
    Ok(format!("Success: {:?}", data))
}
```

### Error Context

```rust
use anyhow::{Context, Result};

async fn get_note_with_context(
    db: &dyn DatabaseService,
    id: &str,
) -> Result<Note> {
    db.read("note", id)
        .await
        .context("Failed to read from database")?
        .ok_or_else(|| anyhow::anyhow!("Note {} not found", id))
}
```

## Testing Async Code

### Basic Async Tests

```rust
#[tokio::test]
async fn test_database_connect() {
    let db = MockDatabase::new();
    assert!(db.connect().await.is_ok());
}

// With timeout
#[tokio::test(flavor = "multi_thread")]
async fn test_with_timeout() {
    let db = SlowDatabase::new();
    
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        db.connect()
    ).await;
    
    assert!(result.is_err()); // Timeout
}
```

### Testing Concurrent Operations

```rust
#[tokio::test]
async fn test_concurrent_access() {
    let db = Arc::new(MockDatabase::new());
    let mut handles = vec![];
    
    // Spawn 10 concurrent tasks
    for i in 0..10 {
        let db = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            db.create("note", json!({
                "title": format!("Note {}", i)
            })).await
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    assert!(results.iter().all(|r| r.is_ok()));
}
```

## Common Async Pitfalls

### 1. Forgetting .await

```rust
// BAD - Returns Future, not result!
fn bad_query(db: &dyn DatabaseService) -> Vec<Note> {
    db.query("SELECT * FROM notes") // Missing .await!
}

// GOOD
async fn good_query(db: &dyn DatabaseService) -> Result<Vec<Note>, Error> {
    db.query("SELECT * FROM notes").await
}
```

### 2. Blocking in Async Context

```rust
// BAD - Blocks the executor
async fn bad_file_read() -> String {
    std::fs::read_to_string("file.txt").unwrap() // Blocking I/O!
}

// GOOD - Use async version
async fn good_file_read() -> Result<String, Error> {
    tokio::fs::read_to_string("file.txt").await
}

// For CPU-intensive work
async fn cpu_intensive_task(data: Vec<u8>) -> Result<String, Error> {
    // Offload to blocking thread pool
    tokio::task::spawn_blocking(move || {
        expensive_computation(data)
    }).await?
}
```

### 3. Async Recursion

```rust
// BAD - Infinite type
async fn recurse(n: u32) {
    if n > 0 {
        recurse(n - 1).await; // Error: recursive type
    }
}

// GOOD - Use Box::pin
fn recurse_good(n: u32) -> Pin<Box<dyn Future<Output = ()>>> {
    Box::pin(async move {
        if n > 0 {
            recurse_good(n - 1).await;
        }
    })
}
```

## Performance Tips

### 1. Buffer Concurrent Operations

```rust
use futures::stream::StreamExt;

async fn process_many_items(
    db: &dyn DatabaseService,
    items: Vec<String>,
) {
    // Process up to 10 concurrently
    stream::iter(items)
        .map(|item| async move {
            db.create("item", json!({ "name": item })).await
        })
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await;
}
```

### 2. Use Select for Timeouts

```rust
use tokio::select;

async fn query_with_cancel(
    db: &dyn DatabaseService,
    cancel: CancellationToken,
) -> Result<Vec<Note>, Error> {
    select! {
        result = db.query("SELECT * FROM notes") => result,
        _ = cancel.cancelled() => Err(Error::Cancelled),
    }
}
```

### 3. Avoid Unnecessary Cloning

```rust
// BAD - Clones data unnecessarily
async fn bad_pattern(db: Arc<dyn DatabaseService>) {
    for i in 0..100 {
        let db_clone = Arc::clone(&db); // Unnecessary!
        process(db_clone).await;
    }
}

// GOOD - Reuse Arc
async fn good_pattern(db: Arc<dyn DatabaseService>) {
    for i in 0..100 {
        process(Arc::clone(&db)).await; // Clone only when needed
    }
}
```

## Best Practices Summary

1. **Always use timeouts** - Database operations can hang
2. **Release locks quickly** - Don't hold across await points
3. **Use Arc for sharing** - Between async tasks
4. **Handle errors properly** - Use ? and Result
5. **Test concurrent access** - Race conditions are real
6. **Batch operations** - When possible
7. **Stream large results** - Don't load everything into memory

Remember: Async is about concurrency, not parallelism. It lets you do many things at once, but not necessarily faster!
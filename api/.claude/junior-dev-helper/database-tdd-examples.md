# Database TDD Examples for Phase 2

## Test-Driven Development Review

Remember the TDD cycle:
1. **RED** - Write a failing test
2. **GREEN** - Write minimal code to pass
3. **REFACTOR** - Clean up while keeping tests green

For databases, this means testing behavior, not implementation!

## Starting with the Database Trait

### 1. Test the Interface First

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    
    // Start with what we want the API to look like
    #[tokio::test]
    async fn test_database_connect() {
        // Arrange
        let db = create_test_database();
        
        // Act
        let result = db.connect().await;
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(db.health_check().await, DatabaseHealth::Healthy);
    }
    
    #[tokio::test]
    async fn test_database_crud_operations() {
        // Arrange
        let db = create_test_database();
        db.connect().await.unwrap();
        
        let data = json!({
            "title": "Test Note",
            "content": "Test Content"
        });
        
        // Act - Create
        let id = db.create("note", data.clone()).await.unwrap();
        
        // Assert - Created with valid ID
        assert!(id.starts_with("note:"));
        
        // Act - Read
        let retrieved = db.read("note", &id).await.unwrap();
        
        // Assert - Data matches
        assert_eq!(retrieved, Some(data));
    }
}
```

### 2. Implement Minimal Interface

```rust
#[async_trait]
pub trait DatabaseService: Send + Sync {
    async fn connect(&self) -> Result<(), DatabaseError>;
    async fn health_check(&self) -> DatabaseHealth;
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError>;
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError>;
}

// Minimal implementation to make tests compile
pub struct MockDatabase;

#[async_trait]
impl DatabaseService for MockDatabase {
    async fn connect(&self) -> Result<(), DatabaseError> {
        todo!("Make test fail first")
    }
    
    // ... other methods
}
```

### 3. Make Tests Pass

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
    
    async fn health_check(&self) -> DatabaseHealth {
        if *self.connected.read().await {
            DatabaseHealth::Healthy
        } else {
            DatabaseHealth::Unhealthy("Not connected".into())
        }
    }
    
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError> {
        if !*self.connected.read().await {
            return Err(DatabaseError::NotConnected);
        }
        
        let id = Uuid::new_v4().to_string();
        let thing_id = format!("{}:{}", collection, id);
        
        self.data.write().await
            .entry(collection.to_string())
            .or_insert_with(HashMap::new)
            .insert(id, data);
            
        Ok(thing_id)
    }
    
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError> {
        if !*self.connected.read().await {
            return Err(DatabaseError::NotConnected);
        }
        
        Ok(self.data.read().await
            .get(collection)
            .and_then(|coll| coll.get(id))
            .cloned())
    }
}
```

## Testing Retry Logic

### 1. Test Exponential Backoff Behavior

```rust
#[tokio::test]
async fn test_exponential_backoff_sequence() {
    // Arrange
    let mut backoff = ExponentialBackoff::new();
    
    // Act - Collect delays
    let delays: Vec<Duration> = (0..5)
        .map(|_| backoff.next_delay())
        .collect();
    
    // Assert - Each delay roughly doubles
    assert!(delays[0] >= Duration::from_millis(900));  // ~1s with jitter
    assert!(delays[0] <= Duration::from_millis(1100));
    
    assert!(delays[1] >= Duration::from_millis(1800)); // ~2s with jitter
    assert!(delays[1] <= Duration::from_millis(2200));
    
    assert!(delays[2] >= Duration::from_millis(3600)); // ~4s with jitter
    assert!(delays[2] <= Duration::from_millis(4400));
}

#[tokio::test]
async fn test_backoff_reset() {
    // Arrange
    let mut backoff = ExponentialBackoff::new();
    
    // Act - Use backoff
    backoff.next_delay();
    backoff.next_delay();
    backoff.next_delay();
    
    // Reset
    backoff.reset();
    
    // Assert - Back to initial delay
    let delay = backoff.next_delay();
    assert!(delay >= Duration::from_millis(900));
    assert!(delay <= Duration::from_millis(1100));
}
```

### 2. Test Retry with Failing Database

```rust
#[tokio::test]
async fn test_retry_eventually_succeeds() {
    // Arrange - Database that fails 3 times then succeeds
    let attempt_count = Arc::new(AtomicU32::new(0));
    let mock_db = FailingDatabase::new(3, attempt_count.clone());
    
    // Act
    let start = Instant::now();
    let result = connect_with_retry(&mock_db, Duration::from_secs(30)).await;
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(attempt_count.load(Ordering::Relaxed), 4); // 3 failures + 1 success
    assert!(start.elapsed() >= Duration::from_secs(6)); // 1 + 2 + 4 seconds
}

#[tokio::test]
async fn test_retry_timeout() {
    // Arrange - Database that always fails
    let mock_db = AlwaysFailingDatabase::new();
    
    // Act
    let start = Instant::now();
    let result = connect_with_retry(&mock_db, Duration::from_secs(5)).await;
    
    // Assert
    assert!(result.is_err());
    assert!(start.elapsed() >= Duration::from_secs(5));
    assert!(start.elapsed() < Duration::from_secs(6));
}
```

### 3. Mock Implementations for Testing

```rust
struct FailingDatabase {
    failures_before_success: u32,
    attempt_count: Arc<AtomicU32>,
}

#[async_trait]
impl DatabaseService for FailingDatabase {
    async fn connect(&self) -> Result<(), DatabaseError> {
        let attempts = self.attempt_count.fetch_add(1, Ordering::Relaxed) + 1;
        
        if attempts <= self.failures_before_success {
            Err(DatabaseError::ConnectionFailed(
                format!("Attempt {} failed", attempts)
            ))
        } else {
            Ok(())
        }
    }
    
    // ... other trait methods
}
```

## Testing Connection Pools

### 1. Test Pool Initialization

```rust
#[tokio::test]
async fn test_pool_creates_min_connections() {
    // Arrange
    let config = PoolConfig {
        min_connections: 3,
        max_connections: 10,
        ..Default::default()
    };
    
    // Act
    let pool = ConnectionPool::new(config.clone());
    pool.initialize().await.unwrap();
    
    // Assert
    assert_eq!(pool.available_connections().await, 3);
    assert_eq!(pool.total_connections().await, 3);
}

#[tokio::test]
async fn test_pool_respects_max_connections() {
    // Arrange
    let config = PoolConfig {
        min_connections: 2,
        max_connections: 5,
        ..Default::default()
    };
    let pool = ConnectionPool::new(config);
    pool.initialize().await.unwrap();
    
    // Act - Acquire all possible connections
    let mut connections = Vec::new();
    for _ in 0..5 {
        connections.push(pool.acquire().await.unwrap());
    }
    
    // Try to acquire one more (should timeout)
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        pool.acquire()
    ).await;
    
    // Assert
    assert!(result.is_err()); // Timeout
    assert_eq!(pool.total_connections().await, 5);
}
```

### 2. Test Connection Health Monitoring

```rust
#[tokio::test]
async fn test_pool_removes_unhealthy_connections() {
    // Arrange
    let pool = ConnectionPool::new(PoolConfig::default());
    pool.initialize().await.unwrap();
    
    // Make one connection unhealthy
    pool.poison_connection(0).await;
    
    // Act - Run health check
    pool.check_health().await;
    
    // Assert - Unhealthy connection replaced
    let health = pool.health_status().await;
    assert!(health.all_healthy);
    assert_eq!(health.replaced_count, 1);
}

#[tokio::test]
async fn test_pool_maintains_min_connections() {
    // Arrange
    let config = PoolConfig {
        min_connections: 5,
        health_check_interval: Duration::from_millis(100),
        ..Default::default()
    };
    let pool = ConnectionPool::new(config);
    pool.initialize().await.unwrap();
    
    // Act - Kill some connections
    pool.poison_connection(0).await;
    pool.poison_connection(1).await;
    
    // Wait for health check
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Assert - Pool restored to minimum
    assert_eq!(pool.available_connections().await, 5);
}
```

## Testing Database Models

### 1. Test Validation

```rust
#[test]
fn test_note_validation() {
    // Arrange
    let valid_note = Note {
        id: None,
        title: "Valid Title".to_string(),
        content: "Valid content".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // Act & Assert - Valid note passes
    assert!(valid_note.validate().is_ok());
    
    // Test invalid cases
    let empty_title = Note {
        title: "".to_string(),
        ..valid_note.clone()
    };
    assert!(empty_title.validate().is_err());
    
    let too_long_title = Note {
        title: "x".repeat(101),
        ..valid_note.clone()
    };
    assert!(too_long_title.validate().is_err());
}

#[tokio::test]
async fn test_note_crud_with_validation() {
    // Arrange
    let db = create_test_database();
    db.connect().await.unwrap();
    
    let invalid_note = Note {
        id: None,
        title: "".to_string(), // Invalid!
        content: "Content".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // Act - Try to save invalid note
    let result = db.create("note", serde_json::to_value(&invalid_note).unwrap()).await;
    
    // Assert - Should fail validation
    assert!(matches!(result, Err(DatabaseError::ValidationFailed(_))));
}
```

### 2. Test Thing ID Handling

```rust
#[test]
fn test_thing_id_parsing() {
    // Test valid formats
    assert!(parse_thing_id("note:123").is_ok());
    assert!(parse_thing_id("user:alice").is_ok());
    assert!(parse_thing_id("org:my-org").is_ok());
    
    // Test invalid formats
    assert!(parse_thing_id("invalid").is_err());
    assert!(parse_thing_id("note-123").is_err());
    assert!(parse_thing_id("note:123:extra").is_err());
    assert!(parse_thing_id("").is_err());
}

#[tokio::test]
async fn test_create_returns_proper_thing_id() {
    // Arrange
    let db = create_test_database();
    db.connect().await.unwrap();
    
    // Act
    let id = db.create("note", json!({"title": "Test"})).await.unwrap();
    
    // Assert
    let (table, record_id) = parse_thing_id(&id).unwrap();
    assert_eq!(table, "note");
    assert!(!record_id.is_empty());
    
    // Can retrieve with parsed ID
    let retrieved = db.read(&table, &record_id).await.unwrap();
    assert!(retrieved.is_some());
}
```

## Testing Write Queue

### 1. Test Queue Behavior

```rust
#[tokio::test]
async fn test_write_queue_persists_on_db_failure() {
    // Arrange
    let failing_db = AlwaysFailingDatabase::new();
    let queue = WriteQueue::new(failing_db, QueueConfig::default());
    
    // Act - Queue writes
    let write1 = QueuedWrite::Create {
        collection: "note".to_string(),
        data: json!({"title": "Note 1"}),
    };
    queue.enqueue(write1.clone()).await.unwrap();
    
    // Assert - Write persisted to disk
    let persisted = queue.load_from_disk().await.unwrap();
    assert_eq!(persisted.len(), 1);
}

#[tokio::test]
async fn test_write_queue_drains_when_db_recovers() {
    // Arrange
    let recovering_db = RecoveringDatabase::new(3); // Fails 3 times
    let queue = WriteQueue::new(recovering_db, QueueConfig::default());
    
    // Queue writes while DB is down
    for i in 0..5 {
        queue.enqueue(QueuedWrite::Create {
            collection: "note".to_string(),
            data: json!({"title": format!("Note {}", i)}),
        }).await.unwrap();
    }
    
    // Act - Start processing (DB will recover)
    queue.start_processing().await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Assert - Queue drained
    assert_eq!(queue.pending_count().await, 0);
    assert_eq!(queue.successful_count().await, 5);
}
```

### 2. Test Queue Persistence

```rust
#[tokio::test]
async fn test_queue_persistence_format() {
    // Arrange
    let temp_dir = tempdir().unwrap();
    let queue_file = temp_dir.path().join("queue.json");
    
    let config = QueueConfig {
        persist_path: queue_file.clone(),
        ..Default::default()
    };
    
    let queue = WriteQueue::new(MockDatabase::new(), config);
    
    // Act - Queue some writes
    queue.enqueue(QueuedWrite::Create {
        collection: "note".to_string(),
        data: json!({"test": true}),
    }).await.unwrap();
    
    // Force persist
    queue.persist().await.unwrap();
    
    // Assert - File exists and is valid JSON
    assert!(queue_file.exists());
    
    let contents = fs::read_to_string(&queue_file).unwrap();
    let parsed: Value = serde_json::from_str(&contents).unwrap();
    
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}
```

## Testing Health Integration

### 1. Test Health Check Integration

```rust
#[tokio::test]
async fn test_database_health_check() {
    // Arrange
    let db = MockDatabase::new();
    
    // Test when not connected
    assert_eq!(
        db.health_check().await,
        DatabaseHealth::Unhealthy("Not connected".to_string())
    );
    
    // Connect
    db.connect().await.unwrap();
    assert_eq!(db.health_check().await, DatabaseHealth::Healthy);
    
    // Simulate degraded state
    db.set_degraded("High latency").await;
    assert_eq!(
        db.health_check().await,
        DatabaseHealth::Degraded("High latency".to_string())
    );
}

#[tokio::test]
async fn test_health_endpoint_includes_database() {
    // Arrange
    let db = Arc::new(MockDatabase::new());
    let app = create_app(db.clone());
    
    // Act - Check health when DB is down
    let response = app.oneshot()
        .get("/health")
        .await
        .unwrap();
    
    // Assert
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    
    let body: HealthResponse = response.json().await;
    assert!(!body.healthy);
    assert!(body.components.database.contains("Not connected"));
    
    // Connect DB and recheck
    db.connect().await.unwrap();
    
    let response = app.oneshot()
        .get("/health")
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

## Integration Testing

### 1. Test with Real SurrealDB

```rust
#[tokio::test]
async fn test_real_surrealdb_connection() {
    // Skip if not in integration test mode
    if std::env::var("RUN_INTEGRATION_TESTS").is_err() {
        return;
    }
    
    // Arrange
    let container = SurrealDbContainer::new()
        .with_user("root")
        .with_password("root")
        .start()
        .await;
    
    let port = container.get_port().await;
    let config = DatabaseConfig {
        url: format!("ws://localhost:{}", port),
        username: "root".to_string(),
        password: "root".to_string(),
        namespace: "test".to_string(),
        database: "test".to_string(),
    };
    
    // Act
    let db = SurrealDbAdapter::new(config);
    let result = connect_with_retry(&db, Duration::from_secs(30)).await;
    
    // Assert
    assert!(result.is_ok());
    
    // Test CRUD
    let id = db.create("note", json!({
        "title": "Integration Test",
        "content": "Testing with real DB"
    })).await.unwrap();
    
    let retrieved = db.read("note", &id.replace("note:", "")).await.unwrap();
    assert!(retrieved.is_some());
}
```

### 2. Test Error Scenarios

```rust
#[tokio::test]
async fn test_connection_timeout() {
    // Arrange - Unreachable address
    let config = DatabaseConfig {
        url: "ws://192.168.99.99:9999".to_string(), // Non-routable IP
        ..Default::default()
    };
    
    let db = SurrealDbAdapter::new(config);
    
    // Act
    let start = Instant::now();
    let result = connect_with_retry(&db, Duration::from_secs(5)).await;
    
    // Assert
    assert!(result.is_err());
    assert!(start.elapsed() >= Duration::from_secs(5));
    
    match result.unwrap_err() {
        DatabaseError::ConnectionFailed(msg) => {
            assert!(msg.contains("timeout") || msg.contains("unreachable"));
        }
        _ => panic!("Wrong error type"),
    }
}
```

## Performance Testing

### 1. Test Connection Pool Performance

```rust
#[tokio::test]
async fn test_pool_performance_under_load() {
    // Arrange
    let pool = Arc::new(ConnectionPool::new(PoolConfig {
        min_connections: 10,
        max_connections: 50,
        ..Default::default()
    }));
    pool.initialize().await.unwrap();
    
    let start = Instant::now();
    let mut handles = Vec::new();
    
    // Act - Spawn many concurrent operations
    for _ in 0..100 {
        let pool = pool.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                let conn = pool.acquire().await.unwrap();
                // Simulate work
                tokio::time::sleep(Duration::from_millis(10)).await;
                // Connection returned on drop
            }
        });
        handles.push(handle);
    }
    
    // Wait for all
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Assert
    let elapsed = start.elapsed();
    println!("Processed 1000 operations in {:?}", elapsed);
    
    // Should complete in reasonable time
    assert!(elapsed < Duration::from_secs(5));
    
    // Check pool metrics
    let stats = pool.stats();
    assert!(stats.total_acquires == 1000);
    assert!(stats.avg_wait_time < Duration::from_millis(100));
}
```

## Best Practices for Database TDD

### 1. Test Behavior, Not Implementation

```rust
// BAD - Testing implementation details
#[test]
fn test_connection_uses_websocket() {
    let db = SurrealDbAdapter::new(config);
    assert!(db.inner_client.is_websocket()); // Too specific!
}

// GOOD - Testing behavior
#[tokio::test]
async fn test_connection_succeeds() {
    let db = create_database();
    assert!(db.connect().await.is_ok());
}
```

### 2. Use Test Doubles Appropriately

```rust
// For unit tests - use mocks
let mock_db = MockDatabase::new();

// For integration tests - use real database
let real_db = create_test_database_container().await;

// For specific scenarios - use specialized mocks
let failing_db = FailingDatabase::new(3); // Fails 3 times
let slow_db = SlowDatabase::with_latency(Duration::from_secs(2));
```

### 3. Test Edge Cases

```rust
#[tokio::test]
async fn test_empty_collection_name() {
    let db = create_test_database();
    let result = db.create("", json!({})).await;
    assert!(matches!(result, Err(DatabaseError::InvalidInput(_))));
}

#[tokio::test]
async fn test_extremely_large_document() {
    let db = create_test_database();
    let huge_content = "x".repeat(10_000_000); // 10MB
    
    let result = db.create("note", json!({
        "content": huge_content
    })).await;
    
    assert!(matches!(result, Err(DatabaseError::DocumentTooLarge)));
}
```

Remember: Write tests that describe what your system should do, not how it does it. This makes refactoring easier and tests more maintainable!
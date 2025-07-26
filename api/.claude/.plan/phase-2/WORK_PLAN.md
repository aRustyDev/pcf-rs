# Phase 2: Database Layer & Persistence - Work Plan

## Prerequisites

Before starting Phase 2, ensure you have:
- **Completed Phase 1**: All checkpoints passed with server foundation operational
- **Database Knowledge**: Understanding of connection pooling, async operations, and retry patterns
- **SurrealDB Familiarity**: Basic understanding of SurrealDB's Thing IDs and query language
- **Async Rust Proficiency**: Comfortable with Arc<Mutex<T>>, async traits, and tokio runtime
- **Testing Experience**: Familiarity with testcontainers and integration testing

## Quick Reference - Essential Resources

### Example Files
All example files are located in `/api/.claude/.spec/examples/`:
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Comprehensive test examples following TDD
- **[Database Retry Patterns](../../.spec/examples/database-retry-patterns.rs)** - Exponential backoff with jitter implementation
- **[Connection Pool Examples](../../.spec/examples/connection-pool.rs)** - Pool configuration and health monitoring patterns

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[SPEC.md](../../SPEC.md)** - Database connectivity requirements (lines 11-16)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 2 objectives (lines 45-67)
- **[error-handling.md](../../.spec/error-handling.md)** - Error type definitions and guidelines
- **[metrics.md](../../.spec/metrics.md)** - Metrics collection requirements

### Quick Links
- **Verification Script**: `scripts/verify-phase-2.sh`
- **Database Test Suite**: `scripts/test-database.sh`
- **Connection Retry Test**: `scripts/test-connection-retry.sh`

## Overview
This work plan implements the database layer with SurrealDB, focusing on reliability through infinite retry logic, configurable connection pooling, write queue persistence, and comprehensive metrics. The implementation follows TDD practices with clear checkpoint boundaries for review and correction.

## Build and Test Commands

Continue using `just` as the command runner:
- `just test` - Run all tests including database tests
- `just test-db` - Run only database-related tests
- `just bench` - Run performance benchmarks
- `just surrealdb-up` - Start local SurrealDB for testing
- `just surrealdb-down` - Stop SurrealDB container

Always use these commands instead of direct cargo commands to ensure consistency.

## IMPORTANT: Review Process

**This plan includes 5 mandatory review checkpoints where work MUST stop for external review.**

At each checkpoint:
1. **STOP all work** and commit your code
2. **Write any questions** to `api/.claude/.reviews/checkpoint-X-questions.md`
3. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
4. **Wait for feedback** in `api/.claude/.reviews/checkpoint-X-feedback.md`
5. **DO NOT PROCEED** until you receive explicit approval

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Continue following TDD practices from Phase 1:
1. **Write tests FIRST** - Before any implementation
2. **Run tests to see them FAIL** - Confirms test is valid
3. **Write minimal code to make tests PASS** - No more than necessary
4. **REFACTOR** - Clean up while keeping tests green
5. **Document as you go** - Add rustdoc comments and inline explanations

## Done Criteria Checklist
- [ ] SurrealDB connects with retry on failure (configurable max duration via STARTUP_MAX_WAIT, default: 10 minutes)
- [ ] Connection pool healthy with configurable sizing
- [ ] All data models properly validated with Garde
- [ ] CRUD operations tested and working
- [ ] Database health check integrated with Phase 1 system
- [ ] Write queue implemented with configurable persistence format
- [ ] Service returns 503 with Retry-After when database unavailable > configurable timeout (default: 30s, override via DB_UNAVAILABLE_TIMEOUT)
- [ ] All database operations have proper timeouts
- [ ] Metrics collection with feature flags (metrics-basic, metrics-detailed, metrics-all)
- [ ] No `.unwrap()` or `.expect()` in production code paths (test code and compile-time constants excluded)
- [ ] SurrealDB version compatibility checking

## Work Breakdown with Review Checkpoints

### 2.1 Database Architecture & Service Trait (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Defining proper abstractions for database operations
- **Scope**: Target 400-600 lines across 4-5 files (MUST document justification if outside range)
- **Key Components**: 
  - Database service trait with async methods (~150 lines)
  - Error types for database operations (~100 lines)
  - Version compatibility checker (~100 lines)
  - Mock implementation for testing (~150 lines)
- **Patterns**: Repository pattern, dependency injection, version checking

#### Task 2.1.1: Write Database Trait Tests First
Create `src/services/database/mod.rs` with comprehensive test module. MUST write and run tests first to see them fail before implementing:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    
    #[tokio::test]
    async fn test_database_trait_connect() {
        let db = MockDatabase::new();
        let result = db.connect().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_version_compatibility() {
        let checker = VersionChecker::new();
        assert!(checker.is_compatible("1.0.0").is_ok());
        assert!(checker.is_compatible("0.1.0").is_err());
    }
    
    #[tokio::test]
    async fn test_database_health_check() {
        let db = MockDatabase::new();
        let health = db.health_check().await;
        assert_eq!(health, DatabaseHealth::Healthy);
    }
}
```

#### Task 2.1.2: Define Database Service Trait
Create the trait that all database implementations must follow:
```rust
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait DatabaseService: Send + Sync {
    /// Connect to the database with retry logic
    async fn connect(&self) -> Result<(), DatabaseError>;
    
    /// Check database health
    async fn health_check(&self) -> DatabaseHealth;
    
    /// Get database version information
    async fn version(&self) -> Result<DatabaseVersion, DatabaseError>;
    
    /// Create a record
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError>;
    
    /// Read a record by ID
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError>;
    
    /// Update a record
    async fn update(&self, collection: &str, id: &str, data: Value) -> Result<(), DatabaseError>;
    
    /// Delete a record
    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError>;
    
    /// Query records with timeout
    async fn query(&self, collection: &str, query: Query) -> Result<Vec<Value>, DatabaseError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
    Starting,
}
```

#### Task 2.1.3: Implement Version Compatibility
Create version checking with configuration:
```rust
pub struct VersionChecker {
    supported_versions: VersionReq,
    tested_versions: Vec<Version>,
}

impl VersionChecker {
    pub fn new() -> Self {
        Self {
            supported_versions: VersionReq::parse(">=1.0.0, <2.0.0").unwrap(),
            tested_versions: vec![
                Version::parse("1.0.0").unwrap(),
                Version::parse("1.1.0").unwrap(),
                Version::parse("1.2.0").unwrap(),
            ],
        }
    }
    
    pub fn check_version(&self, version: &str) -> VersionCompatibility {
        let ver = match Version::parse(version) {
            Ok(v) => v,
            Err(_) => return VersionCompatibility::Unknown,
        };
        
        if !self.supported_versions.matches(&ver) {
            VersionCompatibility::Unsupported
        } else if self.tested_versions.contains(&ver) {
            VersionCompatibility::Tested
        } else {
            VersionCompatibility::Untested
        }
    }
}

pub enum VersionCompatibility {
    Tested,
    Untested,
    Unsupported,
    Unknown,
}
```

#### Task 2.1.4: Implement Database Error Types
Define comprehensive error types following Phase 1 patterns:
```rust
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Database not connected")]
    NotConnected,
    
    #[error("Query timeout after {0} seconds")]
    QueryTimeout(u64),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Record not found: {collection}/{id}")]
    NotFound { collection: String, id: String },
    
    #[error("Version {0} is not supported (requires {1})")]
    UnsupportedVersion(String, String),
    
    #[error("Database error: {0}")]
    Internal(#[from] anyhow::Error),
}
```

---
## 🛑 CHECKPOINT 1: Database Architecture Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Ensure all tests are written and failing appropriately
2. Verify trait design follows async patterns correctly
3. Document all public APIs with rustdoc
4. Check version compatibility logic
5. Write any questions to `api/.claude/.reviews/checkpoint-1-questions.md`
6. Commit with message: "Checkpoint 1: Database architecture complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 2.2 Connection Management & Retry Logic (3-4 work units)

**Work Unit Context:**
- **Complexity**: High - Implementing retry logic, connection pooling, and state management
- **Scope**: Target 800-1200 lines across 5-6 files (MUST document justification if outside range)
- **Key Components**:
  - Connection pool with configurable sizing (~300 lines)
  - Exponential backoff with jitter (~150 lines)
  - Connection state machine (~200 lines)
  - Health monitoring (~150 lines)
  - Pool metrics collection (~200 lines)
- **Required Algorithms**: MUST implement exponential backoff with jitter, connection pool lifecycle management, periodic health checks

#### Task 2.2.1: Write Connection Tests First
Create comprehensive tests for connection behavior:
```rust
#[cfg(test)]
mod connection_tests {
    use super::*;
    use std::time::{Duration, Instant};
    
    #[tokio::test]
    async fn test_exponential_backoff_with_jitter() {
        let mut backoff = ExponentialBackoff::new();
        
        // Verify sequence approximately follows: 1s, 2s, 4s, 8s, 16s, 32s, 60s (max)
        // MUST allow ±20% tolerance for timing variations
        let delays: Vec<_> = (0..10).map(|_| backoff.next_delay()).collect();
        
        // Check exponential growth with max cap
        assert!(delays[0] >= Duration::from_secs(1));
        assert!(delays[0] < Duration::from_millis(2000)); // With jitter
        assert!(delays[6] <= Duration::from_secs(60));
    }
    
    #[tokio::test]
    async fn test_connection_pool_sizing() {
        let config = PoolConfig {
            min_connections: 2,
            max_connections: 10,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(config);
        pool.initialize().await.unwrap();
        
        let health = pool.health().await;
        assert_eq!(health.total, 2); // Min connections created
    }
}
```

#### Task 2.2.2: Implement Exponential Backoff with Jitter
Use the pattern from examples with enhancements:
```rust
pub struct ExponentialBackoff {
    attempt: u32,
    max_delay: Duration,
    base_delay: Duration,
    jitter: bool,
}

impl ExponentialBackoff {
    pub fn new() -> Self {
        Self {
            attempt: 0,
            max_delay: Duration::from_secs(60),
            base_delay: Duration::from_secs(1),
            jitter: true,
        }
    }
    
    pub fn next_delay(&mut self) -> Duration {
        let exp_delay = self.base_delay * 2u32.pow(self.attempt.min(6));
        let delay = exp_delay.min(self.max_delay);
        
        self.attempt += 1;
        
        if self.jitter {
            let jitter_ms = rand::random::<u64>() % 1000;
            delay + Duration::from_millis(jitter_ms)
        } else {
            delay
        }
    }
    
    pub fn reset(&mut self) {
        // MUST be called after successful connection to reset backoff
        self.attempt = 0;
    }
}
```

#### Task 2.2.3: Implement Connection Pool
Create configurable connection pool with health monitoring:
```rust
pub struct ConnectionPool {
    config: PoolConfig,
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    metrics: Arc<PoolMetrics>,
    health_monitor: Arc<HealthMonitor>,
}

#[derive(Clone)]
pub struct PoolConfig {
    pub min_connections: usize,
    pub max_connections: usize,
    pub idle_timeout: Duration,
    pub acquire_timeout: Duration,
    pub health_check_interval: Duration,
    pub max_lifetime: Duration,
}

impl ConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        
        Self {
            config,
            connections: Arc::new(RwLock::new(Vec::new())),
            semaphore,
            metrics: Arc::new(PoolMetrics::new()),
            health_monitor: Arc::new(HealthMonitor::new()),
        }
    }
    
    pub async fn initialize(&self) -> Result<(), DatabaseError> {
        // Pre-warm pool with minimum connections
        for _ in 0..self.config.min_connections {
            self.create_connection().await?;
        }
        
        // MUST start background health monitoring
        self.start_health_monitor();
        
        Ok(())
    }
}
```

#### Task 2.2.4: Implement Retry Logic
Create retry wrapper with infinite retry for startup:
```rust
pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    operation_name: &str,
    is_startup: bool, // MUST be true only during initial application boot, not reconnections
) -> Result<T, E>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    let mut backoff = ExponentialBackoff::new();
    let start_time = Instant::now();
    
    loop {
        match operation().await {
            Ok(result) => {
                tracing::info!("{} succeeded after {:?}", operation_name, start_time.elapsed());
                return Ok(result);
            }
            Err(err) => {
                let delay = backoff.next_delay();
                
                // Check configurable timeout (default 30s for operations, 10 min for startup)
                let max_duration = if is_startup {
                    Duration::from_secs(
                        env::var("STARTUP_MAX_WAIT")
                            .unwrap_or("600".to_string())
                            .parse()
                            .unwrap_or(600)
                    )
                } else {
                    Duration::from_secs(
                        env::var("DB_OPERATION_TIMEOUT")
                            .unwrap_or("30".to_string())
                            .parse()
                            .unwrap_or(30)
                    )
                };
                
                if start_time.elapsed() > max_duration {
                    tracing::error!("{} failed after 30s: {}", operation_name, err);
                    return Err(err);
                }
                
                tracing::warn!(
                    "{} failed (attempt {}): {}. Retrying in {:?}",
                    operation_name, 
                    backoff.attempt, 
                    err, 
                    delay
                );
                
                tokio::time::sleep(delay).await;
            }
        }
    }
}
```

#### Task 2.2.5: Add Metrics Collection
Implement metrics with feature flags:
```rust
#[cfg(feature = "metrics-basic")]
pub mod metrics {
    use prometheus::{IntGauge, IntGaugeVec, HistogramVec};
    
    lazy_static! {
        pub static ref POOL_SIZE: IntGaugeVec = register_int_gauge_vec!(
            "database_connection_pool_size",
            "Current size of connection pool",
            &["state"] // MUST include all states: active, idle, total
        ).unwrap();
        
        #[cfg(feature = "metrics-detailed")]
        pub static ref QUERY_DURATION: HistogramVec = register_histogram_vec!(
            "database_query_duration_seconds",
            "Database query duration",
            &["operation", "collection"]
        ).unwrap();
        
        #[cfg(feature = "metrics-all")]
        pub static ref CONNECTION_LIFETIME: HistogramVec = register_histogram_vec!(
            "database_connection_lifetime_seconds",
            "How long connections live",
            &["reason"] // expired, idle, error
        ).unwrap();
    }
}
```

---
## 🛑 CHECKPOINT 2: Connection Management Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Test retry behavior with simulated failures
2. Verify pool sizing works correctly
3. Check metrics are properly gated by features
4. Document all configuration options
5. Write any questions to `api/.claude/.reviews/checkpoint-2-questions.md`
6. Commit with message: "Checkpoint 2: Connection management complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 2.3 Data Models & Validation (2 work units)

**Work Unit Context:**
- **Complexity**: Low to Medium - Type definitions and validation
- **Scope**: Target 350-450 lines across 3-4 files (MUST document justification if outside range)
- **Key Components**:
  - Note model with SurrealDB Thing ID (~100 lines)
  - ID conversion utilities (~100 lines)
  - Garde validation rules (~100 lines)
  - Schema conversion utilities (~100 lines)
- **Patterns**: Type conversion, validation, serialization

#### Task 2.3.1: Write Model Tests First
Create comprehensive model tests:
```rust
#[cfg(test)]
mod model_tests {
    use super::*;
    use surrealdb::sql::Thing;
    use garde::Validate;
    
    #[test]
    fn test_note_validation() {
        let valid_note = Note {
            id: None,
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            author: "user123".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec![],
        };
        
        assert!(valid_note.validate().is_ok());
        
        // Test validation failures
        let invalid_note = Note {
            title: "".to_string(), // Empty title
            ..valid_note.clone()
        };
        
        let validation_result = invalid_note.validate();
        assert!(validation_result.is_err());
    }
    
    #[test]
    fn test_thing_id_conversion() {
        let thing = Thing::from(("notes", "abc123"));
        let id = NoteId::from(thing);
        assert_eq!(id.to_string(), "notes:abc123");
        
        // Test round trip
        let parsed = NoteId::from_string("notes:abc123").unwrap();
        assert_eq!(parsed.to_string(), "notes:abc123");
    }
}
```

#### Task 2.3.2: Define Note Model with Validation
Create the Note type with comprehensive validation:
```rust
use serde::{Deserialize, Serialize};
use garde::Validate;
use chrono::{DateTime, Utc};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::SimpleObject))]
pub struct Note {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<NoteId>,
    
    #[garde(length(min = 1, max = 200))]
    #[garde(custom(no_script_tags))]
    pub title: String,
    
    #[garde(length(min = 1, max = 10000))]
    pub content: String,
    
    #[garde(length(min = 1, max = 100))]
    #[garde(pattern("[a-zA-Z0-9_-]+"))]
    pub author: String,
    
    #[garde(skip)]
    pub created_at: DateTime<Utc>,
    
    #[garde(skip)]
    pub updated_at: DateTime<Utc>,
    
    #[garde(length(max = 10))]
    #[garde(inner(length(min = 1, max = 50)))]
    pub tags: Vec<String>,
}

fn no_script_tags(value: &str, _: &()) -> garde::Result {
    if value.contains("<script") || value.contains("</script>") {
        return Err(garde::Error::new("Script tags not allowed"));
    }
    Ok(())
}
```

#### Task 2.3.3: Implement SurrealDB ID Handling
Create ID type with proper conversions:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct NoteId(Thing);

impl NoteId {
    pub fn new() -> Self {
        Self(Thing::from(("notes", Id::ulid())))
    }
    
    pub fn from_string(s: &str) -> Result<Self, ValidationError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(ValidationError::new("Invalid ID format"));
        }
        
        if parts[0] != "notes" {
            return Err(ValidationError::new("Invalid collection name"));
        }
        
        Ok(Self(Thing::from((parts[0], parts[1]))))
    }
    
    pub fn collection(&self) -> &str {
        &self.0.tb
    }
    
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }
}

impl fmt::Display for NoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.0.tb, self.0.id)
    }
}

// Serde implementations for API compatibility
impl Serialize for NoteId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for NoteId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_string(&s).map_err(serde::de::Error::custom)
    }
}
```

---
## 🛑 CHECKPOINT 3: Data Models Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Ensure all validation rules are tested
2. Verify ID conversions work both directions
3. Check GraphQL compatibility (if feature enabled)
4. Document validation rules clearly
5. Write any questions to `api/.claude/.reviews/checkpoint-3-questions.md`
6. Commit with message: "Checkpoint 3: Data models complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 2.4 Write Queue & Health Integration (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Queue management, persistence, and health integration
- **Scope**: ~600 lines across 4-5 files
- **Key Components**:
  - Write queue implementation (~200 lines)
  - Configurable persistence (JSON/Bincode) (~150 lines)
  - Queue processing logic (~100 lines)
  - Health check integration (~100 lines)
  - Service unavailable handling (~50 lines)
- **Patterns**: Producer-consumer, persistent queue, health monitoring

#### Task 2.4.1: Write Queue Tests First
Test queue behavior and persistence:
```rust
#[tokio::test]
async fn test_write_queue_persistence() {
    let config = QueueConfig {
        max_size: 1000,
        persistence_format: PersistenceFormat::Json,
        persistence_file: Some("test_queue.json".into()), // MAY be None for in-memory only operation
    };
    
    let queue = WriteQueue::new(config);
    
    // Queue some writes
    for i in 0..5 {
        queue.enqueue(WriteOperation::Create {
            collection: "notes".to_string(),
            data: json!({"title": format!("Note {}", i)}),
        }).await.unwrap();
    }
    
    // Persist
    queue.persist().await.unwrap();
    
    // Create new queue and restore
    let queue2 = WriteQueue::new(config);
    queue2.restore().await.unwrap();
    
    assert_eq!(queue2.len().await, 5);
}

#[tokio::test]
async fn test_service_unavailable_after_timeout() {
    // Test with configurable timeout (default 30s)
    let db = SurrealDatabase::new(config);
    
    // Simulate connection failure beyond configured timeout
    let timeout = env::var("DB_UNAVAILABLE_TIMEOUT")
        .unwrap_or("30".to_string())
        .parse()
        .unwrap_or(30);
    db.set_connection_failed(Instant::now() - Duration::from_secs(timeout + 1));
    
    let result = db.create("notes", json!({})).await;
    
    match result {
        Err(DatabaseError::ServiceUnavailable { retry_after }) => {
            assert_eq!(retry_after, 60); // Next retry in 60s
        }
        _ => panic!("Expected ServiceUnavailable error"),
    }
}
```

#### Task 2.4.2: Implement Write Queue
Create queue with configurable persistence:
```rust
#[derive(Debug, Clone)]
pub enum PersistenceFormat {
    Json,
    Bincode,
    MessagePack,
    Cbor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedWrite {
    pub id: Uuid,
    pub operation: WriteOperation,
    pub queued_at: DateTime<Utc>,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteOperation {
    Create { collection: String, data: Value },
    Update { collection: String, id: String, data: Value },
    Delete { collection: String, id: String },
}

pub struct WriteQueue {
    config: QueueConfig,
    queue: Arc<RwLock<VecDeque<QueuedWrite>>>,
    metrics: Arc<QueueMetrics>,
}

impl WriteQueue {
    pub async fn enqueue(&self, operation: WriteOperation) -> Result<Uuid, DatabaseError> {
        let mut queue = self.queue.write().await;
        
        if queue.len() >= self.config.max_size {
            // MUST return error when queue full, MAY implement backpressure in future
            #[cfg(feature = "metrics-detailed")]
            self.metrics.queue_full.inc();
            
            return Err(DatabaseError::QueueFull);
        }
        
        let id = Uuid::new_v4();
        let write = QueuedWrite {
            id,
            operation,
            queued_at: Utc::now(),
            retry_count: 0,
            last_error: None,
        };
        
        queue.push_back(write);
        
        #[cfg(feature = "metrics-basic")]
        self.metrics.queue_size.set(queue.len() as i64);
        
        Ok(id)
    }
    
    pub async fn persist(&self) -> Result<(), DatabaseError> {
        let queue = self.queue.read().await;
        let path = self.config.persistence_file.as_ref()
            .ok_or_else(|| DatabaseError::Internal(anyhow!("No persistence file configured")))?;
        
        let data = match self.config.persistence_format {
            PersistenceFormat::Json => {
                serde_json::to_vec_pretty(&*queue)?
            }
            PersistenceFormat::Bincode => {
                bincode::serialize(&*queue)?
            }
            PersistenceFormat::MessagePack => {
                rmp_serde::to_vec(&*queue)?
            }
            PersistenceFormat::Cbor => {
                serde_cbor::to_vec(&*queue)?
            }
        };
        
        tokio::fs::write(path, data).await?;
        
        tracing::debug!("Persisted {} queued writes", queue.len());
        Ok(())
    }
}
```

#### Task 2.4.3: Integrate with Health System
Update health checks from Phase 1:
```rust
impl SurrealDatabase {
    pub async fn health_check(&self) -> DatabaseHealth {
        let state = self.connection_state.lock().await;
        
        match &*state {
            ConnectionState::Connected(since) => {
                // Perform ping to verify connection
                match self.ping_internal().await {
                    Ok(latency) => {
                        #[cfg(feature = "metrics-detailed")]
                        self.metrics.ping_latency.observe(latency.as_secs_f64());
                        
                        DatabaseHealth::Healthy
                    }
                    Err(_) => DatabaseHealth::Degraded("Ping failed".to_string())
                }
            }
            ConnectionState::Connecting => DatabaseHealth::Starting,
            ConnectionState::Failed(since) => {
                let duration = since.elapsed();
                if duration > Duration::from_secs(30) {
                    DatabaseHealth::Unhealthy(format!(
                        "Connection failed {}s ago",
                        duration.as_secs()
                    ))
                } else {
                    DatabaseHealth::Degraded("Reconnecting...".to_string())
                }
            }
            ConnectionState::Disconnected => {
                DatabaseHealth::Unhealthy("Not connected".to_string())
            }
        }
    }
    
    fn should_return_unavailable(&self) -> Option<u64> {
        if let ConnectionState::Failed(since) = &*self.connection_state.lock().await {
            if since.elapsed() > Duration::from_secs(30) {
                // Calculate next retry time
                let next_retry = 60; // seconds
                return Some(next_retry);
            }
        }
        None
    }
}

// In API handlers
async fn handle_database_operation<T>(
    db: &SurrealDatabase,
    operation: impl Future<Output = Result<T, DatabaseError>>,
) -> Result<T, AppError> {
    if let Some(retry_after) = db.should_return_unavailable() {
        return Err(AppError::ServiceUnavailable {
            message: "Database unavailable".to_string(),
            retry_after: Some(retry_after),
        });
    }
    
    operation.await.map_err(Into::into)
}
```

---
## 🛑 CHECKPOINT 4: Write Queue & Health Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Test queue persistence in all formats
2. Verify 503 response after 30s disconnection
3. Check health integration with Phase 1
4. Test queue processing on reconnection
5. Write any questions to `api/.claude/.reviews/checkpoint-4-questions.md`
6. Commit with message: "Checkpoint 4: Write queue and health complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 2.5 Complete Integration & Metrics (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Full system integration and testing
- **Scope**: ~800 lines of tests and integration
- **Key Components**:
  - SurrealDB adapter implementation (~300 lines)
  - Integration test suite (~300 lines)
  - Metrics finalization (~100 lines)
  - Verification scripts (~100 lines)
- **Patterns**: Integration testing, metrics collection, system verification

#### Task 2.5.1: Implement SurrealDB Adapter
Complete the database service implementation:
```rust
pub struct SurrealDatabase {
    client: Arc<RwLock<Option<Surreal<Client>>>>,
    config: DatabaseConfig,
    connection_state: Arc<RwLock<ConnectionState>>,
    write_queue: Arc<WriteQueue>,
    pool: Arc<ConnectionPool>,
    version_checker: Arc<VersionChecker>,
    metrics: Arc<DatabaseMetrics>,
}

#[async_trait]
impl DatabaseService for SurrealDatabase {
    async fn connect(&self) -> Result<(), DatabaseError> {
        let mut state = self.connection_state.write().await;
        *state = ConnectionState::Connecting;
        
        let connect_fn = || {
            let config = self.config.clone();
            Box::pin(async move {
                let client = Surreal::new::<Ws>(&config.endpoint).await?;
                
                // Check version
                let version_info = client.version().await?;
                Ok((client, version_info))
            })
        };
        
        match retry_with_backoff(connect_fn, "SurrealDB connection", true).await {
            Ok((client, version)) => {
                // Check version compatibility
                match self.version_checker.check_version(&version.to_string()) {
                    VersionCompatibility::Unsupported => {
                        return Err(DatabaseError::UnsupportedVersion(
                            version.to_string(),
                            self.version_checker.supported_versions.to_string(),
                        ));
                    }
                    VersionCompatibility::Untested => {
                        tracing::warn!(
                            "SurrealDB version {} is untested. Supported versions: {:?}",
                            version,
                            self.version_checker.tested_versions
                        );
                    }
                    _ => {}
                }
                
                // Configure namespace and database
                client.use_ns(&self.config.namespace)
                    .use_db(&self.config.database)
                    .await?;
                
                *self.client.write().await = Some(client);
                *state = ConnectionState::Connected(Instant::now());
                
                // Process queued writes
                self.process_write_queue().await;
                
                tracing::info!("Connected to SurrealDB version {}", version);
                Ok(())
            }
            Err(err) => {
                *state = ConnectionState::Failed(Instant::now());
                Err(err)
            }
        }
    }
    
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError> {
        #[cfg(feature = "metrics-detailed")]
        let _timer = self.metrics.query_duration
            .with_label_values(&["create", collection])
            .start_timer();
        
        if let Some(client) = self.get_client().await? {
            let result: Option<Record> = client
                .create(collection)
                .content(data)
                .await
                .map_err(|e| DatabaseError::Internal(e.into()))?;
            
            Ok(result.map(|r| r.id.to_string()).unwrap_or_default())
        } else {
            // Queue for later
            self.write_queue.enqueue(WriteOperation::Create {
                collection: collection.to_string(),
                data,
            }).await?;
            
            Ok(Uuid::new_v4().to_string())
        }
    }
}
```

#### Task 2.5.2: Create Integration Tests
Comprehensive integration test suite:
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use testcontainers::{clients, images::surrealdb::SurrealDb};
    
    #[tokio::test]
    async fn test_full_lifecycle() {
        let docker = clients::Cli::default();
        let container = docker.run(SurrealDb::default());
        let port = container.get_host_port_ipv4(8000);
        
        let config = DatabaseConfig {
            endpoint: format!("ws://localhost:{}", port),
            namespace: "test".to_string(),
            database: "test".to_string(),
            pool_config: PoolConfig {
                min_connections: 2,
                max_connections: 5,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let db = SurrealDatabase::new(config);
        
        // Test connection
        db.connect().await.unwrap();
        
        // Test CRUD
        let note = Note {
            id: None,
            title: "Integration Test".to_string(),
            content: "Testing full lifecycle".to_string(),
            author: "test_user".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["test".to_string()],
        };
        
        let id = db.create("notes", serde_json::to_value(&note).unwrap())
            .await
            .unwrap();
        
        let retrieved = db.read("notes", &id).await.unwrap();
        assert!(retrieved.is_some());
        
        // Test query
        let results = db.query("notes", Query::filter("author", "test_user"))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }
    
    #[tokio::test]
    async fn test_connection_recovery() {
        // Test system recovers when database becomes available
        let db = SurrealDatabase::new(test_config());
        
        // Start with no database
        let connect_result = db.connect().await;
        // MUST retry with configurable max duration during startup (default: 10 minutes)
        
        // Start database container
        let docker = clients::Cli::default();
        let _container = docker.run(SurrealDb::default());
        
        // Should eventually connect
        // ... test implementation
    }
}
```

#### Task 2.5.3: Add Verification Scripts
Create comprehensive verification:
```bash
#!/bin/bash
# scripts/verify-phase-2.sh
set -e

echo "=== Phase 2 Database Layer Verification ==="

# Check compilation
echo "✓ Checking compilation..."
just build || { echo "Build failed"; exit 1; }

# Check feature combinations
echo "✓ Testing feature combinations..."
cargo check --no-default-features || { echo "Check failed"; exit 1; }
cargo check --features metrics-basic || { echo "Metrics-basic check failed"; exit 1; }
cargo check --features metrics-detailed || { echo "Metrics-detailed check failed"; exit 1; }
cargo check --features metrics-all || { echo "Metrics-all check failed"; exit 1; }

# Run tests
echo "✓ Running unit tests..."
just test-db

# Start test database
echo "✓ Starting SurrealDB..."
just surrealdb-up

# Run integration tests
echo "✓ Running integration tests..."
cargo test --test '*' -- --test-threads=1

# Test connection retry
echo "✓ Testing connection retry behavior..."
./scripts/test-connection-retry.sh

# Check metrics
echo "✓ Verifying metrics endpoint..."
cargo run --features metrics-all &
SERVER_PID=$!
sleep 5
curl -s http://localhost:8080/metrics | grep -E "database_"
kill $SERVER_PID

# Cleanup
just surrealdb-down

echo "=== All Phase 2 verification passed! ==="
```

#### Task 2.5.4: Finalize Metrics
Complete metrics implementation with all feature levels:
```rust
#[cfg(feature = "metrics-basic")]
pub struct DatabaseMetrics {
    // Basic metrics - always collected
    pub connection_state: IntGaugeVec,
    pub pool_size: IntGaugeVec,
    pub write_queue_size: IntGauge,
}

#[cfg(feature = "metrics-detailed")]
impl DatabaseMetrics {
    // Detailed metrics - query level
    pub query_duration: HistogramVec,
    pub query_count: IntCounterVec,
    pub error_count: IntCounterVec,
}

#[cfg(feature = "metrics-all")]
impl DatabaseMetrics {
    // All metrics - includes debugging
    pub connection_lifetime: HistogramVec,
    pub retry_attempts: IntCounterVec,
    pub cache_hit_rate: GaugeVec,
    pub version_info: IntGaugeVec,
}
```

---
## 🛑 CHECKPOINT 5: Complete Integration Review

**STOP HERE FOR FINAL EXTERNAL REVIEW**

**Before requesting review:**
1. Run full verification script successfully
2. Test all CRUD operations
3. Verify metrics at all feature levels
4. Check memory usage and performance
5. Review all documentation
6. Write any questions to `api/.claude/.reviews/checkpoint-5-questions.md`
7. Commit with message: "Checkpoint 5: Phase 2 complete"

**Final Review Checklist:**
- [ ] All Done Criteria met
- [ ] Integration tests comprehensive
- [ ] Test coverage MUST be ≥80% overall, ≥95% for critical paths (connection retry, write queue, health checks)
  - MAY exclude system-dependent code with documented justification
  - Use `cargo tarpaulin --exclude-files 'tests/*' --out Html` to verify
- [ ] Metrics properly feature-gated
- [ ] Documentation complete
- [ ] No security vulnerabilities
- [ ] Performance acceptable (p99 < 100ms for queries, MAY allow up to 150ms with justification)

**DO NOT PROCEED** to Phase 3 until final approval received.

---

## Test Coverage Requirements

**MUST achieve:**
- ≥80% overall test coverage
- ≥95% coverage on critical paths:
  - Connection retry logic
  - Write queue operations
  - Health check state transitions
  - Error handling paths

**MAY exclude from coverage with documentation:**
- System-dependent code (e.g., OS-specific paths)
- Third-party library wrappers (if thin)
- Compile-time constants and type definitions
- Code that requires specific hardware/network conditions

**Recovery if coverage is below target:**
1. Document why coverage cannot be achieved
2. Add integration tests to compensate
3. Get approval with remediation plan

## Common Issues and Solutions

### Connection Issues
- **"Connection refused"**: Check SurrealDB is running on correct port
- **Version warnings**: Update tested_versions list after testing
- **Pool exhaustion**: Increase max_connections in config

### Write Queue Issues  
- **Queue fills up**: Check why database is unavailable, increase limit
- **Persistence fails**: Check file permissions and disk space
- **Wrong format**: Verify persistence_format matches file content

### Performance Issues
- **Slow queries**: Enable metrics-detailed to identify bottlenecks
- **High memory**: Reduce pool size or queue limit
- **Metric cardinality**: Check label combinations aren't exploding

## Next Phase Preview

Phase 3 will implement the GraphQL layer, building on the database foundation:
- GraphQL schema generation from models
- Query and mutation resolvers
- Subscription support with WebSockets
- Integration with authorization (Phase 4)

---
*This work plan follows the same structure and TDD practices as Phase 1, adapted specifically for database layer requirements.*
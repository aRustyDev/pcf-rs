# Testing Strategy Specification

## Core Requirements

### Coverage Requirements
- MUST achieve minimum 80% code coverage across all modules
- MUST achieve 100% coverage for critical paths:
  - Authorization checks (all permit/deny paths)
  - Database retry logic and circuit breakers
  - Health check state transitions
  - Error handling and propagation
  - Configuration validation
  - Security validations (input sanitization, auth checks)
- SHOULD achieve 90% coverage for GraphQL resolvers
- MUST track coverage trends and fail CI if coverage decreases

### Test Execution Requirements
- Unit tests MUST complete within 10 seconds
- Integration tests MUST complete within 2 minutes
- E2E tests MUST complete within 5 minutes
- All tests MUST be deterministic (no flaky tests)
- Tests MUST be runnable in parallel
- CI MUST run all tests before allowing merge

## Test Organization

### Directory Structure
```
src/tests/
├── unit/                      # Fast, isolated unit tests
│   ├── schema/               # Type conversion tests
│   ├── helpers/              # Helper function tests  
│   ├── auth/                 # Authorization logic tests
│   └── services/             # Service trait implementation tests
│
├── integration/              # Tests with real/mock dependencies
│   ├── database/            # Database operation tests
│   ├── graphql/             # GraphQL resolver tests
│   ├── health/              # Health check behavior tests
│   └── auth/                # SpiceDB integration tests
│
├── e2e/                     # Full end-to-end tests
│   ├── flows/               # Complete user flows
│   ├── subscriptions/       # WebSocket subscription tests
│   └── scenarios/           # Complex multi-operation tests
│
├── fixtures/                # Test data builders
│   ├── builders.rs          # Builder patterns for test data
│   ├── factories.rs         # Factory functions
│   └── generators.rs        # Random data generators
│
├── common/                  # Shared test utilities
│   ├── mod.rs              # Common test setup
│   ├── containers.rs       # Test container management
│   └── mocks.rs            # Mock service implementations
│
└── critical/                # Critical path tests (100% coverage required)
    ├── authorization.rs     # All auth paths must be tested
    ├── retry_logic.rs      # Database retry scenarios
    ├── circuit_breaker.rs  # Circuit breaker state transitions
    └── security.rs         # Input validation and sanitization
```

## Testing Patterns

### Builder Pattern for Test Data
```rust
// tests/fixtures/builders.rs
use chrono::{DateTime, Utc};
use surrealdb::sql::Thing;
use uuid::Uuid;
use faker::Faker;

pub struct NoteBuilder {
    title: String,
    content: String,
    author: String,
    created_at: Option<DateTime<Utc>>,
}

impl NoteBuilder {
    pub fn new() -> Self {
        Self {
            title: "Default Title".to_string(),
            content: "Default Content".to_string(),
            author: "test_user".to_string(),
            created_at: None,
        }
    }
    
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }
    
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }
    
    pub fn with_author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }
    
    pub fn with_random_content(mut self) -> Self {
        use faker::lorem::en::*;
        self.content = Paragraph(3..5).fake();
        self
    }
    
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }
    
    pub fn build(self) -> Note {
        let id = Thing::from(("note", Uuid::new_v4().to_string()));
        Note {
            id: Some(id.clone()),
            id_string: id.to_string(),
            title: self.title,
            content: self.content,
            author: self.author,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            updated_at: Utc::now(),
        }
    }
}

// Usage in tests
#[test]
fn test_note_creation() {
    let note = NoteBuilder::new()
        .with_title("Test Note")
        .with_random_content()
        .build();
        
    assert_eq!(note.title, "Test Note");
    assert!(!note.content.is_empty());
}
```

### Property-Based Testing
```rust
// tests/unit/schema/note_validation_tests.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn note_title_validation(
        title in "[a-zA-Z ]{1,200}"
    ) {
        let note = NoteBuilder::new()
            .with_title(&title)
            .build();
            
        assert!(validate_note_title(&note.title).is_ok());
        assert!(note.title.len() <= 200);
    }
    
    #[test]
    fn note_content_validation(
        content in prop::string::string_regex("[a-zA-Z0-9 \n]{1,10000}").unwrap()
    ) {
        let note = NoteBuilder::new()
            .with_content(&content)
            .build();
            
        assert!(validate_note_content(&note.content).is_ok());
        assert!(note.content.len() <= 10000);
    }
    
    #[test]
    fn note_serialization_roundtrip(
        title in "[a-zA-Z ]{1,200}",
        content in "[a-zA-Z ]{1,10000}",
        author in "[a-zA-Z0-9_]{1,100}"
    ) {
        let note = NoteBuilder::new()
            .with_title(&title)
            .with_content(&content)
            .with_author(&author)
            .build();
            
        // Test JSON serialization
        let serialized = serde_json::to_string(&note).unwrap();
        let deserialized: Note = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(note.title, deserialized.title);
        assert_eq!(note.content, deserialized.content);
        assert_eq!(note.author, deserialized.author);
    }
}
```

### Random Data Generation with Security Testing
```rust
// tests/fixtures/generators.rs
use faker::{Faker, Fake};
use rand::Rng;

pub struct NoteGenerator;

impl NoteGenerator {
    /// Generate a specified number of random notes
    pub fn generate_many(count: usize) -> Vec<Note> {
        (0..count)
            .map(|_| Self::generate_one())
            .collect()
    }
    
    /// Generate a single random note
    pub fn generate_one() -> Note {
        use faker::name::en::*;
        use faker::lorem::en::*;
        
        NoteBuilder::new()
            .with_title(&Sentence(3..10).fake::<String>())
            .with_content(&Paragraph(3..5).fake::<String>())
            .with_author(&format!("user_{}", Username().fake::<String>()))
            .build()
    }
    
    /// Generate malicious inputs for security testing
    pub fn generate_malicious() -> Vec<Note> {
        vec![
            // SQL injection attempts
            NoteBuilder::new()
                .with_title("'; DROP TABLE notes; --")
                .with_content("Normal content")
                .build(),
            // XSS attempts
            NoteBuilder::new()
                .with_title("<script>alert('xss')</script>")
                .with_content("<img src=x onerror=alert('xss')>")
                .build(),
            // Path traversal
            NoteBuilder::new()
                .with_title("../../../etc/passwd")
                .build(),
            // Unicode edge cases
            NoteBuilder::new()
                .with_title("\u202E\u0041\u0042\u0043") // Right-to-left override
                .build(),
            // Oversized inputs
            NoteBuilder::new()
                .with_title(&"A".repeat(1000)) // Exceeds 200 char limit
                .with_content(&"B".repeat(20000)) // Exceeds 10000 char limit
                .build(),
        ]
    }
    
    /// Generate notes with specific patterns for testing
    pub fn generate_with_pattern(pattern: NotePattern) -> Note {
        match pattern {
            NotePattern::LongContent => {
                NoteBuilder::new()
                    .with_content(&"x".repeat(9000))
                    .build()
            }
            NotePattern::SpecialCharacters => {
                NoteBuilder::new()
                    .with_title("Note with émojis 🚀 and spëcial çhars")
                    .build()
            }
            NotePattern::OldNote => {
                NoteBuilder::new()
                    .with_created_at(Utc::now() - Duration::days(365))
                    .build()
            }
        }
    }
}

pub enum NotePattern {
    LongContent,
    SpecialCharacters,
    OldNote,
}
```

## Mock Services

### Mock Database Service
```rust
// tests/common/mocks.rs
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Clone)]
pub struct MockDatabaseService {
    notes: Arc<RwLock<HashMap<String, Note>>>,
    fail_next_operation: Arc<RwLock<bool>>,
    delay_ms: Arc<RwLock<Option<u64>>>,
}

impl MockDatabaseService {
    pub fn new() -> Self {
        Self {
            notes: Arc::new(RwLock::new(HashMap::new())),
            fail_next_operation: Arc::new(RwLock::new(false)),
            delay_ms: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Configure the mock to fail the next operation
    pub fn fail_next(self) -> Self {
        *self.fail_next_operation.write().await = true;
        self
    }
    
    /// Configure the mock to delay operations
    pub fn with_delay(self, delay_ms: u64) -> Self {
        *self.delay_ms.write().await = Some(delay_ms);
        self
    }
}

#[async_trait]
impl DatabaseService for MockDatabaseService {
    async fn create_note(&self, input: CreateNoteInput) -> Result<Note> {
        // Check if we should fail
        if *self.fail_next_operation.read().await {
            *self.fail_next_operation.write().await = false;
            return Err(anyhow!("Database operation failed"));
        }
        
        // Apply delay if configured
        if let Some(delay) = *self.delay_ms.read().await {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        
        let note = NoteBuilder::new()
            .with_title(&input.title)
            .with_content(&input.content)
            .with_author(&input.author)
            .build();
            
        self.notes.write().await.insert(note.id_string.clone(), note.clone());
        Ok(note)
    }
    
    async fn get_note(&self, id: &str) -> Result<Option<Note>> {
        Ok(self.notes.read().await.get(id).cloned())
    }
}
```

### Mock SpiceDB Client
```rust
#[derive(Clone)]
pub struct MockSpiceDBClient {
    responses: Arc<RwLock<HashMap<String, bool>>>,
    default_response: Arc<RwLock<bool>>,
}

impl MockSpiceDBClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            default_response: Arc::new(RwLock::new(true)),
        }
    }
    
    /// Set a specific response for a permission check
    pub async fn set_response(&self, user: &str, resource: &str, action: &str, allowed: bool) {
        let key = format!("{}:{}:{}", user, resource, action);
        self.responses.write().await.insert(key, allowed);
    }
    
    /// Set the default response for unspecified checks
    pub async fn set_default_response(&self, allowed: bool) {
        *self.default_response.write().await = allowed;
    }
}

#[async_trait]
impl AuthorizationService for MockSpiceDBClient {
    async fn check_permission(&self, req: CheckPermissionRequest) -> Result<bool> {
        let key = format!("{}:{}:{}", req.subject, req.resource, req.permission);
        
        if let Some(&allowed) = self.responses.read().await.get(&key) {
            Ok(allowed)
        } else {
            Ok(*self.default_response.read().await)
        }
    }
}
```

## Container-Based Integration Testing

### Test Container Setup
```rust
// tests/common/containers.rs
use testcontainers::{clients::Cli, core::WaitFor, Image, Container};

pub struct TestEnvironment {
    docker: Cli,
    surrealdb: Container<'static, SurrealDBImage>,
    spicedb: Option<Container<'static, SpiceDBImage>>,
}

impl TestEnvironment {
    pub async fn new() -> Self {
        let docker = Cli::default();
        
        // Start SurrealDB
        let surrealdb = docker.run(SurrealDBImage::default());
        wait_for_surrealdb(&surrealdb).await;
        
        Self {
            docker,
            surrealdb,
            spicedb: None,
        }
    }
    
    pub async fn with_spicedb(mut self) -> Self {
        let spicedb = self.docker.run(SpiceDBImage::default());
        wait_for_spicedb(&spicedb).await;
        self.spicedb = Some(spicedb);
        self
    }
    
    pub fn surrealdb_url(&self) -> String {
        format!("ws://localhost:{}", self.surrealdb.get_host_port_ipv4(8000))
    }
    
    pub fn spicedb_url(&self) -> Option<String> {
        self.spicedb.as_ref().map(|s| {
            format!("localhost:{}", s.get_host_port_ipv4(50051))
        })
    }
}

// Custom container images
#[derive(Default)]
pub struct SurrealDBImage;

impl Image for SurrealDBImage {
    type Args = ();
    
    fn name(&self) -> String {
        "surrealdb/surrealdb".to_string()
    }
    
    fn tag(&self) -> String {
        "latest".to_string()
    }
    
    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Started web server")]
    }
    
    fn cmd(&self) -> Vec<String> {
        vec![
            "start".to_string(),
            "--user".to_string(),
            "root".to_string(),
            "--pass".to_string(),
            "root".to_string(),
            "memory".to_string(),
        ]
    }
}
```

## Critical Path Tests

### Authorization Flow Tests
```rust
// tests/integration/auth/authorization_tests.rs
#[tokio::test]
async fn test_authorization_cache_hit() {
    let ctx = TestContext::new()
        .with_user("user_123")
        .with_mock_cache()
        .with_mock_spicedb();
        
    let cache = ctx.auth_cache();
    let spicedb = ctx.spicedb_client();
    
    // Pre-populate cache
    cache.set("user_123:note:456:write", true, Duration::from_secs(300)).await;
    
    // This should hit cache and not call SpiceDB
    let result = is_authorized(&ctx, "note:456", "write").await;
    
    assert!(result.is_ok());
    assert_eq!(spicedb.call_count().await, 0); // No SpiceDB calls
}

#[tokio::test]
async fn test_authorization_cache_miss() {
    let ctx = TestContext::new()
        .with_user("user_123")
        .with_mock_cache()
        .with_mock_spicedb();
        
    let spicedb = ctx.spicedb_client();
    spicedb.set_response("user_123", "note:456", "write", true).await;
    
    let result = is_authorized(&ctx, "note:456", "write").await;
    
    assert!(result.is_ok());
    assert_eq!(spicedb.call_count().await, 1); // Called SpiceDB
}

#[tokio::test]
async fn test_authorization_denied() {
    let ctx = TestContext::new()
        .with_user("user_123")
        .with_mock_spicedb();
        
    let spicedb = ctx.spicedb_client();
    spicedb.set_response("user_123", "note:456", "write", false).await;
    
    let result = is_authorized(&ctx, "note:456", "write").await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.extensions.get("code"), Some(&"FORBIDDEN".into()));
}

#[tokio::test]
async fn test_authorization_no_user() {
    let ctx = TestContext::new(); // No user set
    
    let result = is_authorized(&ctx, "note:456", "write").await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.extensions.get("code"), Some(&"UNAUTHORIZED".into()));
}
```

### Database Retry Logic Tests
```rust
// tests/unit/services/retry_tests.rs
#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    
    let operation = || {
        let attempts = attempts_clone.clone();
        async move {
            let count = attempts.fetch_add(1, Ordering::SeqCst);
            if count < 3 {
                Err(anyhow!("Connection refused"))
            } else {
                Ok("Success")
            }
        }
    };
    
    let start = Instant::now();
    let result = retry_with_backoff("test_operation", operation, 5).await;
    let duration = start.elapsed();
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(attempts.load(Ordering::SeqCst), 4); // 3 failures + 1 success
    assert!(duration >= Duration::from_secs(7)); // 1 + 2 + 4 seconds
}

#[tokio::test]
async fn test_retry_max_attempts_exceeded() {
    let operation = || async {
        Err::<String, _>(anyhow!("Always fails"))
    };
    
    let result = retry_with_backoff("failing_operation", operation, 3).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("after 3 attempts"));
}
```

### Health Check Tests
```rust
// tests/integration/health/health_check_tests.rs
#[tokio::test]
async fn test_health_check_all_healthy() {
    let env = TestEnvironment::new().await;
    let app = create_test_app(env).await;
    
    let response = app.oneshot(
        Request::builder()
            .uri("/health/ready")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: HealthStatus = parse_response_body(response).await;
    assert_eq!(body.status, "healthy");
    assert!(body.services.get("database").unwrap().status == "healthy");
}

#[tokio::test]
async fn test_health_check_degraded() {
    let mut env = TestEnvironment::new().await;
    env.disable_cache(); // Non-critical service
    
    let app = create_test_app(env).await;
    
    let response = app.oneshot(
        Request::builder()
            .uri("/health/ready")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: HealthStatus = parse_response_body(response).await;
    assert_eq!(body.status, "degraded");
}

#[tokio::test]
async fn test_health_check_unhealthy() {
    let mut env = TestEnvironment::new().await;
    env.stop_database(); // Critical service
    
    let app = create_test_app(env).await;
    
    let response = app.oneshot(
        Request::builder()
            .uri("/health/ready")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    
    let body: HealthStatus = parse_response_body(response).await;
    assert_eq!(body.status, "unhealthy");
}
```

### GraphQL Schema Tests
```rust
// tests/integration/graphql/schema_tests.rs
#[tokio::test]
async fn test_graphql_introspection() {
    let app = create_test_app_with_mocks().await;
    
    let query = r#"
        {
            __schema {
                queryType {
                    fields {
                        name
                        type {
                            name
                        }
                    }
                }
            }
        }
    "#;
    
    let response = app.graphql_query(query).await;
    
    assert!(response.errors.is_empty());
    assert!(response.data.is_some());
    
    let fields = response.data
        .get("__schema")
        .get("queryType")
        .get("fields")
        .as_array()
        .unwrap();
        
    let field_names: Vec<&str> = fields.iter()
        .map(|f| f.get("name").as_str().unwrap())
        .collect();
        
    assert!(field_names.contains(&"note"));
    assert!(field_names.contains(&"notes"));
    assert!(field_names.contains(&"notesByAuthor"));
    assert!(field_names.contains(&"health"));
}
```

## Testing Best Practices

### Test Independence
- Each test MUST be independent and not rely on other tests
- Tests MUST clean up their data after completion
- Database state MUST be reset between test runs
- Use transactions for database tests where possible

### Performance Testing
```rust
#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_graphql_query_performance() {
    let env = TestEnvironment::new().await;
    let app = create_app(env).await;
    
    // Warmup
    for _ in 0..10 {
        app.graphql_query("{ notes { id title } }").await;
    }
    
    // Measure
    let start = Instant::now();
    let iterations = 1000;
    
    for _ in 0..iterations {
        let response = app.graphql_query("{ notes { id title } }").await;
        assert!(response.errors.is_empty());
    }
    
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() / iterations;
    
    // Performance requirement: < 50ms average
    assert!(
        avg_ms < 50,
        "Query performance degraded: {}ms average",
        avg_ms
    );
}
```

### Chaos Testing
```rust
#[tokio::test]
#[ignore] // Run separately due to system impact
async fn test_chaos_database_failures() {
    let env = TestEnvironment::new()
        .with_chaos_proxy() // Randomly drops connections
        .await;
        
    let app = create_app(env).await;
    let client = GraphQLClient::new(&app.url());
    
    // Run 100 operations with random failures
    let mut successes = 0;
    let mut retries = 0;
    
    for _ in 0..100 {
        match client.create_note(random_note()).await {
            Ok(_) => successes += 1,
            Err(e) if e.is_retryable() => retries += 1,
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
    
    // Should handle transient failures gracefully
    assert!(successes > 50, "Too many failures: {}/100", successes);
    println!("Handled {} transient failures", retries);
}
```

## End-to-End Tests

### Critical Path Tests (100% Coverage Required)

```rust
// tests/critical/authorization.rs
#[tokio::test]
async fn test_authorization_all_paths() {
    let scenarios = vec![
        // Test every authorization decision path
        ("user1", "note:123", "read", true, "owner can read"),
        ("user2", "note:123", "read", false, "non-owner cannot read"),
        ("user1", "note:123", "write", true, "owner can write"),
        ("user2", "note:123", "write", false, "non-owner cannot write"),
        ("user1", "note:123", "delete", true, "owner can delete"),
        ("admin", "note:123", "delete", true, "admin can delete any"),
        ("", "note:123", "read", false, "anonymous cannot read"),
    ];
    
    for (user, resource, action, expected, description) in scenarios {
        let result = is_authorized(
            &create_context(user),
            resource,
            action
        ).await;
        
        assert_eq!(
            result.is_ok(),
            expected,
            "Authorization test failed: {}",
            description
        );
    }
}

// tests/critical/retry_logic.rs
#[tokio::test]
async fn test_database_retry_all_scenarios() {
    // Test successful retry
    let db = MockDatabase::new()
        .fail_times(2)
        .then_succeed();
    let result = connect_with_retry(db).await;
    assert!(result.is_ok());
    
    // Test max retries exceeded
    let db = MockDatabase::new()
        .always_fail();
    let result = connect_with_retry(db).await;
    assert!(result.is_err());
    
    // Test exponential backoff timing
    let start = Instant::now();
    let db = MockDatabase::new()
        .fail_times(3)
        .then_succeed();
    let _ = connect_with_retry(db).await;
    let elapsed = start.elapsed();
    // Should take at least 1 + 2 + 4 = 7 seconds
    assert!(elapsed >= Duration::from_secs(7));
}

// tests/critical/circuit_breaker.rs  
#[tokio::test]
async fn test_circuit_breaker_all_transitions() {
    let breaker = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        timeout: Duration::from_secs(1),
        half_open_max_requests: 1,
    });
    
    // Test CLOSED -> OPEN transition
    for _ in 0..3 {
        let _ = breaker.call(failing_operation).await;
    }
    assert_eq!(breaker.state(), CircuitState::Open);
    
    // Test OPEN -> HALF_OPEN transition
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
    
    // Test HALF_OPEN -> CLOSED transition
    let _ = breaker.call(successful_operation).await;
    let _ = breaker.call(successful_operation).await;
    assert_eq!(breaker.state(), CircuitState::Closed);
    
    // Test HALF_OPEN -> OPEN transition
    for _ in 0..3 {
        let _ = breaker.call(failing_operation).await;
    }
    assert_eq!(breaker.state(), CircuitState::Open);
    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = breaker.call(failing_operation).await;
    assert_eq!(breaker.state(), CircuitState::Open);
}
```

### Complete User Flow Test
```rust
// tests/e2e/flows/note_lifecycle_test.rs
#[tokio::test]
async fn test_complete_note_lifecycle() {
    let env = TestEnvironment::new()
        .with_spicedb()
        .await;
        
    let app = create_app(env).await;
    let client = GraphQLClient::new(&app.url());
    
    // 1. Create a note
    let create_response = client.mutation(r#"
        mutation {
            createNote(input: {
                title: "Test Note"
                content: "This is a test"
                author: "test_user"
            }) {
                id
                title
                content
                author
            }
        }
    "#).await.unwrap();
    
    let note_id = create_response.data
        .get("createNote")
        .get("id")
        .as_str()
        .unwrap();
    
    // 2. Query the note
    let query_response = client.query(&format!(r#"
        {{
            note(id: "{}") {{
                id
                title
                content
            }}
        }}
    "#, note_id)).await.unwrap();
    
    assert_eq!(
        query_response.data.get("note").get("title").as_str().unwrap(),
        "Test Note"
    );
    
    // 3. Update the note
    let update_response = client.mutation(&format!(r#"
        mutation {{
            updateNote(id: "{}", input: {{
                title: "Updated Note"
            }}) {{
                id
                title
            }}
        }}
    "#, note_id)).await.unwrap();
    
    assert_eq!(
        update_response.data.get("updateNote").get("title").as_str().unwrap(),
        "Updated Note"
    );
    
    // 4. Delete the note
    let delete_response = client.mutation(&format!(r#"
        mutation {{
            deleteNote(id: "{}") 
        }}
    "#, note_id)).await.unwrap();
    
    assert_eq!(
        delete_response.data.get("deleteNote").as_bool().unwrap(),
        true
    );
    
    // 5. Verify deletion
    let verify_response = client.query(&format!(r#"
        {{
            note(id: "{}") {{
                id
            }}
        }}
    "#, note_id)).await.unwrap();
    
    assert!(verify_response.data.get("note").is_null());
}
```

### Subscription Test
```rust
// tests/e2e/subscriptions/note_subscription_test.rs
#[tokio::test]
async fn test_note_created_subscription() {
    let env = TestEnvironment::new().await;
    let app = create_app(env).await;
    
    // Connect WebSocket for subscription
    let ws_client = WebSocketClient::connect(&app.ws_url()).await;
    
    // Subscribe to noteCreated
    ws_client.send_message(json!({
        "type": "start",
        "id": "1",
        "payload": {
            "query": "subscription { noteCreated { id title author } }"
        }
    })).await;
    
    // Create a note via mutation
    let client = GraphQLClient::new(&app.url());
    client.mutation(r#"
        mutation {
            createNote(input: {
                title: "Subscription Test"
                content: "Testing subscriptions"
                author: "test_user"
            }) {
                id
            }
        }
    "#).await.unwrap();
    
    // Should receive subscription event
    let event = ws_client.receive_message().await;
    
    assert_eq!(event["type"], "data");
    assert_eq!(event["id"], "1");
    assert_eq!(
        event["payload"]["data"]["noteCreated"]["title"],
        "Subscription Test"
    );
}
```

## Performance Tests

### Load Testing
```rust
// tests/performance/load_test.rs
#[tokio::test]
#[ignore] // Run with cargo test -- --ignored --test load_test
async fn test_concurrent_requests() {
    let env = TestEnvironment::new().await;
    let app = create_app(env).await;
    let base_url = app.url();
    
    // Metrics collection
    let durations = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(AtomicU32::new(0));
    
    // Spawn 100 concurrent tasks
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let url = base_url.clone();
            let durations = durations.clone();
            let errors = errors.clone();
            
            tokio::spawn(async move {
                let client = GraphQLClient::new(&url);
                
                // Each task makes 10 requests
                for j in 0..10 {
                    let start = Instant::now();
                    
                    let result = client.mutation(&format!(r#"
                        mutation {{
                            createNote(input: {{
                                title: "Load Test {} - {}"
                                content: "Performance testing"
                                author: "user_{}"
                            }}) {{
                                id
                            }}
                        }}
                    "#, i, j, i)).await;
                    
                    let duration = start.elapsed();
                    
                    if result.is_ok() {
                        durations.lock().await.push(duration);
                    } else {
                        errors.fetch_add(1, Ordering::SeqCst);
                    }
                }
            })
        })
        .collect();
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Analyze results
    let durations = durations.lock().await;
    let total_requests = 1000;
    let failed_requests = errors.load(Ordering::SeqCst);
    let success_rate = (total_requests - failed_requests) as f64 / total_requests as f64;
    
    // Calculate percentiles
    let mut sorted_durations: Vec<_> = durations.iter().cloned().collect();
    sorted_durations.sort();
    
    let p50 = sorted_durations[sorted_durations.len() / 2];
    let p95 = sorted_durations[sorted_durations.len() * 95 / 100];
    let p99 = sorted_durations[sorted_durations.len() * 99 / 100];
    
    println!("Load Test Results:");
    println!("  Total Requests: {}", total_requests);
    println!("  Success Rate: {:.2}%", success_rate * 100.0);
    println!("  P50 Latency: {:?}", p50);
    println!("  P95 Latency: {:?}", p95);
    println!("  P99 Latency: {:?}", p99);
    
    // Assertions
    assert!(success_rate > 0.99, "Success rate should be >99%");
    assert!(p95 < Duration::from_secs(1), "P95 latency should be <1s");
}
```

## Test Configuration

### Cargo Configuration
```toml
# .cargo/config.toml
[env]
RUST_TEST_THREADS = "4"
RUST_BACKTRACE = "1"
RUST_LOG = "debug"

[profile.test]
opt-level = 0
debug = true

[alias]
test-unit = "test --lib"
test-integration = "test --test '*' --features integration"
test-e2e = "test --test '*' --features e2e"
test-all = "test --all-features"
test-coverage = "tarpaulin --all-features --out Html"
```

### Test Features
```toml
# Cargo.toml
[features]
default = []
demo = []
integration = ["testcontainers", "bollard"]
e2e = ["integration", "reqwest", "tokio-tungstenite"]

[dev-dependencies]
# Testing frameworks
tokio-test = "0.4"
proptest = "1.0"
fake = { version = "2.5", features = ["derive"] }
rstest = "0.18"

# Mocking
mockall = "0.11"
wiremock = "0.5"

# Test containers
testcontainers = { version = "0.14", optional = true }
bollard = { version = "0.14", optional = true }

# HTTP/WebSocket clients for testing
reqwest = { version = "0.11", features = ["json"], optional = true }
tokio-tungstenite = { version = "0.20", optional = true }

# Assertions
pretty_assertions = "1.3"
claims = "0.7"

# Test data generation
arbitrary = { version = "1.3", features = ["derive"] }
faker = "0.2"

# Coverage
tarpaulin = "0.27"
```

## CI/CD Pipeline

### GitHub Actions Example
```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    strategy:
      matrix:
        test-suite: [unit, integration, e2e]
    
    services:
      # Only needed for integration/e2e tests
      surrealdb:
        image: surrealdb/surrealdb:latest
        ports:
          - 8000:8000
        options: >-
          --health-cmd "curl -f http://localhost:8000/health || exit 1"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    
    steps:
      - uses: actions/checkout@v3
      
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
          
      - uses: Swatinem/rust-cache@v2
      
      - name: Run tests
        run: |
          if [ "${{ matrix.test-suite }}" = "unit" ]; then
            cargo test --lib
          elif [ "${{ matrix.test-suite }}" = "integration" ]; then
            cargo test --test '*' --features integration
          else
            cargo test --test '*' --features e2e
          fi
          
  coverage:
    runs-on: ubuntu-latest
    needs: test
    
    steps:
      - uses: actions/checkout@v3
      
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
          
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
        
      - name: Generate coverage
        run: cargo tarpaulin --all-features --out Xml
        
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          fail_ci_if_error: true
```

## Test Best Practices

### 1. Test Naming
```rust
// Good: Descriptive test names
#[test]
fn create_note_with_empty_title_returns_validation_error() { }

// Bad: Generic names
#[test]
fn test1() { }
```

### 2. Test Independence
Each test must be independent and not rely on execution order:
```rust
#[tokio::test]
async fn test_note_creation() {
    let env = TestEnvironment::new().await; // Fresh environment
    // ... test logic
} // Environment cleaned up automatically
```

### 3. Assertion Messages
```rust
assert_eq!(
    result.status, 
    "healthy",
    "Expected healthy status after all services started, got: {:?}",
    result
);
```

### 4. Test Data Isolation
```rust
// Use unique identifiers for test data
let author = format!("test_user_{}", Uuid::new_v4());
let title = format!("Test Note {}", Utc::now().timestamp());
```

### 5. Error Testing
```rust
// Test both success and failure paths
#[rstest]
#[case("", "Title cannot be empty")]
#[case("x".repeat(201), "Title too long")]
fn test_title_validation(#[case] title: String, #[case] expected_error: &str) {
    let result = validate_title(&title);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains(expected_error));
}
```
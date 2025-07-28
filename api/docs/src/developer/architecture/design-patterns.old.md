# Design Patterns

A comprehensive guide to the design patterns used throughout the PCF API codebase, providing reusable solutions to common problems and promoting consistent architecture.

<!-- toc -->

## Overview

The PCF API leverages a variety of design patterns to create a maintainable, scalable, and testable codebase. These patterns range from classic Gang of Four patterns adapted for Rust to modern async patterns and domain-specific solutions for API development.

## Architectural Patterns

### Layered Architecture (N-Tier)

The PCF API follows a clean layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│        Presentation Layer               │
│   (GraphQL Handlers, HTTP Endpoints)    │
├─────────────────────────────────────────┤
│        Business Logic Layer             │
│     (Services, Domain Logic)            │
├─────────────────────────────────────────┤
│        Data Access Layer                │
│    (Database Adapters, Models)          │
├─────────────────────────────────────────┤
│       Infrastructure Layer              │
│  (Config, Logging, Error Handling)      │
└─────────────────────────────────────────┘
```

**Implementation Example**:
```rust
// Presentation Layer
pub async fn graphql_handler(
    State(schema): State<AppSchema>,
    headers: HeaderMap,
    Json(request): Json<Request>,
) -> impl IntoResponse {
    let response = schema.execute(request).await;
    Json(response)
}

// Business Logic Layer
impl Query {
    async fn notes(&self, ctx: &Context<'_>) -> Result<Vec<Note>> {
        let db = ctx.data::<Arc<dyn DatabaseService>>()?;
        let notes = db.query("notes", Query::all()).await?;
        Ok(notes.into_iter().map(Into::into).collect())
    }
}

// Data Access Layer
impl DatabaseService for SurrealDatabase {
    async fn query(&self, collection: &str, query: Query) -> Result<Vec<Value>> {
        // Database-specific implementation
    }
}
```

### Hexagonal Architecture (Ports and Adapters)

The service layer implements hexagonal architecture for flexibility and testability:

```rust
// Port (Core Domain Interface)
#[async_trait]
pub trait DatabaseService: Send + Sync {
    async fn connect(&self) -> Result<(), DatabaseError>;
    async fn health_check(&self) -> DatabaseHealth;
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError>;
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError>;
}

// Adapter for SurrealDB
pub struct SurrealDatabase {
    client: Arc<RwLock<Option<Surreal<Db>>>>,
    config: DatabaseConfig,
}

// Adapter for Testing
pub struct MockDatabase {
    data: Arc<RwLock<HashMap<String, HashMap<String, Value>>>>,
    health: Arc<RwLock<DatabaseHealth>>,
}

// Usage through port
async fn handle_request(db: Arc<dyn DatabaseService>) -> Result<Note> {
    // Works with any DatabaseService implementation
    db.read("notes", "123").await?.ok_or(NotFound)
}
```

## Creational Patterns

### Builder Pattern

Used extensively for constructing complex objects with many optional parameters:

```rust
// GraphQL Schema Builder
pub fn create_schema(
    database: Arc<dyn DatabaseService>,
    config: GraphQLConfig,
) -> AppSchema {
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .data(database)
        .data(dataloaders)
        .limit_depth(config.max_depth)
        .limit_complexity(config.max_complexity);
    
    if !config.enable_introspection {
        builder = builder.disable_introspection();
    }
    
    builder.finish()
}

// Mock Database Builder for Tests
pub struct MockDatabaseBuilder {
    data: HashMap<String, HashMap<String, Value>>,
    errors: HashMap<String, DatabaseError>,
    delay: Option<Duration>,
    health_status: HealthStatus,
}

impl MockDatabaseBuilder {
    pub fn with_data(mut self, collection: &str, id: &str, data: Value) -> Self {
        self.data
            .entry(collection.to_string())
            .or_default()
            .insert(id.to_string(), data);
        self
    }
    
    pub fn with_error_on(mut self, operation: &str, error: DatabaseError) -> Self {
        self.errors.insert(operation.to_string(), error);
        self
    }
    
    pub fn build(self) -> MockDatabase {
        MockDatabase {
            data: Arc::new(RwLock::new(self.data)),
            errors: Arc::new(RwLock::new(self.errors)),
            delay: self.delay,
            health: Arc::new(RwLock::new(DatabaseHealth {
                status: self.health_status,
                // ...
            })),
        }
    }
}
```

### Factory Pattern

Creates objects without specifying their concrete classes:

```rust
// DataLoader Factory
pub fn create_dataloaders(database: Arc<dyn DatabaseService>) -> DataLoaderRegistry {
    DataLoaderRegistry {
        author_notes: DataLoader::new(AuthorNotesLoader::new(database.clone())),
        note_comments: DataLoader::new(NoteCommentsLoader::new(database.clone())),
        user_sessions: DataLoader::new(UserSessionLoader::new(database)),
    }
}

// Error Response Factory
impl AppError {
    pub fn to_response(&self) -> Response<Body> {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::Server(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };
        
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(json!({ "error": message }).to_string().into())
            .unwrap()
    }
}
```

### Singleton Pattern (Thread-Safe)

Implemented using Rust's `OnceLock` for one-time initialization:

```rust
use std::sync::OnceLock;

// Global sanitization patterns
static PATTERNS: OnceLock<SanitizationPatterns> = OnceLock::new();

pub fn get_patterns() -> &'static SanitizationPatterns {
    PATTERNS.get_or_init(|| {
        SanitizationPatterns {
            email: Regex::new(r"[\w._%+-]+@[\w.-]+\.[A-Za-z]{2,}").unwrap(),
            credit_card: Regex::new(r"\b\d{13,19}\b").unwrap(),
            api_key: Regex::new(r"\b(sk_|pk_|api_|key_)[a-zA-Z0-9]+\b").unwrap(),
            // ... more patterns
        }
    })
}

// Version checker singleton
lazy_static! {
    static ref VERSION_CHECKER: VersionChecker = VersionChecker::new();
}
```

## Structural Patterns

### Adapter Pattern

Adapts interfaces to work together:

```rust
// Adapts SurrealDB to our generic DatabaseService interface
impl DatabaseService for SurrealDatabase {
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError> {
        let client = self.get_client().await?;
        
        // Adapt our generic Value to SurrealDB's format
        let result: Option<Record> = client
            .create(collection)
            .content(data)
            .await
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            
        // Adapt SurrealDB's response to our format
        result
            .map(|r| r.id.to_string())
            .ok_or(DatabaseError::QueryFailed("No ID returned".into()))
    }
}

// Adapts async-graphql errors to our error type
impl From<AppError> for async_graphql::Error {
    fn from(err: AppError) -> Self {
        Error::new(err.to_string())
            .extend_with(|_, e| {
                e.set("code", err.error_code());
                e.set("status", err.status_code().as_u16());
            })
    }
}
```

### Facade Pattern

Provides a simplified interface to complex subsystems:

```rust
// GraphQL Schema acts as a facade
pub type AppSchema = Schema<Query, Mutation, Subscription>;

pub fn create_production_schema(
    config: GraphQLConfig,
    database: Arc<dyn DatabaseService>,
) -> AppSchema {
    // Hides complex setup behind simple interface
    let dataloaders = create_dataloaders(database.clone());
    let event_broadcaster = EventBroadcaster::new();
    
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .data(config.clone())
        .data(database)
        .data(dataloaders)
        .data(event_broadcaster)
        .extension(DepthLimit::new(config.max_depth))
        .extension(ComplexityLimit::new(config.max_complexity))
        .extension(MetricsExtension);
    
    if !config.enable_introspection {
        builder = builder.disable_introspection();
    }
    
    builder.finish()
}

// Simple usage hides complexity
let schema = create_production_schema(config, database);
```

### Proxy Pattern

Provides a placeholder or surrogate for another object:

```rust
// Health check caching acts as a proxy
pub struct HealthManager {
    services: Arc<RwLock<HashMap<String, ServiceHealth>>>,
    cache: Arc<RwLock<Option<CachedResponse>>>,
}

impl HealthManager {
    pub async fn get_health(&self) -> HealthResponse {
        let cache_guard = self.cache.read().await;
        
        // Return cached response if valid
        if let Some(cached) = &*cache_guard {
            if cached.expires_at > Utc::now() {
                return cached.response.clone();
            }
            
            // Stale-while-revalidate pattern
            if cached.stale_until > Utc::now() {
                tokio::spawn({
                    let manager = self.clone();
                    async move {
                        manager.refresh_health().await;
                    }
                });
                return cached.response.clone();
            }
        }
        
        drop(cache_guard);
        
        // Otherwise compute fresh response
        self.refresh_health().await
    }
}
```

### Decorator Pattern

Adds new functionality to objects without altering their structure:

```rust
// Logging decorator for database service
pub struct LoggingDatabaseService<T: DatabaseService> {
    inner: T,
}

#[async_trait]
impl<T: DatabaseService> DatabaseService for LoggingDatabaseService<T> {
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError> {
        let start = Instant::now();
        
        info!("Creating record in collection: {}", collection);
        
        let result = self.inner.create(collection, data).await;
        
        match &result {
            Ok(id) => {
                info!(
                    "Successfully created record {} in {} ({:?})",
                    id, collection, start.elapsed()
                );
            }
            Err(e) => {
                error!(
                    "Failed to create record in {}: {} ({:?})",
                    collection, e, start.elapsed()
                );
            }
        }
        
        result
    }
    
    // Delegate other methods with logging
}

// Usage
let database = SurrealDatabase::new(config);
let logged_database = LoggingDatabaseService::new(database);
```

## Behavioral Patterns

### Strategy Pattern

Defines a family of algorithms and makes them interchangeable:

```rust
// Persistence strategy for write queue
#[derive(Debug, Clone)]
pub enum PersistenceFormat {
    Json,
    Bincode,
}

impl WriteQueue {
    pub async fn save_to_disk(&self) -> Result<(), QueueError> {
        let queue = self.queue.read().await;
        let data = queue.iter().cloned().collect::<Vec<_>>();
        
        // Strategy selection
        let bytes = match self.config.persistence_format {
            PersistenceFormat::Json => {
                serde_json::to_vec_pretty(&data)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?
            }
            PersistenceFormat::Bincode => {
                bincode::serialize(&data)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?
            }
        };
        
        tokio::fs::write(&self.config.persistence_path, bytes).await?;
        Ok(())
    }
}

// Retry strategy
pub struct RetryStrategy {
    strategy: RetryStrategyType,
}

pub enum RetryStrategyType {
    Exponential { base: Duration, max: Duration },
    Linear { delay: Duration },
    Fixed { delay: Duration },
}

impl RetryStrategy {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match &self.strategy {
            RetryStrategyType::Exponential { base, max } => {
                let delay = *base * 2u32.pow(attempt.min(10));
                delay.min(*max)
            }
            RetryStrategyType::Linear { delay } => *delay * attempt,
            RetryStrategyType::Fixed { delay } => *delay,
        }
    }
}
```

### Observer Pattern

Defines a one-to-many dependency between objects:

```rust
// Event broadcasting for GraphQL subscriptions
pub struct EventBroadcaster {
    subscribers: Arc<RwLock<HashMap<Uuid, Sender<DomainEvent>>>>,
}

impl EventBroadcaster {
    pub async fn subscribe(&self) -> EventStream {
        let (tx, rx) = mpsc::channel(100);
        let id = Uuid::new_v4();
        
        self.subscribers.write().await.insert(id, tx);
        
        EventStream { id, receiver: rx }
    }
    
    pub async fn broadcast(&self, event: DomainEvent) {
        let subscribers = self.subscribers.read().await;
        
        for (_, sender) in subscribers.iter() {
            // Notify all observers
            let _ = sender.send(event.clone()).await;
        }
    }
}

// Usage in mutations
impl Mutation {
    async fn create_note(&self, ctx: &Context<'_>, input: CreateNoteInput) -> Result<Note> {
        let note = // ... create note
        
        // Notify observers
        let broadcaster = ctx.data::<EventBroadcaster>()?;
        broadcaster.broadcast(DomainEvent::NoteCreated(note.clone())).await;
        
        Ok(note)
    }
}
```

### Chain of Responsibility

Passes requests along a chain of handlers:

```rust
// Error conversion chain
impl From<DatabaseError> for AppError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::NotFound(msg) => AppError::NotFound(msg),
            DatabaseError::ValidationFailed(msg) => AppError::InvalidInput(msg),
            DatabaseError::Timeout(_) | DatabaseError::ConnectionFailed(_) => {
                AppError::ServiceUnavailable(err.to_string())
            }
            _ => AppError::Server(err.to_string()),
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(err: ConfigError) -> Self {
        AppError::Config(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Server(err.to_string())
    }
}

// Middleware chain
let app = Router::new()
    .layer(TraceLayer::new())      // First handler
    .layer(CorsLayer::new())        // Second handler
    .layer(AuthLayer::new())        // Third handler
    .layer(RateLimitLayer::new());  // Fourth handler
```

### Command Pattern

Encapsulates requests as objects:

```rust
// Queued operations as commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedOperation {
    pub id: Uuid,
    pub operation_type: OperationType,
    pub collection: String,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update { id: String },
    Delete { id: String },
}

impl QueuedOperation {
    pub async fn execute(&self, db: &dyn DatabaseService) -> Result<(), DatabaseError> {
        match &self.operation_type {
            OperationType::Create => {
                db.create(&self.collection, self.data.clone()).await?;
            }
            OperationType::Update { id } => {
                db.update(&self.collection, id, self.data.clone()).await?;
            }
            OperationType::Delete { id } => {
                db.delete(&self.collection, id).await?;
            }
        }
        Ok(())
    }
}
```

## Async/Concurrent Patterns

### Shared State Pattern

Thread-safe shared state using Arc and RwLock:

```rust
// Thread-safe write queue
pub struct WriteQueue {
    queue: Arc<RwLock<VecDeque<QueuedOperation>>>,
    metrics: Arc<RwLock<QueueMetrics>>,
    processing: Arc<AtomicBool>,
}

impl WriteQueue {
    pub async fn enqueue(&self, operation: QueuedOperation) -> Result<(), QueueError> {
        let mut queue = self.queue.write().await;
        
        if queue.len() >= self.config.max_size {
            return Err(QueueError::QueueFull);
        }
        
        queue.push_back(operation);
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_enqueued += 1;
        
        Ok(())
    }
}
```

### Actor Pattern

Encapsulates state and behavior:

```rust
// Health manager acts like an actor
pub struct HealthManager {
    state: Arc<RwLock<HealthState>>,
    command_tx: mpsc::Sender<HealthCommand>,
}

enum HealthCommand {
    CheckService(String),
    UpdateStatus(String, ServiceStatus),
    GetStatus(oneshot::Sender<HealthStatus>),
}

impl HealthManager {
    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut rx = self.command_rx;
            
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    HealthCommand::CheckService(name) => {
                        // Perform health check
                    }
                    HealthCommand::UpdateStatus(name, status) => {
                        // Update internal state
                    }
                    HealthCommand::GetStatus(reply) => {
                        // Send current status
                        let _ = reply.send(self.compute_status().await);
                    }
                }
            }
        })
    }
}
```

### Future Composition

Combining async operations:

```rust
// Graceful shutdown with multiple signals
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT, shutting down");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down");
        }
    }
}

// Concurrent initialization
pub async fn initialize_services(config: &AppConfig) -> Result<Services> {
    let (db_result, cache_result, auth_result) = tokio::join!(
        create_database_service(&config.database),
        create_cache_service(&config.cache),
        create_auth_service(&config.auth)
    );
    
    Ok(Services {
        database: db_result?,
        cache: cache_result?,
        auth: auth_result?,
    })
}
```

## Error Handling Patterns

### Error Type Hierarchy

Structured error types with clear semantics:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Internal server error: {0}")]
    Server(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Server(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

### Result Pattern with Early Return

Leverages the ? operator for clean error propagation:

```rust
pub async fn handle_request(
    db: Arc<dyn DatabaseService>,
    auth: Arc<dyn AuthService>,
    request: Request,
) -> Result<Response, AppError> {
    // Early return on auth failure
    let session = auth.verify_token(&request.token).await
        .map_err(|_| AppError::Unauthorized)?;
    
    // Early return on invalid input
    let input = validate_input(&request.body)
        .map_err(|e| AppError::InvalidInput(e.to_string()))?;
    
    // Early return on database error
    let result = db.query("notes", input).await
        .map_err(|e| AppError::from(e))?;
    
    Ok(Response::success(result))
}
```

## Testing Patterns

### Mock Pattern

Test doubles for dependencies:

```rust
pub struct MockDatabase {
    responses: Arc<RwLock<HashMap<String, Result<Value, DatabaseError>>>>,
    call_count: Arc<AtomicU32>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }
    
    pub fn expect_read(&self, id: &str, response: Result<Value, DatabaseError>) {
        self.responses.write().unwrap()
            .insert(format!("read:{}", id), response);
    }
    
    pub fn verify_called(&self, times: u32) {
        assert_eq!(self.call_count.load(Ordering::Relaxed), times);
    }
}

#[async_trait]
impl DatabaseService for MockDatabase {
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError> {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        
        let key = format!("read:{}", id);
        let responses = self.responses.read().unwrap();
        
        match responses.get(&key) {
            Some(Ok(value)) => Ok(Some(value.clone())),
            Some(Err(e)) => Err(e.clone()),
            None => Ok(None),
        }
    }
}
```

### Builder Pattern for Test Data

Simplifies test data construction:

```rust
pub struct NoteBuilder {
    id: Option<Uuid>,
    title: String,
    content: String,
    author: String,
    tags: Vec<String>,
    created_at: Option<DateTime<Utc>>,
}

impl NoteBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            author: "test@example.com".to_string(),
            tags: vec![],
            created_at: None,
        }
    }
    
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    pub fn build(self) -> Note {
        Note {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            title: self.title,
            content: self.content,
            author: self.author,
            tags: self.tags,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            updated_at: Utc::now(),
        }
    }
}

// Usage in tests
#[test]
fn test_note_creation() {
    let note = NoteBuilder::new()
        .with_title("Important Note")
        .with_tags(vec!["urgent".to_string(), "todo".to_string()])
        .build();
    
    assert_eq!(note.title, "Important Note");
    assert_eq!(note.tags.len(), 2);
}
```

## Domain-Specific Patterns

### Repository Pattern

Abstracts data access logic:

```rust
// Generic repository trait
#[async_trait]
pub trait Repository<T, ID> {
    async fn find_by_id(&self, id: &ID) -> Result<Option<T>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<T>, RepositoryError>;
    async fn save(&self, entity: &T) -> Result<ID, RepositoryError>;
    async fn delete(&self, id: &ID) -> Result<(), RepositoryError>;
}

// Concrete implementation
pub struct NoteRepository {
    database: Arc<dyn DatabaseService>,
}

#[async_trait]
impl Repository<Note, Uuid> for NoteRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Note>, RepositoryError> {
        let value = self.database.read("notes", &id.to_string()).await?;
        Ok(value.map(|v| serde_json::from_value(v).unwrap()))
    }
    
    async fn save(&self, note: &Note) -> Result<Uuid, RepositoryError> {
        let value = serde_json::to_value(note)?;
        let id = self.database.create("notes", value).await?;
        Ok(Uuid::parse_str(&id)?)
    }
}
```

### DataLoader Pattern

Solves N+1 query problems in GraphQL:

```rust
pub struct AuthorNotesLoader {
    database: Arc<dyn DatabaseService>,
}

#[async_trait::async_trait]
impl Loader<String> for AuthorNotesLoader {
    type Value = Vec<Note>;
    type Error = AppError;
    
    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        // Batch load all notes for multiple authors
        let query = Query::new()
            .filter("author", "IN", keys.to_vec());
            
        let notes = self.database.query("notes", query).await?;
        
        // Group by author
        let mut result: HashMap<String, Vec<Note>> = HashMap::new();
        for note in notes {
            let note: Note = serde_json::from_value(note)?;
            result.entry(note.author.clone())
                .or_default()
                .push(note);
        }
        
        // Ensure all keys have entries
        for key in keys {
            result.entry(key.clone()).or_default();
        }
        
        Ok(result)
    }
}
```

### Unit of Work Pattern

Manages database transactions:

```rust
pub struct UnitOfWork {
    operations: Vec<Operation>,
    database: Arc<dyn DatabaseService>,
}

impl UnitOfWork {
    pub fn new(database: Arc<dyn DatabaseService>) -> Self {
        Self {
            operations: Vec::new(),
            database,
        }
    }
    
    pub fn create(&mut self, collection: &str, data: Value) {
        self.operations.push(Operation::Create {
            collection: collection.to_string(),
            data,
        });
    }
    
    pub fn update(&mut self, collection: &str, id: &str, data: Value) {
        self.operations.push(Operation::Update {
            collection: collection.to_string(),
            id: id.to_string(),
            data,
        });
    }
    
    pub async fn commit(self) -> Result<(), DatabaseError> {
        // Execute all operations in a transaction
        self.database.transaction(|tx| async {
            for op in self.operations {
                match op {
                    Operation::Create { collection, data } => {
                        tx.create(&collection, data).await?;
                    }
                    Operation::Update { collection, id, data } => {
                        tx.update(&collection, &id, data).await?;
                    }
                }
            }
            Ok(())
        }).await
    }
}
```

### Circuit Breaker Pattern

Prevents cascading failures:

```rust
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure_time: RwLock<Option<Instant>>,
    state: RwLock<CircuitState>,
    config: CircuitBreakerConfig,
}

#[derive(Clone, Copy)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<AppError>,
    {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Open => {
                if self.should_attempt_reset().await {
                    *self.state.write().await = CircuitState::HalfOpen;
                } else {
                    return Err(AppError::ServiceUnavailable("Circuit breaker open".into()).into());
                }
            }
            _ => {}
        }
        
        match operation.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error)
            }
        }
    }
    
    async fn on_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        *self.state.write().await = CircuitState::Closed;
    }
    
    async fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure_time.write().await = Some(Instant::now());
        
        if failures >= self.config.failure_threshold {
            *self.state.write().await = CircuitState::Open;
        }
    }
}
```

## Best Practices

### Pattern Selection Guidelines

1. **Use Builder Pattern when**:
   - Objects have many optional parameters
   - Construction logic is complex
   - You need immutable objects

2. **Use Factory Pattern when**:
   - Creation logic depends on runtime conditions
   - You need to abstract object creation
   - Multiple implementations exist

3. **Use Adapter Pattern when**:
   - Integrating third-party libraries
   - Converting between different interfaces
   - Maintaining backward compatibility

4. **Use Proxy Pattern when**:
   - Adding caching layer
   - Implementing lazy loading
   - Adding access control

5. **Use Observer Pattern when**:
   - Implementing event systems
   - Building reactive features
   - Decoupling components

### Anti-Patterns to Avoid

1. **God Object**: Avoid creating objects that do too much
2. **Anemic Domain Model**: Ensure domain objects contain behavior, not just data
3. **Premature Optimization**: Don't add patterns until they're needed
4. **Pattern Overuse**: Keep it simple when simple works

### Rust-Specific Considerations

1. **Ownership**: Patterns must respect Rust's ownership rules
2. **Trait Bounds**: Use trait objects for runtime polymorphism
3. **Zero-Cost Abstractions**: Prefer compile-time patterns when possible
4. **Error Handling**: Integrate patterns with Result types

## Summary

The PCF API demonstrates mature use of design patterns adapted for Rust's unique features. Key takeaways:

- **Architectural patterns** provide structure and separation of concerns
- **Creational patterns** simplify object construction and configuration
- **Structural patterns** enable flexible composition and adaptation
- **Behavioral patterns** manage complex interactions and algorithms
- **Async patterns** handle concurrent operations safely
- **Domain patterns** solve API-specific challenges

These patterns work together to create a maintainable, testable, and scalable codebase that leverages Rust's strengths while following established software engineering principles.
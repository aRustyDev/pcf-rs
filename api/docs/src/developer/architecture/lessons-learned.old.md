# Lessons Learned

Key insights and best practices discovered during the development of the PCF API project, extracted from code reviews and implementation experience.

<!-- toc -->

## Overview

This document captures the most valuable lessons learned throughout the PCF API development process. These insights come from multiple code review checkpoints, implementation challenges, and architectural decisions that shaped the project.

## Architecture & Design

### Start Small, Build Incrementally

**Challenge**: Initial attempts to load all dependencies (SurrealDB, GraphQL, Kratos) at once caused build failures and complexity overload.

**Lesson**: Follow a strict phased approach, adding dependencies only when needed.

**Implementation**:
```rust
// Phase 1: Minimal dependencies
[dependencies]
tokio = { version = "1.35", features = ["macros", "rt-multi-thread"] }
axum = "0.7"

// Phase 2: Add as needed
async-graphql = "6.0"  // Added when implementing GraphQL
surrealdb = "1.0"      // Added when implementing persistence
```

**Result**: Clean builds, fast compile times, easier debugging.

### Library vs Binary Separation

**Discovery**: Creating `lib.rs` alongside `main.rs` dramatically improved testability.

**Benefits**:
- Better integration testing without full binary
- Modular, reusable components
- Clear public API surface

```rust
// src/lib.rs - Public API
pub mod config;
pub mod error;
pub mod graphql;

// src/main.rs - Thin binary layer
use pcf_api::{config::load_config, server::run};

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config()?;
    run(config).await
}
```

### Trait-Based Architecture

**Success Pattern**: Using traits for service boundaries enabled clean testing and modularity.

```rust
#[async_trait]
pub trait DatabaseService: Send + Sync {
    async fn get_note(&self, id: Uuid) -> Result<Option<Note>>;
    async fn create_note(&self, input: CreateNoteInput) -> Result<Note>;
}

// Easy mocking for tests
pub struct MockDatabaseService {
    notes: Arc<RwLock<HashMap<Uuid, Note>>>,
}
```

## Testing & Quality

### Test-Driven Development Discipline

**Observation**: Code written test-first was cleaner and more focused.

**Example**:
```rust
#[test]
fn test_error_should_have_status_code() {
    assert_eq!(AppError::NotFound.status_code(), StatusCode::NOT_FOUND);
    assert_eq!(AppError::InvalidInput.status_code(), StatusCode::BAD_REQUEST);
}

// This test drove the implementation of status_code()
impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::InvalidInput => StatusCode::BAD_REQUEST,
            // ...
        }
    }
}
```

### Integration Tests Reveal Hidden Gaps

**Problem**: Unit tests passed but system didn't work end-to-end.

**Example**: Logging system was implemented but never initialized.

```rust
// This test caught the missing initialization
#[tokio::test]
async fn test_server_logs_requests() {
    let app = create_test_app();
    let response = app.get("/health").await;
    
    // This failed until logging was properly initialized
    assert!(logs_contain("GET /health"));
}
```

**Lesson**: Always have end-to-end tests that verify actual usage.

### Mock Implementations with Builder Pattern

**Success**: Mock services with builder pattern enabled comprehensive testing.

```rust
let mock_db = MockDatabaseService::builder()
    .with_note(Note {
        id: test_id,
        title: "Test".into(),
        content: "Content".into(),
    })
    .with_error_on_create(DatabaseError::ConnectionLost)
    .build();
```

## Error Handling

### Production vs Development Error Messages

**Critical Learning**: Never expose internal details in production errors.

```rust
impl AppError {
    pub fn client_message(&self) -> String {
        match self {
            // Safe to expose
            AppError::InvalidInput(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            
            // Hide internal details
            AppError::Database(_) => "Database operation failed".into(),
            AppError::Internal(_) => {
                if cfg!(debug_assertions) {
                    format!("Internal error: {:?}", self)
                } else {
                    "An unexpected error occurred".into()
                }
            }
        }
    }
}
```

### Error Conversion Chains

**Problem**: Missing `From` implementations broke at integration points.

**Solution**: Plan error propagation paths early.

```rust
// Complete error chain
DatabaseError -> AppError -> GraphQL Error -> HTTP Response

impl From<DatabaseError> for AppError {
    fn from(err: DatabaseError) -> Self {
        AppError::Database(err)
    }
}

impl From<AppError> for async_graphql::Error {
    fn from(err: AppError) -> Self {
        Error::new(err.client_message())
            .extend_with(|_, e| {
                e.set("code", err.error_code());
            })
    }
}
```

### No Unwrap in Production Code

**Critical Finding**: `.unwrap()` in version parsing caused panics.

**Solution**:
```rust
// Bad
const VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

// Good
lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION should be valid semver");
}

// Better for runtime values
pub fn parse_version(s: &str) -> Result<Version, VersionError> {
    Version::parse(s).map_err(|e| VersionError::InvalidFormat(e.to_string()))
}
```

## Configuration Management

### Hierarchical Configuration Success

**Pattern**: 4-tier configuration system provided excellent flexibility.

```rust
let config = Figment::new()
    .merge(Serialized::defaults(AppConfig::default()))      // 1. Defaults
    .merge(Toml::file("config/default.toml"))              // 2. Base file
    .merge(Toml::file(format!("config/{}.toml", env)))     // 3. Environment
    .merge(Env::prefixed("PCF_API__"))                     // 4. Env vars
    .extract()?;
```

### Validation at Parse Time

**Learning**: Catch configuration errors immediately, not at usage time.

```rust
#[derive(Deserialize, Validate)]
pub struct ServerConfig {
    #[garde(range(min = 1024, max = 65535))]
    pub port: u16,
    
    #[garde(ip)]
    pub host: IpAddr,
}

// Validate immediately after loading
let config: AppConfig = figment.extract()?;
config.validate()?;  // Fails fast with clear errors
```

### Configuration Integration Gaps

**Common Issue**: Config system built but not wired to application.

**Checklist**:
- ✓ Configuration loaded in main()
- ✓ Config passed to all components
- ✓ Environment variables tested
- ✓ File overrides verified
- ✓ Validation errors handled

## Security Considerations

### Sanitization Must Be End-to-End

**Problem**: Log sanitization implemented but not connected to logging pipeline.

**Solution**: Ensure security features are integrated throughout:

```rust
// Not enough to just have sanitization functions
pub fn sanitize_log(message: &str) -> String { /* ... */ }

// Must be integrated into logging layer
let (non_blocking, _guard) = tracing_appender::non_blocking(stdout());
let subscriber = FmtSubscriber::builder()
    .with_env_filter(EnvFilter::from_default_env())
    .fmt_layer()
    .with_writer(non_blocking)
    .with_filter(SanitizingFilter::new())  // Actually use it!
    .finish();
```

### Feature-Gated Security Controls

**Success Pattern**: Compile-time security decisions prevent accidents.

```rust
#[cfg(feature = "demo")]
pub fn graphql_playground() -> impl IntoResponse {
    GraphQLPlaygroundConfig::default()
}

#[cfg(not(feature = "demo"))]
pub fn graphql_playground() -> impl IntoResponse {
    StatusCode::NOT_FOUND  // Doesn't exist in production
}
```

## Performance Insights

### Async-First Architecture

**Learning**: Design async from the ground up, don't retrofit.

```rust
// Good: Async traits from the start
#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check(&self) -> HealthStatus;
}

// Bad: Sync trait that blocks
pub trait HealthCheck {
    fn check(&self) -> HealthStatus;  // Blocks the runtime
}
```

### Resource Limiting Patterns

**Discovery**: Multiple layers of resource control work best.

```rust
// Connection pool limits
let pool = PoolOptions::new()
    .max_connections(config.database.max_connections)
    .min_connections(config.database.min_connections)
    .connect_timeout(config.database.connect_timeout)
    .create_pool()?;

// Request-level limits
let semaphore = Arc::new(Semaphore::new(config.server.max_concurrent_requests));

// Query complexity limits
let schema = Schema::build(Query, Mutation, Subscription)
    .extension(DepthLimit::new(config.graphql.max_depth))
    .extension(ComplexityLimit::new(config.graphql.max_complexity))
    .finish();
```

## Developer Experience

### Clear Module Structure

**Success**: Consistent module organization made navigation easy.

```
src/
├── module_name/
│   ├── mod.rs          # Public interface
│   ├── types.rs        # Type definitions
│   ├── handlers.rs     # HTTP handlers (if applicable)
│   ├── service.rs      # Business logic
│   └── tests.rs        # Unit tests
```

### Progressive Enhancement

**Learning**: Each phase should build cleanly on the previous.

- Phase 1: Core structure
- Phase 2: Add features
- Phase 3: Production hardening
- Phase 4: Advanced features

No major refactoring needed between phases.

## Common Pitfalls and Solutions

### Incomplete Integration

**Pattern**: Component built but not connected.

**Solution**: Integration checklist for each component:
- [ ] Component implemented
- [ ] Wired into dependency injection
- [ ] Configuration connected
- [ ] Tests verify integration
- [ ] Logs show component active

### Test-Only Code in Production

**Problem**: Debug code leaked into production paths.

**Solution**: Clear separation:
```rust
#[cfg(test)]
mod test_helpers {
    pub fn create_test_note() -> Note { /* ... */ }
}

// Use cfg_attr for test-only derives
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]  // PartialEq only in tests
pub struct Note { /* ... */ }
```

### Over-Engineering Early

**Anti-pattern**: Adding all features upfront.

**Better Approach**:
1. Implement minimum viable feature
2. Add tests
3. Get it working end-to-end
4. Enhance based on actual needs

## Key Success Patterns

### Build → Test → Integrate → Verify

Every component needs the full cycle:

1. **Build**: Implement the component
2. **Test**: Unit and integration tests
3. **Integrate**: Wire into the system
4. **Verify**: End-to-end testing

### Feature Flags for Environment Differences

```rust
// Compile-time for security features
#[cfg(feature = "production")]
const INTROSPECTION_ENABLED: bool = false;

// Runtime for operational features
let playground_enabled = config.graphql.playground && cfg!(feature = "demo");
```

### Observability as First-Class Concern

**Not an afterthought**: Build in from the start.

```rust
#[tracing::instrument(
    name = "graphql_request",
    skip(schema, request),
    fields(
        operation_name = %request.operation_name().unwrap_or("unnamed"),
        trace_id = %trace_id,
    )
)]
async fn graphql_handler(
    State(schema): State<AppSchema>,
    headers: HeaderMap,
    Json(request): Json<Request>,
) -> impl IntoResponse {
    // Observability built into every handler
}
```

## Conclusion

The PCF API project demonstrated that disciplined, phased development with:
- Strong testing practices
- Security-first thinking
- Clear architectural boundaries
- Comprehensive observability

Leads to robust, production-ready systems. The iterative review process caught issues early and ensured high quality throughout.

The most important lesson: **Every component must be built, tested, integrated, and verified end-to-end before moving forward.**

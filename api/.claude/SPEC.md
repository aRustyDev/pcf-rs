## Critical Requirements

### Core Requirements

**Stability & Reliability:**
- API Servers MUST NEVER exit without logging a message describing the reason for the exit with level ERROR or FATAL.
- API Servers MUST only exit due to a critical error that prevents safe operation, or when receiving SIGTERM/SIGINT signals.
- API Servers MUST NOT use `.unwrap()` or `.expect()` in production code paths. All Results MUST be handled explicitly.
- API Servers MUST implement graceful shutdown with a 30-second timeout for in-flight requests.

**Database Connectivity:**
- API Servers MUST retry failed database connections with exponential backoff starting at 1s, doubling up to 60s max.
- During startup: MUST continue retrying indefinitely until connection succeeds.
- During operation: SHOULD use cached data for reads when database is unavailable.
- MUST queue writes for retry when database is unavailable, up to a configurable limit (default: 1000 items).
- If database unavailable > 30s: MUST return 503 Service Unavailable with Retry-After header.

**Health Checks:**
- API Servers MUST provide health check endpoints compatible with container orchestration:
  - `/health` - Liveness check returning 200 OK if the process is running
  - `/health/ready` - Readiness check returning 200 OK only when all critical services are available
- Health checks MUST complete within 5 seconds or return 503 Service Unavailable.
- Health check endpoints MUST NOT require authentication.

**GraphQL Requirements:**
- This API server MUST support GraphQL queries, mutations, and subscriptions over HTTP and WebSocket.
- Query complexity MUST be calculated and limited to prevent resource exhaustion (default: 1000 complexity points).
- Query depth MUST be limited to prevent deep nesting attacks (default: 15 levels).
- If limits are exceeded, MUST return 400 Bad Request with clear error message.

**Observability:**
- This API server MUST produce Prometheus-compatible metrics at `/metrics` endpoint.
- Metric cardinality per metric name MUST NOT exceed 1000 unique label combinations.
- This API server MUST produce structured JSON logs with trace correlation.
- All log entries MUST include: timestamp, level, module, trace_id, and message fields.
- Sensitive data (passwords, tokens, PII) MUST NEVER be logged at any level.

**Architecture:**
- This API server MUST be modular with clear separation of concerns.
- All modules MUST implement OpenTelemetry tracing with span creation for significant operations.
- Configuration MUST use Figment for 4-tier precedence: defaults → files → env → CLI args.
- All configuration MUST be validated with Garde before use.

**Security:**
- Demo functionality MUST be behind a compile-time feature flag.
- Release builds MUST fail compilation if demo feature is enabled:
  ```rust
  #[cfg(all(not(debug_assertions), feature = "demo"))]  
  compile_error!("Demo mode MUST NOT be enabled in release builds");
  ```
- All user inputs MUST be validated before processing.
- Authentication MUST be enforced for all GraphQL operations except introspection in demo mode.

## Module Tree

```
src/
├── main.rs                    # Bootstrap server, setup tracing, health check loop, GraphQL server
├── config.rs                  # Figment config management & Garde validation
├── health.rs                  # Health check endpoints (/health, /health/ready)
├── helpers/                   # Shared utilities
│   └── authorization.rs       # Standardized is_authorized function
│
├── schema/                    # Single source of truth for all types
│   ├── mod.rs                # Schema traits and conversion utilities
│   ├── demo/                 # Demo types (only with demo feature flag)
│   │   └── note.rs          # Note type with GraphQL/DB derives
│   └── core/                 # Production types (future)
│
├── graphql/                   # GraphQL-specific implementation
│   ├── mod.rs                # Schema builder and setup
│   ├── context.rs            # Request context (auth, dataloaders)
│   ├── resolvers/            # GraphQL resolver implementations
│   │   ├── queries.rs        # Query resolvers
│   │   ├── mutations.rs      # Mutation resolvers
│   │   └── subscriptions.rs  # Subscription resolvers
│   └── errors.rs             # GraphQL error handling
│
├── auth/                      # Authentication & Authorization
│   ├── mod.rs                # Auth traits and interfaces
│   ├── session.rs            # Session extraction from headers
│   ├── cache.rs              # Authorization result caching
│   ├── spicedb/              # SpiceDB integration (future)
│   │   └── client.rs         # SpiceDB client wrapper
│   └── hydra/                # OAuth2/OIDC integration (future)
│
├── services/                  # External service integrations
│   ├── mod.rs                # Service traits and registry
│   ├── database/             # Database implementations
│   │   ├── mod.rs           # Database trait definition
│   │   └── surrealdb.rs     # SurrealDB adapter
│   └── microservices/        # Microservice clients (future)
│
└── middleware/               # Cross-cutting concerns
    ├── mod.rs
    ├── tracing.rs           # Request tracing setup
    ├── metrics.rs           # Metrics collection
    └── retry.rs             # Retry logic for external services
```

## GraphQL Schema

The API uses a schema-first approach with a single source of truth for types. Types are defined once with multiple derive macros for GraphQL, database serialization, and validation.

**Schema Requirements:**
- Types defined in `schema/` modules with appropriate derives
- GraphQL types automatically derived from Rust structs
- Database types use the same structs with serde
- Schema export available at `/schema` endpoint (demo mode only)
- Strong typing throughout with compile-time validation

**Demo Schema:**
- Simple note-taking API for testing and development
- Includes: Note type with CRUD operations
- Subscriptions for real-time updates
- No authentication required in demo mode

## Authorization

Authorization uses a standardized pattern with SpiceDB as the authorization engine.

**Pattern:**
- Single `is_authorized(ctx, resource, action)` helper function
- Called explicitly at the start of each resolver
- Results cached for configurable duration (5 minutes default)
- SpiceDB handles all complex permission logic
- API server only asks "can user X do action Y on resource Z"

**Cache Strategy:**
- Authorization results cached per user+resource+action
- Cache TTL configurable in debug builds only
- Production uses hardcoded 5-minute TTL
- Cache misses trigger SpiceDB check

## Error Handling

All errors follow consistent patterns for security and debugging.

**Error Categories:**
- `INVALID_INPUT` - Client provided invalid data (400)
- `NOT_FOUND` - Requested resource not found (404)
- `UNAUTHORIZED` - Authentication required (401)
- `FORBIDDEN` - Permission denied (403)
- `CONFLICT` - Operation conflicts with current state (409)
- `UNPROCESSABLE_ENTITY` - Business rule violation (422)
- `DATABASE_ERROR` - Database operation failed (503)
- `SERVICE_UNAVAILABLE` - External service down (503)
- `INTERNAL_ERROR` - Unexpected server error (500)
- `TIMEOUT` - Operation exceeded time limit (504)

**Error Response Format:**
```json
{
  "error": {
    "code": "INVALID_INPUT",
    "message": "Email address is not valid",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z",
    "path": ["createUser", "email"],
    "extensions": {
      "field": "email",
      "constraint": "email_format"
    }
  }
}
```

**Error Handling Rules:**
- MUST map all errors to appropriate HTTP status codes
- MUST include trace_id in all error responses for correlation
- MUST log full error details internally at ERROR level with trace_id
- MUST NEVER expose internal implementation details in error messages
- MUST NEVER panic - all panics must be caught and converted to INTERNAL_ERROR
- SHOULD provide actionable error messages when safe to do so
- MUST implement circuit breakers for external service calls with configurable thresholds

## Health Checks

Two health check endpoints provide different levels of detail.

**`GET /health`**
- Simple liveness check
- Returns 200 OK if server is running
- Used by Docker HEALTHCHECK

**`GET /health/ready`**
- Comprehensive readiness check
- Returns JSON with service statuses
- Checks all critical dependencies
- Used by Kubernetes readiness probe

**Health Status Levels:**
- `healthy` - All systems operational
- `degraded` - Non-critical services down
- `unhealthy` - Critical services down
- `starting` - Initial startup phase

**Service Dependencies & Health Checks:**

*Critical Services (affect readiness):*
- Database (SurrealDB) - MUST be connected and responding
- Authorization (SpiceDB) - SHOULD be available, MAY use cache if down < 5 minutes

*Non-Critical Services (logged but don't affect readiness):*
- Authentication (Kratos/Hydra) - MAY operate in degraded mode
- External webhooks - MAY disable if consistently failing
- Metrics collection - MAY continue without metrics

**Degraded Mode Operation:**
When non-critical services are unavailable:
- MUST log service status at WARN level every 30 seconds
- SHOULD attempt reconnection with exponential backoff
- MAY serve cached data for read operations
- MUST queue write operations up to configured limits
- MUST return appropriate error codes when operations cannot complete

**CLI Mode:**
- `pcf-api healthcheck` runs health check and exits
- Exit code 0 for healthy/degraded
- Exit code 1 for unhealthy
- Exit code 2 for connection failure

## Logs

Structured logging with tracing support for observability.

**Log Levels:**
- `TRACE` - Detailed debugging information
- `DEBUG` - Debugging information (not in production)
- `INFO` - Normal operations
- `WARN` - Warning conditions
- `ERROR` - Error conditions
- `FATAL` - Fatal errors (causes exit)

**Log Structure:**
- ISO8601 timestamp
- Log level
- Module path
- Trace ID (for request correlation)
- Message
- Additional fields as JSON

**Requirements:**
- Every request gets unique trace ID
- Trace ID propagated through all operations
- Sensitive data never logged
- Configurable per-module log levels
- JSON format in production

## Metrics

Prometheus-compatible metrics for monitoring and alerting.

**Request Metrics:**
- `graphql_request_total` - Counter by operation type/name
- `graphql_request_duration_seconds` - Histogram of request duration
- `graphql_field_resolution_duration_seconds` - Per-field performance
- `graphql_active_subscriptions` - Current subscription count

**System Metrics:**
- `http_request_total` - All HTTP requests
- `http_request_duration_seconds` - HTTP request duration
- Process metrics (memory, CPU, file descriptors)

**Database Metrics:**
- `database_connection_pool_size` - Connection pool status
- `database_query_total` - Query counts by operation
- `database_query_duration_seconds` - Query performance

**External Service Metrics:**
- `external_service_health` - Service health status (0/1)
- `external_service_latency_seconds` - Service call latency

**Requirements:**
- Exposed at `/metrics` endpoint
- Global labels for service/environment/instance
- Careful cardinality control
- No high-cardinality labels (like user IDs)

## Testing

Comprehensive testing strategy ensuring reliability and maintainability.

**Test Organization:**
- `src/tests/` directory structure
- Unit tests for isolated components
- Integration tests with containerized dependencies
- End-to-end tests for complete workflows
- Shared test utilities and fixtures

**Testing Patterns:**
- Builder pattern for test data generation
- Property-based testing for edge cases
- Mock services for unit tests
- Containerized services for integration tests
- Random data generation for stress testing

**Coverage Requirements:**
- MUST achieve minimum 80% code coverage for all modules
- MUST achieve 100% coverage for critical paths:
  - Authorization checks (permit/deny logic)
  - Database retry and circuit breaker logic
  - Health check state transitions
  - Error handling and propagation
  - Configuration validation
  - Security validations
- SHOULD achieve 90% coverage for GraphQL resolvers
- MAY exclude from coverage:
  - Simple getter/setter methods
  - Derived trait implementations
  - Code that only runs in demo mode (though it must still be tested)

**Test Quality Requirements:**
- MUST test both success and failure paths
- MUST test edge cases and boundary conditions
- MUST use property-based testing for input validation
- SHOULD use mutation testing to verify test effectiveness
- MUST run tests in CI before allowing merge

**Test Execution:**
- Unit tests run with mocks (fast)
- Integration tests use containers (thorough)
- Parallel test execution where possible
- Performance tests run separately
- All environments containerized

**Critical Path Tests:**
- Authorization flow (permit/deny)
- Database retry logic
- Health check state transitions
- Error handling and propagation
- GraphQL schema compliance

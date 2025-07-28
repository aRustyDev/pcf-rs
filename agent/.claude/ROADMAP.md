# PCF-RS API Development Roadmap

## Overview
This roadmap outlines the development phases for building a production-ready GraphQL API server with comprehensive observability, security, and scalability features. The implementation follows a modular architecture with strict separation of concerns.

**IMPORTANT**: This roadmap is designed so any agent can clearly understand:
- The scope and requirements for each phase
- What "done" looks like for each task
- Dependencies between phases
- Critical vs non-critical paths

**Current Status**: Project has empty main.rs with archived implementation available. Decision needed on restoration vs fresh start.

## Phase 1: Foundation & Core Infrastructure — Priority: CRITICAL
**Goal**: Establish the basic server infrastructure with proper configuration management and health checks.

**Done Criteria**: 
- Server starts successfully with Axum
- Configuration loads from all 4 tiers (defaults → files → env vars → CLI)
- Health check endpoints respond correctly
- Graceful shutdown implemented
- Structured logging with tracing operational

### 1.1 Project Setup & Configuration
- [ ] Initialize Rust project with proper module structure
- [ ] Implement Figment-based configuration management with Garde validation
- [ ] Set up 4-tier configuration hierarchy (defaults → files → env vars → CLI args)
- [ ] Create development and production configuration profiles
- [ ] Implement proper error types and handling patterns

### 1.2 Basic Server Bootstrap
- [ ] Create main.rs with Axum HTTP server setup
- [ ] Implement graceful shutdown handling
- [ ] Add structured logging with tracing (JSON format for production)
- [ ] Set up health check endpoints (/health and /health/ready)
- [ ] Implement server lifecycle management (startup, shutdown, signal handling)

### 1.3 Health Check System
- [ ] Simple liveness endpoint (/health) returning plain text "OK"
- [ ] Comprehensive readiness endpoint (/health/ready) with JSON response
- [ ] Service status tracking (healthy/degraded/unhealthy/starting)
- [ ] Health check caching to prevent overload
- [ ] CLI mode for health checks (pcf-api healthcheck)

## Phase 2: Database Layer & Persistence — Priority: CRITICAL
**Goal**: Establish reliable database connectivity with proper retry logic and connection pooling.

**Done Criteria**:
- SurrealDB connects with infinite retry on failure
- Connection pool healthy and metrics exposed
- All data models properly validated
- CRUD operations tested and working
- Database health check integrated

### 2.1 SurrealDB Integration
- [ ] Database service trait definition
- [ ] SurrealDB adapter implementation
- [ ] Connection pool configuration with proper limits
- [ ] Implement infinite retry with exponential backoff for connections
- [ ] Database health check implementation

### 2.2 Schema & Data Models
- [ ] Define Note type with GraphQL and database derives
- [ ] Implement proper ID handling for SurrealDB Thing type
- [ ] Create schema conversion utilities
- [ ] Add validation rules using Garde

## Phase 3: GraphQL Implementation — Priority: CRITICAL
**Goal**: Build a fully-functional GraphQL API with queries, mutations, and subscriptions.

**Done Criteria**:
- GraphQL playground accessible in demo mode
- All queries, mutations, subscriptions functional
- Security controls enforced (depth, complexity, introspection)
- Error handling returns proper GraphQL errors
- Schema export available in demo mode

### 3.1 GraphQL Schema Setup
- [ ] GraphQL schema builder with async-graphql
- [ ] Request context implementation (auth, dataloaders)
- [ ] Error handling with proper GraphQL error types
- [ ] Schema export endpoint (demo mode only)

### 3.2 Resolvers Implementation
- [ ] Query resolvers (note, notes, notesByAuthor, health)
- [ ] Mutation resolvers (createNote, updateNote, deleteNote)
- [ ] Subscription resolvers (noteCreated, noteUpdated, noteDeleted)
- [ ] Field-level resolver performance tracking

### 3.3 Security Controls
- [ ] Query depth limiting (configurable, max 15 in production)
- [ ] Query complexity limiting with cost analysis
- [ ] Disable introspection in production
- [ ] Disable GraphQL playground/GraphiQL in production

## Phase 4: Authorization & Authentication — Priority: HIGH
**Goal**: Implement secure, performant authorization with SpiceDB integration.

**Done Criteria**:
- All endpoints require authorization (except health checks)
- SpiceDB permission checks working correctly
- Authorization caching reduces load
- Proper 401 (unauthenticated) vs 403 (unauthorized) responses
- Demo mode bypass functional for testing

### 4.1 Authorization Framework
- [ ] Standard is_authorized helper function
- [ ] Authorization result caching (5 min TTL in production)
- [ ] Cache implementation with cleanup strategies
- [ ] Demo mode bypass for testing

### 4.2 SpiceDB Integration
- [ ] SpiceDB client wrapper
- [ ] Permission check implementation
- [ ] Resource formatting standards (type:id)
- [ ] Standard CRUD permissions (read/write/delete/create)
- [ ] SpiceDB health check integration

### 4.3 Session Management
- [ ] Extract authentication from request headers
- [ ] User context propagation through resolvers
- [ ] Proper error responses (401 vs 403)

## Phase 5: Observability & Monitoring — Priority: HIGH
**Goal**: Comprehensive observability with metrics, logs, and distributed tracing.

**Done Criteria**:
- /metrics endpoint returns valid Prometheus format
- All operations emit structured logs with trace IDs
- Distributed tracing spans created for all operations
- No sensitive data in logs
- Monitoring dashboards created

### 5.1 Metrics Implementation
- [ ] Prometheus metrics endpoint (/metrics)
- [ ] GraphQL request metrics (count, duration, errors)
- [ ] HTTP request metrics
- [ ] Database connection pool metrics
- [ ] External service health metrics
- [ ] System metrics (CPU, memory, file descriptors)
- [ ] Business metrics with cardinality control

### 5.2 Structured Logging
- [ ] Log level configuration per module
- [ ] Trace ID propagation for request correlation
- [ ] Security rules (no PII, sanitized queries)
- [ ] Performance considerations (async logging, sampling)
- [ ] Different formatters for dev vs production

### 5.3 Distributed Tracing
- [ ] OpenTelemetry integration
- [ ] Span creation for all operations
- [ ] Trace context propagation
- [ ] Integration with external services

## Phase 6: Performance Optimization — Priority: MEDIUM
**Goal**: Optimize for production workloads with proper caching and connection management.

**Done Criteria**:
- No N+1 queries detected in tests
- P99 response times under 200ms
- Timeouts cascade properly without hanging
- Cache hit rate > 50% for common queries
- Load tests pass at 1000 RPS

### 6.1 DataLoader Implementation
- [ ] N+1 query prevention for all relationships
- [ ] Batch loading with configurable batch sizes
- [ ] Per-request caching
- [ ] Integration with GraphQL context

### 6.2 Response Caching
- [ ] Schema introspection caching
- [ ] User-specific query result caching
- [ ] Cache key strategies with proper isolation
- [ ] TTL configuration

### 6.3 Request Timeouts
- [ ] HTTP server timeout (30s)
- [ ] GraphQL execution timeout (25s)
- [ ] Database query timeout (20s)
- [ ] Proper timeout hierarchy for clean error propagation

## Phase 7: Container & Deployment — Priority: MEDIUM
**Goal**: Production-ready containerization with security best practices.

**Done Criteria**:
- Docker image < 50MB
- Zero security vulnerabilities in scan
- Kubernetes deployment successful
- Health checks work in container
- Secrets properly managed

### 7.1 Docker Implementation
- [ ] Multi-stage Dockerfile with static binary
- [ ] Scratch-based runtime image
- [ ] Proper build caching for dependencies
- [ ] Health check integration
- [ ] Security scanning

### 7.2 Kubernetes Readiness
- [ ] Proper readiness/liveness probes
- [ ] ConfigMap and Secret integration
- [ ] Horizontal pod autoscaling metrics
- [ ] Resource limits and requests

### 7.3 Secret Management
- [ ] Environment variable injection
- [ ] Kubernetes Secrets support
- [ ] HashiCorp Vault integration (optional)
- [ ] No secrets in images or logs

## Phase 8: Testing Strategy — Priority: HIGH
**Goal**: Comprehensive test coverage ensuring reliability and maintainability.

**Done Criteria**:
- Overall test coverage > 80%
- Critical path coverage = 100%
- All test types implemented (unit, integration, E2E)
- CI/CD pipeline all green
- Performance tests establish baselines

### 8.1 Test Infrastructure
- [ ] Unit test framework setup
- [ ] Integration test with testcontainers
- [ ] E2E test framework
- [ ] Test data builders and factories
- [ ] Mock service implementations

### 8.2 Critical Path Testing
- [ ] Authorization flow tests (100% coverage required)
- [ ] Database retry logic tests (100% coverage required)
- [ ] Health check state transition tests (100% coverage required)
- [ ] Error handling and propagation tests
- [ ] GraphQL schema compliance tests

### 8.3 Performance Testing
- [ ] Load testing framework
- [ ] Concurrent request handling tests
- [ ] Memory leak detection
- [ ] Latency percentile tracking

## Phase 9: Production Features — Priority: LOW
**Goal**: Additional features for production operations and future scalability.

**Done Criteria**:
- Advanced authorization patterns working
- Circuit breakers prevent cascade failures
- SLO/SLI dashboards operational
- Error budgets tracked
- Ready for multi-region deployment

### 9.1 Advanced Authorization
- [ ] Batch authorization checks
- [ ] Permission hints in responses
- [ ] Role-based access patterns
- [ ] Audit logging for authorization decisions

### 9.2 Microservice Integration
- [ ] Service discovery
- [ ] Circuit breaker implementation
- [ ] Retry strategies for external services
- [ ] Distributed transaction patterns

### 9.3 Advanced Monitoring
- [ ] Custom Grafana dashboards
- [ ] Alerting rules for Prometheus
- [ ] SLO/SLI definitions
- [ ] Error budget tracking

## Implementation Guidelines

### Development Principles
1. **Test-Driven Development**: Write tests before implementation
2. **Security First**: Never expose internal details, always sanitize
3. **Performance Aware**: Consider latency and resource usage
4. **Observable by Default**: Instrument everything
5. **Fail Gracefully**: Never panic, always recover

### Module Dependencies
- Each phase builds on previous phases
- Critical priority items block all subsequent work
- High priority items should be completed before medium/low
- Parallel work possible within same phase

### Success Criteria
- 80% overall test coverage (100% for critical paths)
- All security controls implemented and tested
- Performance meets defined SLOs
- Zero panics in production code
- Comprehensive observability coverage

## Work Estimation by Phase

The following estimates use "units of work" where 1 unit represents a small, well-defined task that can be completed independently. These estimates help understand the relative complexity and effort required for each phase.

### Phase Complexity Overview
- **Phase 1 (Foundation)**: 8-12 units - Establishing core infrastructure and configuration
- **Phase 2 (Database)**: 5-7 units - Database integration with retry logic
- **Phase 3 (GraphQL)**: 10-15 units - Full GraphQL implementation with security
- **Phase 4 (Authorization)**: 10-12 units - SpiceDB integration and caching
- **Phase 5 (Observability)**: 8-10 units - Metrics, logging, and tracing
- **Phase 6 (Performance)**: 5-7 units - Optimization and caching strategies
- **Phase 7 (Containerization)**: 4-6 units - Docker and Kubernetes setup
- **Phase 8 (Testing)**: 15-20 units - Comprehensive test coverage (ongoing)
- **Phase 9 (Production)**: 10-15 units - Advanced features and hardening

**Total Work Units**: 75-104 units

### Work Distribution
- **Critical Priority**: ~40 units (Phases 1-3)
- **High Priority**: ~35 units (Phases 4-5, 8)
- **Medium Priority**: ~15 units (Phases 6-7)
- **Low Priority**: ~15 units (Phase 9)

### Complexity Factors
- **High Complexity**: Authorization system, GraphQL subscriptions, testing infrastructure
- **Medium Complexity**: Database retry logic, observability, performance optimization
- **Low Complexity**: Configuration, health checks, containerization

## Key Implementation Notes

### From Archived Implementation
The project has a complete implementation archived at `.archive/` that includes:
- Working GraphQL server with SurrealDB
- ORY Kratos authentication integration
- SpiceDB authorization
- Webhook handlers
- Complete configuration management

**Decision Required**: Either restore from archive or implement fresh following this roadmap.

### Critical Requirements (from SPEC.md)
1. **Server MUST never exit**: Implement infinite retry for all external connections
2. **Observable by default**: Every operation must have metrics and traces
3. **Security first**: Never expose internal details, validate all inputs
4. **Demo mode**: Feature flag for easier testing without auth
5. **Module boundaries**: Strict separation between layers

### Configuration Hierarchy (from CLAUDE.local.md)
1. Default values (in code)
2. Configuration files (TOML/JSON/YAML)
3. Environment variables
4. CLI arguments

### External Service Dependencies
- **SurrealDB**: Primary database (infinite retry required)
- **SpiceDB**: Authorization decisions (cache results)
- **ORY Hydra**: OAuth2/OIDC (optional in demo mode)
- **ORY Kratos**: Identity management (optional in demo mode)

## Next Steps
1. Review and approve roadmap
2. Decide on archive restoration vs fresh implementation
3. Set up project repository and CI/CD
4. Begin Phase 1 implementation
5. Establish code review process
6. Define specific SLOs and performance targets

---
*This roadmap is a living document. Update as implementation progresses and new requirements emerge.*
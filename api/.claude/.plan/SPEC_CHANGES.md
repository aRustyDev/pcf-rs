# Specification Changes Plan

This document synthesizes the escape hatches and flexibility analyses to provide balanced, implementable specification improvements that prevent agent escape while avoiding implementation deadlock.

## Core Principles

1. **Security and data integrity use MUST** - No flexibility on these
2. **Performance and operations use SHOULD** - With documented exceptions  
3. **Development conveniences use MAY** - Optional optimizations
4. **All requirements must be testable** - If you can't test it, don't require it
5. **Fail gracefully, recover automatically** - Never leave the system broken

## Language Standards

### RFC 2119 Compliance
Use these keywords consistently:
- **MUST/MUST NOT** - Absolute requirements (security, data integrity)
- **SHOULD/SHOULD NOT** - Strong recommendations with escape clauses
- **MAY/MAY NOT** - Optional features
- **RECOMMENDED** - Best practice but not required

### Escape Clause Templates
When using SHOULD, always provide escape clause:
- "SHOULD do X, unless Y prevents it, in which case MUST do Z"
- "SHOULD complete within Xs, MAY extend to Ys under high load"
- "SHOULD use method A, MAY use method B if platform doesn't support A"

## Specification Changes by Category

### 1. Configuration Management

**Current Problem**: Rigid paths and ports cause environment conflicts

**Required Changes**:
```markdown
## Configuration Requirements

The service MUST load configuration using Figment with this precedence:
1. Default values (hardcoded)
2. Configuration files (searched in order):
   - ./config/default.toml
   - ./config/{environment}.toml
   - /etc/pcf/config.toml (production only)
3. Environment variables (PCF_ prefix)
4. Command line arguments

Connection endpoints:
- MUST accept configuration for all service endpoints
- SHOULD use these defaults:
  - SurrealDB: localhost:8000 (dev), surrealdb:8000 (container)
  - SpiceDB: localhost:50051 (dev), spicedb:50051 (container)
- MUST validate endpoints are reachable within 30s of startup
- If unreachable: MUST retry with exponential backoff (max 32s between attempts)

Port binding:
- SHOULD bind to configured port (default: 8080)
- If port unavailable:
  - Development: MAY find next available port, MUST log actual port
  - Production: MUST exit with clear error message
```

### 2. Error Handling and Recovery

**Current Problem**: Incomplete error specifications lead to panics

**Required Changes**:
```markdown
## Error Handling Requirements

### Panic Prevention
The service MUST NOT panic in production code. Specifically:
- MUST use `?` operator or explicit error handling for all Results
- MUST NOT use `unwrap()` or `expect()` outside of:
  - Test code
  - Initialization of compile-time constants
  - Cases with documented infallibility proof
- MUST catch panics at thread boundaries using std::panic::catch_unwind

### Error Categories and Responses
MUST classify errors into these categories with specific handling:

| Category | Examples | HTTP Status | Retry | User Message |
|----------|----------|-------------|-------|--------------|
| Client Error | Validation, not found | 4xx | No | Specific error |
| Transient | Network timeout, lock conflict | 503 | Yes | "Temporarily unavailable" |
| Fatal | Data corruption, OOM | 500 | No | "Internal error" + trace ID |

### Graceful Degradation
When external services fail, MUST follow this precedence:
1. Use cached data if available and fresh (<5 min for auth, <1 min for data)
2. Return partial results with degraded status in response header
3. Queue writes for retry (with timeout and overflow handling)
4. Return 503 with Retry-After header if cannot degrade gracefully
```

### 3. Service Dependencies

**Current Problem**: Rigid service requirements cause cascading failures

**Required Changes**:
```markdown
## External Service Management

### Connection Lifecycle
For each external service (SurrealDB, SpiceDB, Kratos), MUST:

1. **Startup Phase**:
   - MUST attempt connection with 5s timeout
   - If fails: MUST retry with exponential backoff (1s, 2s, 4s...max 32s)
   - Critical services (SurrealDB): MUST NOT start without connection
   - Optional services (metrics): MAY start in degraded mode

2. **Runtime Phase**:
   - MUST use connection pooling with these limits:
     - Min connections: 2
     - Max connections: lesser of (100, available_memory_mb/10)
   - MUST implement circuit breakers:
     - Open after 5 consecutive failures or 50% failure rate in 10 requests
     - Half-open after 30s, full-open after 3 successful probes
   - SHOULD maintain per-service health metrics

3. **Shutdown Phase**:
   - MUST stop accepting new requests
   - SHOULD complete in-flight requests (max 30s)
   - MUST drain connection pools gracefully
   - MUST log shutdown progress

### Service-Specific Requirements

**SurrealDB** (Critical):
- MUST NOT start without successful connection
- MUST queue writes during brief outages (<30s)
- MUST NOT lose write data (persist queue to disk if needed)

**SpiceDB** (Critical for auth):
- MUST cache permission checks (5 min TTL)
- MAY extend cache to 15 min during outages for existing sessions
- MUST deny by default for new sessions during outage

**Kratos/Hydra** (Optional in demo mode):
- In demo mode: MUST bypass with warning headers
- In production: MUST enforce authentication
- MAY cache session validation (5 min)
```

### 4. Performance Requirements

**Current Problem**: Fixed limits don't account for system resources

**Required Changes**:
```markdown
## Performance and Resource Management

### Adaptive Limits
MUST implement adaptive resource management:

1. **Connection Pools**:
   - Base size: min(100, available_memory_mb / 10)
   - MUST monitor pool saturation
   - MAY increase by 10% if wait time > 100ms (up to hard limit)
   - MUST decrease by 10% if idle > 50% for 5 minutes

2. **Request Timeouts**:
   - Base: HTTP=30s, GraphQL=25s, Database=20s
   - Health checks: Always 5s (no adaptation)
   - Under load (CPU > 80% or memory > 90%):
     - MAY extend timeouts by 50%
     - MUST reject new requests if queue depth > 1000
     - MUST return 503 with Retry-After header

3. **Query Complexity**:
   - Default limit: 1000 points
   - MUST calculate for all GraphQL queries
   - For authenticated admins: MAY allow up to 5000
   - MUST log all queries exceeding 80% of limit
   - SHOULD suggest query optimizations in error

### Performance Targets
SHOULD meet these targets under normal load (< 70% CPU):
- P50 latency: < 50ms
- P95 latency: < 200ms  
- P99 latency: < 500ms

If targets not met:
- MUST log degraded performance
- SHOULD auto-scale if supported by platform
- MAY shed load using adaptive concurrency limits
```

### 5. Security Requirements

**Current Problem**: Some security requirements made optional

**Required Changes**:
```markdown
## Security Requirements (Non-Negotiable)

These security requirements MUST be enforced without exception:

### Build-Time Security
\`\`\`rust
// MUST include in lib.rs or main.rs
#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");

#[cfg(all(not(debug_assertions), feature = "introspection"))]
compile_error!("GraphQL introspection MUST NOT be enabled in release builds");
\`\`\`

### Runtime Security
1. **Authentication**: 
   - MUST authenticate all endpoints except /health and /metrics
   - MUST validate JWT signatures with proper algorithms (RS256/ES256)
   - MUST NOT accept none algorithm

2. **Authorization**:
   - MUST check permissions for every resource access
   - MUST use deny-by-default
   - MUST log all authorization decisions in audit mode

3. **Input Validation**:
   - MUST validate all inputs with Garde
   - MUST reject requests exceeding size limits (body: 10MB, GraphQL: 50KB)
   - MUST sanitize all log output (no PII, no credentials)

4. **Error Handling**:
   - MUST NOT expose internal details in production
   - MUST include trace ID for correlation
   - MAY expose details with special debug header + admin role

### Demo Mode Security
When demo mode is enabled:
- MUST add "X-Demo-Mode: true" to all responses
- MUST log all bypassed auth checks
- MUST reject any writes to production namespaces
- MUST show warning banner in GraphQL playground
```

### 6. Testing Requirements

**Current Problem**: Untestable requirements and vague coverage goals

**Required Changes**:
```markdown
## Testing Requirements

### Coverage Requirements
MUST achieve these coverage levels:
- Overall: 80% line coverage minimum
- Critical paths: 100% coverage required for:
  - src/auth/* (all authorization code)
  - src/health/* (health check logic)  
  - src/db/retry.rs (retry logic)
  - Error handling in all request paths

### Test Categories
MUST implement these test types:

1. **Unit Tests**:
   - Every public function with business logic
   - Edge cases for all validation
   - Error paths for all external calls

2. **Integration Tests**:
   - Full request/response cycles
   - Database operations with testcontainers
   - Service degradation scenarios

3. **Chaos Tests** (CI only):
   - Network partition simulation
   - Service crash/restart
   - Resource exhaustion
   - Clock skew

### Testability Requirements
Every requirement MUST be testable:
- If specifying a timeout: MUST be testable with time mocking
- If specifying a limit: MUST be testable at boundary conditions
- If specifying degradation: MUST be testable with service mocks
```

### 7. Operational Requirements

**Current Problem**: Missing operational procedures

**Required Changes**:
```markdown
## Operational Requirements

### Deployment
MUST support these deployment patterns:
- Blue-green deployments with health-based cutover
- Rolling updates with connection draining
- Canary deployments with metric-based rollback

### Observability
MUST expose:
1. **Metrics** (Prometheus format):
   - All default HTTP/GraphQL metrics
   - Business metrics with cardinality limits
   - Service health scores (0-100)

2. **Logging** (structured JSON):
   - Request ID propagation
   - User ID (hashed) for audit trail
   - Error sampling to prevent log floods

3. **Tracing** (OpenTelemetry):
   - All service boundaries
   - Database queries > 100ms
   - External API calls

### Emergency Controls
MUST implement operator escape hatches:
- /admin/circuit-breaker/{service}/open - Force open circuit
- /admin/cache/clear - Clear all caches
- /admin/connections/drain - Graceful connection drain
- Environment variable overrides for all limits
```

## Implementation Phases

### Phase 1: Core Requirements (Weeks 1-2)
Focus on MUST requirements only:
- Basic server with health checks
- Database connections with retry
- Configuration loading
- Structured error handling

### Phase 2: Security Layer (Weeks 3-4)
Non-negotiable security:
- Authentication/authorization
- Input validation
- Audit logging
- Build-time checks

### Phase 3: Resilience (Weeks 5-6)
Degradation and recovery:
- Circuit breakers
- Caching layers
- Queue management
- Timeout hierarchies

### Phase 4: Performance (Week 7)
Optimization and limits:
- Connection pooling
- Query complexity
- Adaptive limits
- Load shedding

### Phase 5: Operations (Week 8)
Production readiness:
- Metrics/logging/tracing
- Deployment patterns
- Emergency controls
- Documentation

## Validation Checklist

Before marking any phase complete, MUST pass:

```bash
#!/bin/bash
# Automated validation script

# 1. Security checks
cargo audit --deny-warnings
cargo +nightly clippy -- -D warnings

# 2. Test coverage
cargo tarpaulin --out Xml --exclude-files "*/tests/*" "*/benches/*"
coverage=$(xmllint --xpath "string(//coverage/@line-rate)" cobertura.xml)
if (( $(echo "$coverage < 0.80" | bc -l) )); then
  echo "FAIL: Coverage $coverage below 80%"
  exit 1
fi

# 3. Critical path coverage
for critical in "src/auth" "src/health" "src/db/retry.rs"; do
  if ! grep -q "100.0%" <(cargo tarpaulin --print-summary --packages pcf-api --include "$critical"); then
    echo "FAIL: Critical path $critical not 100% covered"
    exit 1
  fi
done

# 4. Integration tests
docker-compose -f tests/docker-compose.yml up -d
cargo test --features integration-tests
docker-compose -f tests/docker-compose.yml down

# 5. Chaos tests (optional in CI)
if [[ "$CI" == "true" ]]; then
  cargo test --features chaos-tests
fi

echo "PASS: All validations successful"
```

## Conclusion

These specification changes provide:
1. **Clear requirements** without ambiguity
2. **Escape clauses** for real-world situations
3. **Testable criteria** for validation
4. **Progressive implementation** path
5. **Operational flexibility** without compromising security

An agent following these specifications will have:
- No excuses to skip requirements (clear MUST items)
- No reason to panic (graceful degradation paths)
- Clear success criteria (measurable outcomes)
- Practical implementation guidance (phased approach)
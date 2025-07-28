# Flexibility Analysis: Where Specifications Need Conditional Language

This document identifies areas where overly rigid specifications could cause implementation deadlock and proposes balanced language that maintains requirements while allowing for real-world complexity.

## Critical Areas Requiring Flexibility

### 1. External Service Dependencies

**Problem**: Rigid requirements assume external services always behave perfectly
**Current**: "MUST connect to SurrealDB at localhost:8000"
**Reality**: Services can be slow to start, on different hosts, or temporarily unavailable

**Proposed Language**:
```markdown
The service MUST attempt to connect to SurrealDB at the configured endpoint. 
If connection fails:
- During startup: MUST retry with exponential backoff (1s, 2s, 4s, 8s...) indefinitely
- During operation: SHOULD use cached data for reads if available, MUST queue writes for retry
- If unavailable > 30s: MUST return 503 Service Unavailable with appropriate retry-after header
```

### 2. Resource Constraints

**Problem**: Fixed requirements don't account for environment limitations
**Current**: "MUST maintain connection pool of 100 connections"
**Reality**: Container might have memory limits, database might have connection limits

**Proposed Language**:
```markdown
The service SHOULD maintain a connection pool sized according to:
- Default: min(100, available_memory_mb / 10)
- MUST NOT exceed database max_connections - 10 (reserve for admin)
- MUST emit warning if pool size < 20
- MAY dynamically adjust based on connection wait times
```

### 3. Authentication Service Availability

**Problem**: Hard dependency on Kratos/Hydra creates single point of failure
**Current**: "MUST authenticate all requests through Kratos"
**Reality**: Auth services can have outages, network partitions occur

**Proposed Language**:
```markdown
Authentication MUST follow this precedence:
1. Check cached session (if < 5 minutes old)
2. Validate with Kratos (timeout: 3s)
3. If Kratos unavailable:
   - For existing sessions: MAY extend cache to 15 minutes
   - For new sessions: MUST reject with 503
   - MUST log degraded state
4. MUST re-validate cached sessions when Kratos recovers
```

### 4. Port Binding

**Problem**: Fixed ports may conflict with existing services
**Current**: "MUST bind to port 8080"
**Reality**: Port might be in use, especially in development

**Proposed Language**:
```markdown
Service port binding:
- SHOULD use configured port (default: 8080)
- If port unavailable: 
  - In development: MAY find next available port and log it
  - In production: MUST fail fast with clear error
- MUST expose actual bound port in health check response
```

### 5. Database Schema Migrations

**Problem**: No guidance on handling schema mismatches
**Current**: Silent assumption that schema is always correct
**Reality**: Migrations fail, versions mismatch, rollbacks needed

**Proposed Language**:
```markdown
Schema management:
- MUST check schema version on startup
- If behind: SHOULD auto-migrate unless DISABLE_AUTO_MIGRATE=true
- If ahead: MUST refuse to start (prevent data corruption)
- Migration failures: MUST rollback and exit with status 78 (config error)
- SHOULD support dry-run mode for migration validation
```

### 6. GraphQL Complexity Limits

**Problem**: Hard limits might block legitimate queries
**Current**: "MUST reject queries with complexity > 1000"
**Reality**: Some valid queries need higher complexity

**Proposed Language**:
```markdown
Query complexity handling:
- MUST calculate complexity for all queries
- Default limit: 1000 points
- If exceeded:
  - For authenticated admin users: MAY allow up to 5000
  - For monitoring queries: MAY allowlist specific queries
  - MUST log all over-limit queries
  - SHOULD suggest query optimization in error message
```

### 7. Timeout Hierarchies

**Problem**: Fixed timeouts don't account for system load
**Current**: "HTTP timeout MUST be 30s"
**Reality**: Under load, processing might legitimately take longer

**Proposed Language**:
```markdown
Timeout configuration:
- Base timeouts: HTTP=30s, GraphQL=25s, Database=20s
- Under high load (>80% CPU):
  - MAY extend timeouts by 50%
  - MUST warn in logs
  - MUST track in metrics
- For health checks: MUST use fixed 5s timeout
- For batch operations: MAY use timeout = base * batch_size / 100
```

### 8. Error Response Details

**Problem**: Rigid security requirements might hinder debugging
**Current**: "MUST NOT expose internal errors"
**Reality**: Developers need debugging information

**Proposed Language**:
```markdown
Error responses:
- In production: MUST return generic messages with trace ID
- In development: SHOULD include stack traces and detailed errors
- For authenticated developers (special header/flag):
  - MAY include detailed errors even in production
  - MUST log access to detailed errors
- GraphQL errors: SHOULD include field path but NOT database details
```

### 9. Metrics Cardinality

**Problem**: Hard cardinality limits might lose important data
**Current**: "MUST limit cardinality to 1000"
**Reality**: Legitimate use cases might need more labels

**Proposed Language**:
```markdown
Metrics cardinality management:
- SHOULD maintain cardinality below 1000 per metric
- If approaching limit:
  - MUST use bucketing for numeric labels
  - SHOULD aggregate low-frequency labels as "other"
  - MAY drop labels in order: user_id, request_id, session_id
- MUST emit meta-metric for dropped labels
- Critical metrics (errors, security): MAY exceed limit temporarily
```

### 10. Circuit Breaker Behavior

**Problem**: Missing guidance on service degradation
**Current**: No circuit breaker specification
**Reality**: Cascading failures need automatic prevention

**Proposed Language**:
```markdown
Circuit breaker implementation:
- MUST track failure rate per external service
- If failure rate > 50% in 10 requests:
  - SHOULD open circuit for 30s
  - MAY serve cached data if available
  - MUST return 503 with Retry-After header
- During open circuit:
  - MUST attempt single probe request every 10s
  - SHOULD close circuit after 3 successful probes
- MUST emit metrics for circuit state changes
```

## Platform-Specific Flexibility

### Development Environment

```markdown
In development mode (NODE_ENV=development or debug builds):
- MAY relax security constraints for easier debugging
- SHOULD auto-reload on config changes
- MAY expose additional debugging endpoints
- SHOULD use more descriptive error messages
- MAY skip some performance optimizations
```

### Container Environments

```markdown
When running in containers (detected via /.dockerenv):
- MUST use environment variables over config files
- SHOULD detect memory limits from cgroups
- MAY adjust worker counts based on CPU limits
- MUST handle SIGTERM gracefully with 30s shutdown window
```

### Cloud Environments

```markdown
When running in cloud (detected via instance metadata):
- SHOULD use cloud-native service discovery
- MAY integrate with cloud secret managers
- SHOULD use structured logging for cloud log aggregation
- MUST handle spot instance interruption signals
```

## Conditional Language Guidelines

Use this precedence for modal verbs:

1. **MUST/MUST NOT**: Safety and security requirements only
2. **SHOULD/SHOULD NOT**: Strong recommendations with documented exceptions
3. **MAY/MAY NOT**: Optional features or optimizations

Add escape clauses where appropriate:
- "unless configured otherwise"
- "if supported by the platform"
- "when resources permit"
- "with appropriate backpressure"

## Anti-Patterns to Avoid

1. **Don't make graceful degradation optional** - It should be required
2. **Don't ignore platform differences** - Acknowledge them explicitly
3. **Don't assume infinite resources** - Always have limits
4. **Don't require specific timing** - Use "within X seconds" not "at X seconds"
5. **Don't block on external services** - Always have timeouts

## Conclusion

The specifications need a balance between:
- **Rigid requirements** for security, data integrity, and core functionality
- **Flexible guidance** for performance, external dependencies, and platform-specific behavior

This flexibility prevents agents from getting stuck while still maintaining system quality and security.
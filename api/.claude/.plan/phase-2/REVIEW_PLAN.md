# Phase 2 Review Plan - Database Layer & Persistence

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 2 checkpoints. Phase 2 implements the database layer with SurrealDB, including connection management, retry logic, write queues, and comprehensive metrics collection.

## Review Context

Phase 2 builds upon the foundation established in Phase 1, adding critical database functionality. Reviews must verify not only the new functionality but also proper integration with existing systems (health checks, configuration, error handling).

## Core Review Principles

### Test-Driven Development (TDD) Verification
At every checkpoint, MUST verify TDD practices by checking:
1. **Database trait tests exist before implementation**
2. **Connection tests precede pool implementation**
3. **Retry logic tests written before retry code**
4. **Write queue tests before queue implementation**
5. **All edge cases covered** - network failures, timeouts, full queues

### Documentation Standards
Database code MUST have comprehensive documentation including:
1. **Connection lifecycle documented** - creation, pooling, cleanup
2. **Retry behavior clearly explained** - backoff sequence, jitter
3. **Configuration options documented** - with defaults and limits
4. **Error scenarios documented** - what triggers each error
5. **Performance implications noted** - connection limits, queue sizes

### Code Quality Requirements
1. **NO .unwrap() or .expect() in production code paths** - test code and compile-time constants MAY use these with justification
2. **All futures properly handled** - no forgotten awaits
3. **Resource cleanup guaranteed** - connections, locks, queues
4. **Metrics properly scoped** - avoid high cardinality
5. **Feature flags correctly gated** - metrics levels work as designed

## Review Process Flow

1. **Receive checkpoint artifacts**:
   - Checkpoint number and description
   - All code files created/modified
   - Test output showing TDD approach
   - Any questions in `api/.claude/.reviews/checkpoint-X-questions.md`

2. **Execute review checklist** - MUST complete ALL items before proceeding

3. **Test the implementation** using provided test commands

4. **Document findings** in `api/.claude/.reviews/checkpoint-X-feedback.md`

5. **Provide clear decision**:
   - APPROVED: All requirements met
   - APPROVED WITH CONDITIONS: Minor issues that can be fixed in parallel
   - CHANGES REQUIRED: Critical issues blocking progress

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: Database Architecture & Service Trait

**What You're Reviewing**: Core database abstractions and trait definitions

**Key Specifications to Verify**:
- Database trait covers all CRUD operations
- Error types properly categorized
- SurrealDB version checking implemented
- Mock implementation for testing

**Required Tests** (MUST execute all and verify output):
```bash
# Test trait compilation and mock implementation
cargo test services::database::tests --lib

# Verify error mapping
cargo test database_errors --lib

# Check version compatibility logic
SURREALDB_VERSION="1.5.0" cargo test version_check
```

**Critical Code Reviews**:
- Verify async trait methods properly defined
- Check Thing ID handling for SurrealDB
- Ensure generic trait allows future database switches
- Validate error conversion implementations

**Review Checklist**:
```markdown
## Checkpoint 1 Review - Database Architecture

### Trait Design
- [ ] DatabaseService trait is generic and complete
- [ ] All CRUD operations present with proper signatures
- [ ] Health check method returns detailed status
- [ ] Version compatibility checking implemented

### Error Handling
- [ ] DatabaseError enum covers all scenarios
- [ ] Proper From implementations for conversions
- [ ] No internal details leaked in Display
- [ ] HTTP status codes correctly mapped

### Version Management
- [ ] Configurable SurrealDB version with defaults
- [ ] Version mismatch produces clear warnings
- [ ] Compatibility matrix documented
- [ ] Version check happens on startup

### TDD Verification
- [ ] Mock tests written before trait implementation
- [ ] Error scenario tests comprehensive
- [ ] Version compatibility tests present
- [ ] Test structure follows Phase 1 patterns

### Documentation
- [ ] Trait methods fully documented
- [ ] Examples provided for common operations
- [ ] Version compatibility table included
- [ ] Migration notes for future databases

### Issues Found
[List any issues with severity and required fixes]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 2: Connection Management & Retry Logic

**What You're Reviewing**: Connection pooling and retry patterns

**Key Specifications to Verify**:
- Connection pool with configurable sizing
- Exponential backoff with jitter (1s → 60s max)
- Infinite retry during startup
- Health monitoring for connections

**Required Tests** (MUST execute all and verify output):
```bash
# Test connection pool operations
cargo test connection_pool --features test-containers

# Verify retry logic with network failures
cargo test retry_patterns -- --nocapture

# Test pool sizing configurations
POOL_MIN=5 POOL_MAX=20 cargo test pool_config

# Stress test connection management
cargo test pool_stress_test --release
```

**Performance Tests**:
```bash
# Benchmark connection acquisition
cargo bench connection_pool

# Test pool under load
./scripts/load-test-connections.sh
```

**Review Checklist**:
```markdown
## Checkpoint 2 Review - Connection Management

### Connection Pool Implementation
- [ ] Configurable min/max connections
- [ ] Proper connection lifecycle (create/health/close)
- [ ] Idle timeout removes excess connections
- [ ] Max lifetime enforced
- [ ] Acquire timeout configurable

### Retry Logic
- [ ] Exponential backoff sequence approximately follows pattern (1,2,4,8,16,32,60) with ±20% tolerance for jitter
- [ ] Jitter properly applied
- [ ] Retry for startup with configurable max duration (STARTUP_MAX_WAIT, default: 10 minutes)
- [ ] Timeout-based retry for operations
- [ ] Clear logging of retry attempts

### Health Monitoring
- [ ] Periodic health checks on idle connections
- [ ] Unhealthy connections removed
- [ ] Pool maintains minimum connections
- [ ] Metrics track pool health

### Resource Management
- [ ] No connection leaks under normal operation (documented edge cases acceptable)
- [ ] Proper cleanup on pool shutdown
- [ ] Semaphore correctly limits total connections
- [ ] Async returns handled properly

### Configuration
- [ ] Pool size based on deployment profile
- [ ] Environment variable overrides work
- [ ] Reasonable defaults for all settings
- [ ] Configuration validation present

### TDD Verification
- [ ] Connection failure tests written first
- [ ] Pool exhaustion tests present
- [ ] Health check failure scenarios tested
- [ ] Concurrent access tests implemented

### Issues Found
[Detailed findings with remediation steps]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: Data Models & Validation

**What You're Reviewing**: Data structures and validation logic

**Key Specifications to Verify**:
- Models use Garde validation
- Thing IDs properly handled
- Serialization/deserialization correct
- Type safety throughout

**Required Tests** (MUST execute all and verify output):
```bash
# Test model validation
cargo test models::validation

# Verify serialization round-trips
cargo test serialization_tests

# Test Thing ID handling
cargo test thing_id_operations

# Validation edge cases
cargo test validation_edge_cases
```

**Security Validation**:
- No SQL injection possible
- Input length limits enforced
- Special characters properly handled
- Thing ID format strictly validated

**Review Checklist**:
```markdown
## Checkpoint 3 Review - Data Models

### Model Structure
- [ ] All models derive necessary traits
- [ ] Thing ID wrapper type implemented
- [ ] Proper type conversions defined
- [ ] Optional fields handled correctly

### Validation Rules
- [ ] Garde validation on all user inputs
- [ ] Length limits reasonable and documented
- [ ] Format validation for special fields
- [ ] Custom validators where needed

### Serialization
- [ ] JSON serialization/deserialization works
- [ ] Thing IDs serialize to expected format
- [ ] Null handling explicit
- [ ] Error messages don't leak internals

### Type Safety
- [ ] No stringly-typed APIs
- [ ] Enums used for finite sets
- [ ] NewType pattern for IDs
- [ ] Phantom types where appropriate

### Security
- [ ] Input validation prevents injection
- [ ] Size limits prevent DoS
- [ ] No sensitive data in Debug output
- [ ] Validation errors are safe to expose

### TDD Verification
- [ ] Validation tests cover all rules
- [ ] Serialization tests round-trip
- [ ] Edge case tests comprehensive
- [ ] Security tests present

### Issues Found
[List with security impact assessment]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 4: Write Queue & Health Integration

**What You're Reviewing**: Write queue implementation and health check integration

**Key Specifications to Verify**:
- Write queue with configurable limit (default 1000)
- Persistence with format options (JSON/Bincode/MessagePack/CBOR)
- Service returns 503 when database unavailable > 30s
- Health checks reflect database status

**Required Tests** (MUST execute all and verify output):
```bash
# Test write queue operations
cargo test write_queue

# Test persistence formats
cargo test queue_persistence -- --nocapture

# Test 503 response behavior
cargo test service_unavailable_response

# Test health check integration
cargo test health_with_database

# Queue overflow behavior
cargo test queue_limits
```

**Persistence Format Tests**:
```bash
# Benchmark different formats
cargo bench persistence_formats

# Test format migration
./scripts/test-queue-migration.sh
```

**Review Checklist**:
```markdown
## Checkpoint 4 Review - Write Queue & Health

### Write Queue Implementation
- [ ] Queue respects size limits
- [ ] FIFO ordering maintained
- [ ] Retry logic for failed writes
- [ ] Queue metrics exposed
- [ ] Memory usage bounded

### Persistence Layer
- [ ] All formats work (JSON/Bincode/MessagePack/CBOR)
- [ ] Format configurable via environment
- [ ] Persistence survives restarts
- [ ] Migration path documented
- [ ] Performance acceptable for each format

### Service Availability
- [ ] 503 returned after 30s database unavailability
- [ ] Retry-After header included
- [ ] Graceful degradation for reads
- [ ] Clear error messages
- [ ] Recovery properly handled

### Health Integration
- [ ] /health/ready reflects database status
- [ ] Status transitions logged
- [ ] Startup state handled correctly
- [ ] Degraded mode when queue filling
- [ ] Metrics show queue depth

### Error Scenarios
- [ ] Queue full handling correct
- [ ] Persistence failures handled
- [ ] Database recovery detected
- [ ] No data loss scenarios
- [ ] Circuit breaker integration

### TDD Verification
- [ ] Queue overflow tests written first
- [ ] Persistence failure tests present
- [ ] Health state transition tests
- [ ] Recovery scenario tests

### Issues Found
[Include data integrity concerns]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 5: Complete Integration & Metrics

**What You're Reviewing**: Full database integration and metrics collection

**Key Specifications to Verify**:
- All database operations properly integrated
- Metrics with feature flags (basic/detailed/all)
- No high cardinality labels
- Full system working end-to-end

**Required Tests** (MUST execute all and verify output):
```bash
# Run Phase 2 verification script
./scripts/verify-phase-2.sh

# Test all metric levels
cargo test --features metrics-basic
cargo test --features metrics-detailed  
cargo test --features metrics-all

# Integration tests with real SurrealDB
just surrealdb-up
cargo test --features integration-tests
just surrealdb-down

# Load test the complete system
./scripts/load-test-phase-2.sh
```

**Metrics Validation**:
```bash
# Check metrics endpoint
curl http://localhost:8080/metrics | grep database_

# Verify cardinality limits
./scripts/check-metric-cardinality.sh

# Test metric feature flags
cargo build --features metrics-basic
cargo build --features metrics-detailed
cargo build --features metrics-all
```

**Review Checklist**:
```markdown
## Checkpoint 5 Review - Complete Integration

### System Integration
- [ ] Database integrated with Phase 1 server
- [ ] Configuration properly loaded
- [ ] Health checks fully integrated
- [ ] Graceful shutdown handles database
- [ ] All endpoints use database

### Metrics Implementation
- [ ] Basic metrics: connection pool, query counts
- [ ] Detailed metrics: query duration, queue depth
- [ ] All metrics: field-level timing, cache stats
- [ ] Feature flags work correctly
- [ ] No high cardinality issues

### Performance
- [ ] Connection pool sized appropriately
- [ ] Query timeouts reasonable
- [ ] Write queue performs well
- [ ] No memory leaks
- [ ] CPU usage acceptable

### Operational Readiness
- [ ] Logs helpful for debugging
- [ ] Metrics enable monitoring
- [ ] Error messages actionable
- [ ] Configuration documented
- [ ] Runbook updated

### End-to-End Testing
- [ ] CRUD operations work
- [ ] Retry logic functions correctly
- [ ] Queue processes writes
- [ ] Health checks accurate
- [ ] Metrics collected properly

### TDD Summary
- [ ] Test coverage MUST be ≥80% overall
- [ ] Critical paths MUST have ≥95% coverage (MAY exclude system-dependent code with documentation)
- [ ] Integration tests comprehensive
- [ ] Performance tests present
- [ ] No untested code paths

### Code Quality Final Check
- [ ] No .unwrap() or .expect() in production code paths (test code excluded)
- [ ] All TODOs addressed
- [ ] No debug prints
- [ ] Documentation complete
- [ ] Code formatted and linted

### Issues Found
[Final issues before Phase 3]

### Recommendations for Phase 3
[Specific suggestions based on Phase 2 implementation]

### Decision: [APPROVED FOR PHASE 3 / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 2 Status: [COMPLETE / INCOMPLETE]
```

## How to Test Database-Specific Scenarios

### Connection Failure Testing
```bash
# Start SurrealDB on non-standard port
docker run -p 8001:8000 surrealdb/surrealdb:latest

# Run app expecting default port
SURREALDB_ENDPOINT=ws://localhost:8000 cargo run

# MUST observe retry attempts at intervals: 1s, 2s, 4s, 8s, 16s, 32s, 60s (±20% for jitter)
```

### Write Queue Testing
```bash
# Fill the write queue
for i in {1..1000}; do
  curl -X POST http://localhost:8080/api/test \
    -H "Content-Type: application/json" \
    -d '{"data": "test"}'
done

# Next request should get 503
curl -v -X POST http://localhost:8080/api/test
```

### Performance Testing
```bash
# Connection pool stress test
ab -n 10000 -c 100 http://localhost:8080/api/read

# Write queue stress test  
ab -n 5000 -c 50 -p payload.json http://localhost:8080/api/write
```

## Common Issues to Watch For

### Database-Specific Issues
1. **Connection Leaks**: Connections not returned to pool
2. **Retry Storms**: Too aggressive retry without backoff
3. **Queue Memory**: Unbounded queue growth
4. **Thing ID Errors**: Improper SurrealDB ID handling
5. **Timeout Cascades**: Timeouts not properly propagated

### Integration Issues
1. **Health Check Lag**: Database status not reflected quickly
2. **Metrics Explosion**: High cardinality from IDs/queries
3. **Configuration Conflicts**: Database config vs server config
4. **Error Mapping**: Database errors not properly categorized
5. **Startup Race**: Server ready before database connected

## Issue Severity Definitions

**CRITICAL**: Blocks phase completion, MUST fix before approval
- Server exits on database failure
- Data loss scenarios
- Security vulnerabilities
- Specification violations

**HIGH**: Should fix before approval, MAY approve with immediate remediation plan
- Performance degradation >20%
- Missing critical tests
- Resource leaks under load
- Incomplete error handling

**MEDIUM**: Should fix within phase, MAY defer if documented
- Missing documentation
- Code style inconsistencies
- Non-critical test coverage gaps
- Minor performance issues

**LOW**: Track for future improvement
- Optimization opportunities
- Nice-to-have features
- Code refactoring suggestions

## Review Decision Framework

### APPROVED
MUST grant approval ONLY when ALL of the following are met:
- All database operations tested and working
- Retry logic demonstrates proper backoff
- Write queue handles overflow gracefully
- Metrics stay within cardinality limits
- Integration with Phase 1 seamless

### APPROVED WITH CONDITIONS
MAY grant conditional approval when:
- All critical requirements met
- Only MEDIUM or LOW severity issues remain
- Clear remediation plan provided
- Timeline for fixes agreed upon

### CHANGES REQUIRED
MUST require changes when:
- Resource leaks detected
- Retry logic too aggressive
- Queue can lose data
- High cardinality metrics
- Poor error handling
- Missing critical tests

## Final Phase 2 Validation

Before approving Phase 2 completion:
1. Run 24-hour soak test (MUST pass: <1% memory growth, 0 crashes, <0.01% error rate)
2. Verify no significant memory growth (MAY allow up to 1% per hour during recovery)
3. Test database failure/recovery scenarios
4. Validate all metrics stay within cardinality limits (<1000 unique label combinations per metric)
5. Review documentation completeness
6. Ensure smooth Phase 3 transition path

## Recovery Guidance for Reviewers

### When Tests Fail
1. Check if failure is environment-specific
2. Verify test assumptions are correct
3. Allow re-run with documented changes
4. MAY approve with fix commitment if non-critical

### When Coverage is Below Target
1. Verify if excluded code is truly untestable
2. Check if integration tests compensate
3. Document specific reasons for exclusion
4. MAY approve if overall quality is high

### When Performance Doesn't Meet Targets
1. Establish current baseline
2. Identify bottlenecks with profiling
3. Create optimization plan with timeline
4. MAY approve if degradation <20% with plan

Remember: Phase 2 is critical infrastructure. Be thorough in review, as issues here affect all subsequent phases.
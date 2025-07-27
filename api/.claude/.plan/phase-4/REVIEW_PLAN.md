# Phase 4 Review Plan - Authorization & Authentication

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 4 checkpoints. Phase 4 implements secure authorization with SpiceDB integration, caching, and resilience patterns. The system must never fail due to authorization service unavailability while maintaining security.

## Review Context

Phase 4 adds the authorization layer to the PCF-RS API, requiring careful review of:
- Authorization helper pattern consistency
- Caching implementation (positive-only, 5-minute TTL)
- Circuit breaker resilience patterns
- SpiceDB integration and fallback rules
- Security implications of all decisions
- Performance impact on GraphQL operations

## Core Review Principles

### Test-Driven Development (TDD) Verification
At every checkpoint, MUST verify TDD practices by checking:
1. **Authorization tests exist before implementation**
2. **Cache tests precede cache code**
3. **Circuit breaker tests written before state machine**
4. **Fallback tests before fallback rules**
5. **All edge cases covered** - no auth, wrong auth, SpiceDB down, cache full

### Documentation Standards
Authorization code MUST have comprehensive documentation including:
1. **Security implications documented** - Why decisions are conservative
2. **Cache strategy explained** - Why positive-only, TTL choices
3. **Fallback rules justified** - Why specific operations allowed/denied
4. **Circuit breaker thresholds** - Why specific values chosen
5. **Audit requirements clear** - What must be logged and why

### Code Quality Requirements
1. **NO .unwrap() or .expect() in production code paths** - test code and compile-time constants MAY use these with justification
2. **All async operations properly handled** - No forgotten awaits
3. **Resource cleanup guaranteed** - Cache cleanup tasks, connections
4. **No sensitive data in logs** - User IDs okay, tokens/passwords never
5. **Thread safety verified** - All shared state properly protected

## Review Process Flow

1. **Receive checkpoint artifacts**:
   - Checkpoint number and description
   - All code files created/modified
   - Test output showing TDD approach
   - Any questions in `api/.claude/.reviews/checkpoint-X-questions.md`

2. **Execute comprehensive review checklist**

3. **Test the authorization implementation** using provided test commands

4. **Check for cleanliness and artifacts**

5. **Document findings** in `api/.claude/.reviews/checkpoint-X-feedback.md`

6. **Write progress notes** in `api/.claude/.reviews/checkpoint-X-review-vY.md`

7. **Answer any questions** found in `api/.claude/.reviews/checkpoint-X-questions.md`

8. **Provide clear decision**:
   - APPROVED: All requirements met, no issues found
   - APPROVED WITH CONDITIONS: Minor issues that can be fixed in parallel
   - CHANGES REQUIRED: Critical issues blocking progress

## Review Scope Requirements

**MANDATORY**: Reviews are scoped to ONLY:
- The current checkpoint being reviewed
- Previously completed checkpoints in this phase
- Integration with Phase 1, 2, and 3 components

**DO NOT** review or suggest changes for future checkpoints.

## Comprehensive Cleanliness Verification

### Code Cleanliness Checklist
**MUST verify ALL items - any failure requires "CHANGES REQUIRED"**

#### Development Artifacts
- [ ] No temporary files (*.tmp, *.bak, *.orig, test_*, demo_*)
- [ ] No IDE-specific files (.idea/, .vscode/ unless explicitly needed)
- [ ] No build artifacts in wrong directories
- [ ] No test databases or SpiceDB data files committed
- [ ] No example or scratch files

#### Code Hygiene
- [ ] No commented-out code (except with // KEEP: justification)
- [ ] No debug statements (println!, dbg!, console.log, tracing::debug in production paths)
- [ ] No test-only code in production paths
- [ ] No hardcoded values (endpoints, keys, user IDs)
- [ ] No TODO/FIXME without issue numbers
- [ ] No unreachable or dead code
- [ ] No unused imports or variables
- [ ] No temporary workarounds without documentation

#### Security Hygiene
- [ ] No credentials or keys in code
- [ ] No bypass mechanisms in production code
- [ ] No test users or permissions
- [ ] No debug endpoints exposed
- [ ] No sensitive data in error messages
- [ ] No authorization shortcuts

#### Documentation Quality
- [ ] All public APIs have rustdoc/comments
- [ ] Security decisions documented
- [ ] Complex algorithms explained
- [ ] No outdated or misleading comments
- [ ] Configuration options documented
- [ ] Examples work as written

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: Authorization Framework Review

**What You're Reviewing**: Core authorization helper, context extraction, session management, and audit logging

**Key Specifications to Verify**:
- Standard `is_authorized` helper follows specification pattern
- Authentication context properly extracted from headers
- Demo mode bypass only works with feature flag
- Audit logging captures all required fields
- Error responses distinguish 401 vs 403 correctly

**Required Tests** (MUST execute all and verify output):
```bash
# Test authorization helper
cargo test helpers::authorization::tests

# Test authentication context
cargo test auth::tests::test_auth_context

# Test demo mode bypass
cargo test --features demo test_demo_mode_bypass

# Verify audit logging
cargo test auth::audit::tests

# Check error responses
cargo test test_error_codes
```

**Critical Security Reviews**:
- Verify NO authorization bypasses except demo mode
- Check authentication is required for all operations
- Ensure audit logs cannot be disabled
- Validate error messages don't leak information
- Confirm session extraction is secure

**Performance Validation**:
- Context extraction should be < 1ms
- Helper function overhead minimal
- No blocking operations in auth path

**Review Checklist**:
```markdown
## Checkpoint 1 Review - Authorization Framework

### Authorization Helper
- [ ] is_authorized follows exact specification pattern
- [ ] Demo mode bypass properly feature-gated
- [ ] Authentication always required (except demo)
- [ ] Cache check happens before SpiceDB
- [ ] Positive results cached only
- [ ] Audit logging always occurs

### Authentication Context
- [ ] Extracted from correct headers
- [ ] User ID required for authentication
- [ ] Trace ID generated if missing
- [ ] Session token handled securely
- [ ] No sensitive data logged

### Error Handling
- [ ] 401 for missing authentication
- [ ] 403 for denied authorization
- [ ] Error codes follow GraphQL conventions
- [ ] Messages don't reveal system details
- [ ] All paths return proper errors

### Audit Logging
- [ ] Every authorization decision logged
- [ ] Required fields present (user, resource, action, result)
- [ ] Source tracked (cache/spicedb/fallback)
- [ ] Structured JSON format
- [ ] No sensitive data in logs

### Code Quality
- [ ] No .unwrap() in production paths
- [ ] All Results handled explicitly
- [ ] Async functions properly awaited
- [ ] Public APIs documented
- [ ] Tests follow TDD pattern

### Cleanliness Check
- [ ] No debug prints or logs
- [ ] No commented-out code
- [ ] No test users or data
- [ ] No temporary files
- [ ] All TODOs have issue numbers

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 2: Authorization Cache Review

**What You're Reviewing**: Cache trait, implementation, TTL management, and cleanup tasks

**Key Specifications to Verify**:
- Cache stores positive results only (never negative)
- TTL is exactly 5 minutes (300 seconds) normally
- Extended to 30 minutes during SpiceDB outage
- LRU eviction when at capacity
- Background cleanup task running
- Thread-safe concurrent access

**Required Tests** (MUST execute all and verify output):
```bash
# Test cache operations
cargo test auth::cache::tests

# Test TTL expiration
cargo test cache_tests::test_cache_ttl_expiration

# Test LRU eviction
cargo test cache_tests::test_cache_max_size_eviction

# Test cleanup task
cargo test cache_tests::test_cache_cleanup_task

# Verify thread safety
cargo test cache_concurrent_access -- --test-threads=10
```

**Memory and Performance Validation**:
```bash
# Check memory usage
cargo run --example cache_memory_test

# Verify no memory leaks
valgrind --leak-check=full cargo test cache_tests

# Benchmark cache performance
cargo bench cache_operations
```

**Review Checklist**:
```markdown
## Checkpoint 2 Review - Authorization Cache

### Cache Implementation
- [ ] Trait defines all required operations
- [ ] In-memory implementation correct
- [ ] Only positive results cached
- [ ] TTL exactly 5 minutes default
- [ ] Extended TTL 30 minutes for degraded mode

### Cache Behavior
- [ ] Get returns None for expired entries
- [ ] Set overwrites existing entries
- [ ] LRU eviction when full
- [ ] User invalidation clears all user entries
- [ ] Statistics accurately tracked

### Cleanup Task
- [ ] Background task spawned on creation
- [ ] Runs every 60 seconds
- [ ] Removes expired entries
- [ ] Handles eviction correctly
- [ ] Doesn't block operations

### Thread Safety
- [ ] All operations use RwLock appropriately
- [ ] No data races possible
- [ ] Concurrent access tested
- [ ] Performance acceptable under load
- [ ] No deadlock scenarios

### Metrics
- [ ] Cache size tracked
- [ ] Hit/miss rates calculated
- [ ] Operations counted
- [ ] Metrics have low cardinality
- [ ] No sensitive data in metrics

### Resource Management
- [ ] Memory usage bounded
- [ ] No memory leaks
- [ ] Cleanup task can't fail
- [ ] Graceful shutdown supported
- [ ] No zombie tasks

### Cleanliness Check
- [ ] No test cache entries
- [ ] No debug configuration
- [ ] No performance hacks
- [ ] Clean error handling
- [ ] No magic numbers

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: SpiceDB Integration Review

**What You're Reviewing**: SpiceDB client, circuit breaker, fallback rules, and resilience patterns

**Key Specifications to Verify**:
- Circuit breaker prevents cascade failures
- Fallback rules are appropriately conservative
- SpiceDB timeouts configured correctly
- Connection pooling for gRPC works
- Health checks integrated properly
- No authorization bypasses in fallback

**Required Tests** (MUST execute all and verify output):
```bash
# Start SpiceDB for testing
just spicedb-up

# Test SpiceDB client
cargo test spicedb_tests::test_spicedb_permission_check

# Test circuit breaker
cargo test spicedb_tests::test_circuit_breaker_opens_on_failures

# Test fallback rules
cargo test spicedb_tests::test_fallback_rules

# Test health check
cargo test spicedb_health_check_integration

# Stop SpiceDB and test resilience
just spicedb-down
cargo test test_graceful_degradation
```

**Resilience Testing**:
```bash
# Simulate SpiceDB failures
./scripts/test-circuit-breaker.sh

# Test fallback authorization
./scripts/test-fallback-auth.sh

# Verify no cascading failures
./scripts/chaos-test-spicedb.sh
```

**Review Checklist**:
```markdown
## Checkpoint 3 Review - SpiceDB Integration

### SpiceDB Client
- [ ] gRPC connection configured correctly
- [ ] Authentication via preshared key
- [ ] Timeouts reasonable (2s default)
- [ ] Connection pooling enabled
- [ ] Error handling comprehensive

### Circuit Breaker
- [ ] Opens after 3 failures
- [ ] Half-open after 5 seconds
- [ ] Closes after 2 successes
- [ ] State transitions correct
- [ ] Metrics track state changes

### Fallback Rules
- [ ] Read-only operations only
- [ ] Owner can read own resources
- [ ] No cross-user access
- [ ] No write operations
- [ ] Health checks always allowed

### Integration
- [ ] Helper uses circuit breaker
- [ ] Fallback triggered correctly
- [ ] Cache TTL extended during outage
- [ ] Audit logs show correct source
- [ ] Errors handled gracefully

### Security in Degraded Mode
- [ ] No privilege escalation
- [ ] Conservative decisions only
- [ ] Clear audit trail
- [ ] No data leakage
- [ ] Appropriate warnings logged

### Performance
- [ ] SpiceDB calls < 50ms p95
- [ ] Circuit breaker adds < 1ms
- [ ] Fallback decisions < 1ms
- [ ] No blocking operations
- [ ] Connection pool sized well

### Cleanliness Check
- [ ] No hardcoded endpoints
- [ ] No test permissions
- [ ] No SpiceDB test data
- [ ] Clean connection handling
- [ ] No temporary workarounds

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 4: Complete Integration Review

**What You're Reviewing**: Full authorization integration, GraphQL resolver updates, demo mode, and system verification

**Key Specifications to Verify**:
- All GraphQL operations use authorization
- Demo mode properly gated by feature flag
- Integration tests cover all scenarios
- Performance benchmarks acceptable
- No authorization bypasses exist
- Production ready

**Required Tests** (MUST execute all and verify output):
```bash
# Run full verification script
./scripts/verify-phase-4.sh

# Test all GraphQL operations
cargo test integration_tests::test_full_authorization_flow

# Test error responses
cargo test integration_tests::test_unauthorized_returns_401
cargo test integration_tests::test_forbidden_returns_403

# Test demo mode
cargo test --features demo integration_tests::test_demo_mode_bypass

# Performance benchmarks
cargo bench benchmark_authorization
```

**Production Readiness Validation**:
```bash
# Security scan
cargo audit

# Check for unwraps
! grep -r "\.unwrap()" src/ --include="*.rs" | grep -v "^src/.*test"

# Verify no demo in release
cargo build --release 2>&1 | grep -q "Demo mode MUST NOT be enabled" && echo "PASS"

# Load test authorization
artillery run load-tests/auth-load-test.yml
```

**Review Checklist**:
```markdown
## Checkpoint 4 Review - Complete Integration

### GraphQL Integration
- [ ] All queries use is_authorized
- [ ] All mutations check permissions
- [ ] Subscriptions verify authorization
- [ ] Consistent error handling
- [ ] No bypasses or shortcuts

### Demo Mode
- [ ] Only enabled with feature flag
- [ ] Compile error in release builds
- [ ] Clear warning when enabled
- [ ] All operations bypassed
- [ ] No demo code in production paths

### Integration Tests
- [ ] Full auth flow tested
- [ ] 401 vs 403 responses correct
- [ ] Circuit breaker scenarios tested
- [ ] Fallback rules tested
- [ ] Cache behavior verified

### Performance
- [ ] Cache hit rate >90% in tests
- [ ] Authorization <5ms p99 (cached)
- [ ] Authorization <50ms p99 (SpiceDB)
- [ ] No performance regressions
- [ ] Benchmarks establish baseline

### Security Validation
- [ ] No authorization bypasses
- [ ] All endpoints protected
- [ ] Audit trail complete
- [ ] No sensitive data leaks
- [ ] Demo mode safe

### Production Readiness
- [ ] All tests passing
- [ ] No security vulnerabilities
- [ ] Documentation complete
- [ ] Metrics and monitoring ready
- [ ] Deployment guide updated

### Final Cleanliness
- [ ] No test data anywhere
- [ ] No debug configurations
- [ ] No temporary solutions
- [ ] No outstanding TODOs
- [ ] Code ready for production

### Decision: [APPROVED FOR PHASE 5 / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 4 Status: [COMPLETE / INCOMPLETE]
```

## Issue Severity Definitions

**CRITICAL**: Blocks phase completion, MUST fix before approval
- Authorization bypasses
- Security vulnerabilities
- Data leakage risks
- System crashes or panics
- Missing authentication checks

**HIGH**: Should fix before approval, MAY approve with immediate remediation plan
- Missing tests for critical paths
- Performance degradation >20%
- Incomplete error handling
- Thread safety issues
- Missing audit logs

**MEDIUM**: Should fix within phase, MAY defer if documented
- Missing documentation
- Code style inconsistencies
- Non-critical test coverage gaps
- Minor performance issues
- Incomplete metrics

**LOW**: Track for future improvement
- Optimization opportunities
- Nice-to-have features
- Code refactoring suggestions
- Enhanced monitoring

## How to Test Authorization Scenarios

### Authentication Testing
```bash
# Test missing authentication
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ notes { edges { node { id } } } }"}' \
  | jq '.errors[0].extensions.code' # Should be "UNAUTHORIZED"

# Test with authentication
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -H "x-user-id: alice" \
  -d '{"query":"{ notes { edges { node { id } } } }"}' \
  | jq '.data' # Should return results
```

### Authorization Testing
```bash
# Setup test permissions in SpiceDB
zed relationship create notes:123 owner user:alice
zed relationship create notes:456 owner user:bob

# Test authorized access
curl -X POST http://localhost:8080/graphql \
  -H "x-user-id: alice" \
  -d '{"query":"{ note(id: \"123\") { title } }"}' # Should work

# Test unauthorized access
curl -X POST http://localhost:8080/graphql \
  -H "x-user-id: alice" \
  -d '{"query":"{ note(id: \"456\") { title } }"}' # Should be FORBIDDEN
```

### Circuit Breaker Testing
```bash
# Stop SpiceDB to trigger circuit breaker
docker stop spicedb

# Make several requests to open circuit
for i in {1..5}; do
  curl -X POST http://localhost:8080/graphql \
    -H "x-user-id: alice" \
    -d '{"query":"{ note(id: \"alice:123\") { title } }"}'
done

# Check metrics for circuit state
curl -s http://localhost:8080/metrics | grep circuit_breaker_state
```

## Common Issues to Watch For

### Authorization Issues
1. **Missing auth checks**: Every GraphQL operation must call is_authorized
2. **Wrong resource format**: Must be "type:id" format
3. **Cache key format**: Must be "user:resource:action"
4. **Negative caching**: Never cache authorization denials
5. **Demo mode leaks**: No demo code in production builds

### Performance Issues
1. **Cache misses**: Should be <10% after warm-up
2. **SpiceDB latency**: Check connection pooling
3. **Circuit breaker too sensitive**: Adjust thresholds
4. **Memory growth**: Check cache eviction
5. **Slow fallback**: Fallback should be instant

### Security Issues
1. **Information leakage**: Error messages revealing system details
2. **Timing attacks**: Consistent response times
3. **Privilege escalation**: Fallback rules too permissive
4. **Missing audit logs**: Every decision must be logged
5. **Token exposure**: Never log authentication tokens

## Review Decision Framework

### APPROVED
Grant approval ONLY when ALL of the following are met:
- All checklist items pass
- No CRITICAL or HIGH issues
- Tests achieve >95% coverage on auth paths
- Security review finds no vulnerabilities
- Performance meets requirements
- No cleanliness issues found

### APPROVED WITH CONDITIONS
MAY grant conditional approval when:
- All CRITICAL issues resolved
- Plan exists for HIGH issues
- MEDIUM issues documented
- Timeline for fixes agreed
- No security risks

### CHANGES REQUIRED
MUST require changes when:
- Any CRITICAL issues found
- Security vulnerabilities exist
- Authorization can be bypassed
- Tests failing or insufficient
- Major cleanliness issues
- Performance unacceptable

## Final Phase 4 Validation

Before approving Phase 4 completion:
1. Manually test authorization on all endpoints
2. Verify audit logs capture all decisions
3. Simulate SpiceDB outage and test fallback
4. Check cache metrics and hit rates
5. Ensure no authorization bypasses exist
6. Validate demo mode is development-only
7. Confirm production deployment ready

## Recovery Guidance for Reviewers

### When Tests Fail
1. Check if SpiceDB is running (`docker ps`)
2. Verify test database has fixtures
3. Check for port conflicts
4. Review test logs for details

### When Integration Fails
1. Verify all components running
2. Check configuration alignment
3. Review integration test setup
4. Trace through auth flow

### When Performance is Poor
1. Check cache configuration
2. Verify SpiceDB connection pooling
3. Look for synchronous operations
4. Profile authorization path

Remember: Phase 4 establishes the security foundation for the entire API. Be thorough in review, as authorization bugs can have severe consequences.
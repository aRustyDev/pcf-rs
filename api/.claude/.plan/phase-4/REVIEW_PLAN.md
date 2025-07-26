# Phase 4 Review Plan - Guidelines for Reviewing Agents

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 4 checkpoints. As a reviewing agent, you are responsible for ensuring the authorization implementation meets all specifications, provides proper resilience, and integrates seamlessly with previous phases.

## Your Responsibilities as Reviewer

1. **Thoroughly examine all provided artifacts**
2. **Test authorization flows end-to-end**
3. **Verify resilience patterns (circuit breaker, fallback)**
4. **Check cache implementation and performance**
5. **Validate audit logging completeness**
6. **Test degraded mode operation**
7. **Ensure security best practices**
8. **Verify TDD practices were followed**
9. **Provide explicit feedback on findings**
10. **Give clear approval or rejection**

## Core Review Principles

### Authorization Security Verification
At every checkpoint, verify security requirements are met:
1. **Fail-secure approach** - Default deny, explicit allow
2. **Proper error codes** - 401 vs 403 distinction
3. **No information leakage** - Generic error messages
4. **Audit completeness** - Every decision logged
5. **Positive-only caching** - Never cache denials

### Resilience Verification
Ensure the system remains available during failures:
1. **Circuit breaker works** - Opens/closes appropriately
2. **Fallback rules apply** - Conservative permissions
3. **Cache extends during outage** - 30-minute TTL
4. **Health checks bypass auth** - Always accessible
5. **Graceful degradation** - Read-only during outage

### Performance Verification
Check authorization doesn't become a bottleneck:
1. **Cache hit rate > 80%** - After warm-up period
2. **Sub-10ms cached checks** - Fast path optimization
3. **Batch authorization** - For list operations
4. **LRU eviction works** - Cache doesn't grow unbounded
5. **Metrics available** - For monitoring

### Test-Driven Development (TDD) Verification
Continue verifying TDD practices:
1. **Tests exist before implementation** - Check git history
2. **Tests fail first, then pass** - Red-Green-Refactor
3. **Tests cover failure scenarios** - Outages, denials
4. **Integration tests comprehensive** - End-to-end flows

## Review Process

For each checkpoint review:

1. **Receive from implementing agent**:
   - Link to Phase 4 REVIEW_PLAN.md
   - Link to Phase 4 WORK_PLAN.md
   - Specific checkpoint number
   - All artifacts listed for that checkpoint
   - Evidence of TDD approach

2. **Perform the review** using checkpoint-specific checklist

3. **Test critical authorization features**:
   - Permission checks work correctly
   - Circuit breaker activates on failures
   - Cache improves performance
   - Fallback rules are conservative
   - Audit logs capture all decisions

4. **Document your findings** in structured format

5. **Provide clear decision**: APPROVED or CHANGES REQUIRED

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: Authorization Framework Review

**What You're Reviewing**: Core authorization helper and context

**Key Specifications to Verify**:
- Standard is_authorized function exists
- Auth context properly extracted
- Demo mode bypass works
- Fallback rules implemented
- Session extraction from JWT

**Required Tests**:
```bash
# Test authorization helper
cargo test helpers::authorization

# Test without authentication
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ note(id: \"123\") { title } }"}'
# Should return UNAUTHORIZED

# Test with authentication but no permission
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test_token" \
  -d '{"query":"{ note(id: \"restricted\") { title } }"}'
# Should return FORBIDDEN

# Test demo mode
DEMO_MODE=true cargo run --features demo
# Should bypass all auth checks
```

**Fallback Rules Test**:
```bash
# Simulate SpiceDB unavailable
# Only these should work:
# - Health checks (always)
# - User reading own profile
# - All writes should be denied
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 1 Review Results

### Authorization Helper
- [ ] is_authorized function exists: [YES/NO]
- [ ] Follows specification pattern: [YES/NO]
- [ ] Returns proper error types: [YES/NO]
- [ ] Integrates with GraphQL context: [YES/NO]

### Auth Context
- [ ] Extracts from headers correctly: [YES/NO]
- [ ] Handles missing auth: [YES/NO]
- [ ] Parses JWT claims: [YES/NO]
- [ ] Demo mode tokens work: [YES/NO]

### Error Handling
- [ ] Returns 401 for no auth: [YES/NO - test output]
- [ ] Returns 403 for no permission: [YES/NO - test output]
- [ ] Error messages are generic: [YES/NO]
- [ ] No internal details leaked: [YES/NO]

### Fallback Rules
- [ ] Health checks always allowed: [YES/NO]
- [ ] Own profile readable: [YES/NO]
- [ ] All writes denied: [YES/NO]
- [ ] Conservative by default: [YES/NO]

### Demo Mode
- [ ] Bypass flag works: [YES/NO]
- [ ] Only in debug builds: [YES/NO]
- [ ] Clearly documented: [YES/NO]

### TDD Verification
- [ ] Tests written first: [YES/NO - evidence]
- [ ] Auth scenarios covered: [YES/NO]
- [ ] Fallback rules tested: [YES/NO]
- [ ] Error cases tested: [YES/NO]

### Issues Found
[List with severity and fixes]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 2: Cache and SpiceDB Integration Review

**What You're Reviewing**: Authorization cache and SpiceDB client

**Key Specifications to Verify**:
- Cache only stores positive results
- LRU eviction when full
- SpiceDB client with circuit breaker
- TTL extends during outages
- Proper connection pooling

**Required Tests**:
```bash
# Test cache behavior
cargo test auth::cache

# Test cache performance
# First request (cold cache)
time curl -X POST http://localhost:8080/graphql \
  -H "Authorization: Bearer test_token" \
  -d '{"query":"{ note(id: \"123\") { title } }"}'

# Second request (warm cache)
time curl -X POST http://localhost:8080/graphql \
  -H "Authorization: Bearer test_token" \
  -d '{"query":"{ note(id: \"123\") { title } }"}'
# Should be significantly faster

# Test circuit breaker
# Make SpiceDB unavailable
docker stop spicedb
# Make several requests - circuit should open
# Further requests should use fallback immediately
```

**Cache Verification**:
```bash
# Check metrics
curl http://localhost:8080/metrics | grep auth_cache
# Should show:
# - auth_cache_size
# - auth_cache_hits_total
# - auth_cache_misses_total
# - auth_cache_evictions_total

# Verify positive-only caching
# Make request that gets denied
# Check cache size doesn't increase
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 2 Review Results

### Cache Implementation
- [ ] Only caches positive results: [YES/NO - verified how]
- [ ] LRU eviction works: [YES/NO - test output]
- [ ] TTL management correct: [YES/NO]
- [ ] Thread-safe operations: [YES/NO]
- [ ] Metrics exposed: [YES/NO - list metrics]

### Cache Performance
- [ ] Hit rate after warm-up: ___% (target > 80%)
- [ ] Cached check latency: ___ms (target < 10ms)
- [ ] Memory usage reasonable: [YES/NO - size: ___]
- [ ] Eviction rate acceptable: [YES/NO - rate: ___/min]

### SpiceDB Client
- [ ] Connection pooling works: [YES/NO]
- [ ] Timeout configured (2s): [YES/NO]
- [ ] Bulk checks implemented: [YES/NO]
- [ ] Error handling proper: [YES/NO]

### Circuit Breaker
- [ ] Opens after failures: [YES/NO - threshold: ___]
- [ ] Closes after success: [YES/NO - threshold: ___]
- [ ] Half-open state works: [YES/NO]
- [ ] Metrics track state: [YES/NO]

### Degraded Mode
- [ ] Cache TTL extends to 30min: [YES/NO - verified]
- [ ] Fallback rules activate: [YES/NO]
- [ ] No service disruption: [YES/NO]
- [ ] Appropriate logging: [YES/NO - log samples]

### TDD Verification
- [ ] Cache tests comprehensive: [YES/NO]
- [ ] Circuit breaker tested: [YES/NO]
- [ ] Integration tests exist: [YES/NO]
- [ ] Failure scenarios covered: [YES/NO]

### Issues Found
[Detailed findings with impact]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: Complete Phase 4 System Review

**What You're Reviewing**: Complete authorization system integration

**Key Specifications to Verify**:
- All GraphQL operations check auth
- Audit logging captures everything
- Metrics provide visibility
- System resilient to failures
- Performance acceptable

**Required Comprehensive Tests**:
```bash
# Run verification script
./scripts/verify-phase-4.sh
# Should complete all checks

# Test complete auth flow
cargo test --test auth_integration

# Audit log verification
# Make several auth decisions
# Check audit log contains all decisions
tail -f /var/log/auth_audit.log | jq .

# Performance test
# Run 1000 requests, measure cache effectiveness
ab -n 1000 -c 10 -H "Authorization: Bearer test_token" \
  http://localhost:8080/graphql
```

**Resilience Testing**:
```bash
# Test 1: SpiceDB outage handling
docker stop spicedb
# System should continue with fallback
# Health checks still work
# Writes are denied
# Cache extends TTL

# Test 2: SpiceDB recovery
docker start spicedb
# Circuit breaker should recover
# Normal operations resume
# Cache TTL returns to normal

# Test 3: High load
# Generate many unique permission checks
# Verify LRU eviction works
# No memory leaks
```

**Your Review Output Should Include**:
```markdown
## Phase 4 Complete System Review

### Done Criteria Verification
- [ ] All endpoints require authorization: [YES/NO - exceptions: ___]
- [ ] SpiceDB permission checks working: [YES/NO]
- [ ] Authorization caching reduces load: [YES/NO - metrics]
- [ ] Proper 401 vs 403 responses: [YES/NO - tested]
- [ ] Demo mode bypass functional: [YES/NO]
- [ ] Circuit breaker prevents cascades: [YES/NO]
- [ ] Fallback rules work: [YES/NO - tested scenarios]
- [ ] Audit logging complete: [YES/NO - sample logs]
- [ ] Metrics track performance: [YES/NO - metrics list]

### Integration Testing
- [ ] GraphQL resolvers use is_authorized: [YES/NO - count: ___]
- [ ] Batch authorization for lists: [YES/NO]
- [ ] Session extraction works: [YES/NO]
- [ ] Error responses consistent: [YES/NO]

### Resilience Testing
SpiceDB Outage:
- [ ] System remains available: [YES/NO]
- [ ] Fallback rules apply: [YES/NO]
- [ ] Health checks work: [YES/NO]
- [ ] Appropriate error messages: [YES/NO]

Recovery:
- [ ] Circuit breaker recovers: [YES/NO - time: ___]
- [ ] Normal operations resume: [YES/NO]
- [ ] No manual intervention needed: [YES/NO]

### Performance Analysis
- [ ] Cache hit rate: ___% (target > 80%)
- [ ] Average auth check latency: ___ms
- [ ] P99 latency: ___ms
- [ ] Requests per second: ___
- [ ] Memory usage stable: [YES/NO]

### Audit & Compliance
- [ ] All decisions logged: [YES/NO - verified how]
- [ ] Log format structured: [YES/NO - sample]
- [ ] No sensitive data in logs: [YES/NO]
- [ ] Audit queries work: [YES/NO]
- [ ] Retention policy defined: [YES/NO]

### Security Review
- [ ] Fail-secure implementation: [YES/NO]
- [ ] No authorization bypasses: [YES/NO]
- [ ] JWT validation proper: [YES/NO]
- [ ] Demo mode production-safe: [YES/NO]

### Documentation Review
- [ ] Architecture documented: [YES/NO]
- [ ] SpiceDB schema clear: [YES/NO]
- [ ] Troubleshooting guide: [YES/NO]
- [ ] Metrics descriptions: [YES/NO]

### Outstanding Issues
[Any issues for Phase 5 consideration]

### Recommendations for Phase 5
[Suggestions based on auth implementation]

### Decision: [APPROVED FOR PHASE 5 / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 4 Status: [COMPLETE / INCOMPLETE]
```

## How to Handle Issues

When you find issues during review:

1. **Categorize by severity**:
   - **CRITICAL**: Security vulnerabilities, auth bypasses
   - **HIGH**: Resilience failures, no audit logs
   - **MEDIUM**: Performance issues, missing metrics
   - **LOW**: Documentation, code style

2. **Test security thoroughly**:
   - Can auth be bypassed? (CRITICAL)
   - Do fallback rules expose data? (CRITICAL)
   - Are denials cached? (HIGH)
   - Is audit logging complete? (HIGH)

3. **Provide specific fixes**:
   ```markdown
   Issue: Negative results being cached
   Severity: HIGH
   Fix: Add check in cache.set():
   ```rust
   if !allowed {
       return; // Never cache negative results
   }
   ```

## Review Decision Framework

### APPROVED
Grant approval when:
- All Done Criteria met
- Security implementation solid
- Resilience patterns work
- Performance acceptable
- Only LOW severity issues

### CHANGES REQUIRED
Require changes when:
- Security vulnerabilities found
- Auth can be bypassed
- No resilience to failures
- Audit logging incomplete
- Any CRITICAL or HIGH issues

## Testing Authorization Resilience

Critical scenarios that MUST be tested:

1. **SpiceDB Unavailable at Startup**
   - Server should start successfully
   - Health checks should work
   - Fallback rules should apply
   - Clear logging of degraded state

2. **SpiceDB Fails During Operation**
   - Existing cache continues working
   - New requests use fallback
   - Circuit breaker activates
   - Automatic recovery when available

3. **Cache Overflow**
   - LRU eviction activates
   - Performance remains stable
   - No memory leaks
   - Metrics show evictions

4. **High Permission Churn**
   - Many unique permission checks
   - Cache remains effective
   - No thundering herd on SpiceDB

## Final Review Checklist

Before submitting your review:
- [ ] Tested all authorization flows
- [ ] Verified resilience patterns
- [ ] Checked cache effectiveness
- [ ] Validated audit completeness
- [ ] Reviewed security implementation
- [ ] Tested degraded mode operation
- [ ] Made clear APPROVED/CHANGES REQUIRED decision
- [ ] Included specific remediation if needed

## Template for Review Submission

```markdown
# Phase 4 - Checkpoint [N] Review

**Reviewer**: [Your designation]
**Date**: [Current date]
**Implementation Agent**: [Agent who requested review]

## Review Summary
[2-3 sentences summarizing authorization implementation state]

## Detailed Findings
[Your complete review output for this checkpoint]

## Security Assessment
- Authorization enforcement: [Complete/Gaps found]
- Error handling: [Secure/Information leaks]
- Audit trail: [Complete/Missing decisions]
- Demo mode: [Safe/Security risks]

## Resilience Assessment
- Circuit breaker: [Working/Issues]
- Fallback rules: [Conservative/Too permissive]
- Cache effectiveness: [High/Low - metrics]
- Degraded operation: [Smooth/Disrupted]

## Required Actions
1. [Specific action with priority]
2. [Specific action with priority]

## Decision
**[APPROVED / CHANGES REQUIRED]**

[If CHANGES REQUIRED]
The implementing agent must:
1. [Specific requirement]
2. [Specific requirement]
3. Request re-review when complete

[If APPROVED]
The implementing agent may proceed to [next checkpoint/phase].
```

Remember: Authorization is critical for security. Be thorough in testing all flows, especially failure scenarios. The system must remain secure even when degraded.
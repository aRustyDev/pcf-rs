# Phase 3 Review Plan - Guidelines for Reviewing Agents

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 3 checkpoints. As a reviewing agent, you are responsible for ensuring the GraphQL implementation meets all specifications, integrates properly with Phase 1 & 2 infrastructure, and follows security best practices.

## Your Responsibilities as Reviewer

1. **Thoroughly examine all provided artifacts**
2. **Test GraphQL operations (queries, mutations, subscriptions)**
3. **Verify security controls are enforced**
4. **Check performance optimizations work**
5. **Validate error handling and messages**
6. **Test WebSocket subscriptions**
7. **Ensure metrics collection functions**
8. **Verify TDD practices were followed**
9. **Provide explicit feedback on findings**
10. **Give clear approval or rejection**

## Core Review Principles

### GraphQL Security Verification
At every checkpoint, verify security requirements are met:
1. **Query depth limiting** - Default 15, configurable
2. **Query complexity calculation** - Default 1000 points
3. **Introspection disabled** - Only in production
4. **Playground restricted** - Demo mode only
5. **Input validation** - All inputs validated with Garde

### Performance Verification
Ensure performance optimizations are implemented:
1. **DataLoader pattern** - N+1 queries prevented
2. **Query timeouts** - 30 second maximum
3. **Subscription limits** - 1000 per instance
4. **Caching where appropriate** - Schema introspection cached
5. **Efficient pagination** - Cursor-based with limits

### Test-Driven Development (TDD) Verification
Continue verifying TDD practices from previous phases:
1. **Tests exist before implementation** - Check git history
2. **Tests fail first, then pass** - Red-Green-Refactor cycle
3. **Tests drive the design** - Implementation matches test requirements
4. **Tests are comprehensive** - All operations, security, errors

### Integration Standards
Phase 3 must integrate seamlessly with Phase 1 & 2:
1. **Uses Phase 1 error types** - Mapped to GraphQL errors
2. **Uses Phase 2 database layer** - Through context
3. **Respects health check system** - GraphQL status included
4. **Emits proper metrics** - Prometheus format
5. **Follows logging standards** - Structured with trace IDs

## Review Process

For each checkpoint review:

1. **Receive from implementing agent**:
   - Link to Phase 3 REVIEW_PLAN.md
   - Link to Phase 3 WORK_PLAN.md
   - Specific checkpoint number
   - All artifacts listed for that checkpoint
   - Evidence of TDD approach

2. **Perform the review** using checkpoint-specific checklist

3. **Test critical GraphQL features**:
   - Query execution
   - Mutation operations
   - Subscription delivery
   - Security limits
   - Error handling

4. **Document your findings** in structured format

5. **Provide clear decision**: APPROVED or CHANGES REQUIRED

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: GraphQL Foundation Review

**What You're Reviewing**: Schema setup, context, and error mapping

**Key Specifications to Verify**:
- GraphQL schema builds successfully
- Context includes database and session
- Errors map correctly from AppError
- Security configurations in place
- Introspection disabled in production

**Required Tests**:
```bash
# Test schema builds
cargo test graphql::tests::test_schema_builds_successfully

# Test introspection control
ENVIRONMENT=production cargo test test_introspection_disabled

# Verify error mapping
cargo test graphql::errors

# Check context setup
cargo doc --no-deps --open
# Review GraphQLContext implementation
```

**GraphQL Playground Test**:
```bash
# Start in demo mode
cargo run --features demo

# Visit http://localhost:8080/graphql
# Should see GraphQL playground

# Start in production mode
ENVIRONMENT=production cargo run
# Visit http://localhost:8080/graphql
# Should get 404
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 1 Review Results

### Schema Foundation
- [ ] Schema builds without errors: [YES/NO]
- [ ] All types properly defined: [YES/NO]
- [ ] Extensions configured: [YES/NO]
- [ ] Security limits set: [YES/NO]

### Context Implementation
- [ ] Database service accessible: [YES/NO]
- [ ] Session handling present: [YES/NO]
- [ ] Trace ID propagated: [YES/NO]
- [ ] Request timing tracked: [YES/NO]

### Error Handling
- [ ] AppError → GraphQL mapping works: [YES/NO]
- [ ] Error codes preserved: [YES/NO]
- [ ] Internal errors sanitized: [YES/NO]
- [ ] Validation errors clear: [YES/NO]

### Security Configuration
- [ ] Introspection disabled in prod: [YES/NO - test output]
- [ ] Depth limit configurable: [YES/NO]
- [ ] Complexity limit configurable: [YES/NO]
- [ ] Demo mode check works: [YES/NO]

### TDD Verification
- [ ] Tests written before schema: [YES/NO]
- [ ] Context tests comprehensive: [YES/NO]
- [ ] Error mapping tested: [YES/NO]
- [ ] Security tests present: [YES/NO]

### Issues Found
[List with severity and fixes]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 2: Query and Mutation Resolvers Review

**What You're Reviewing**: All query and mutation implementations

**Key Specifications to Verify**:
- All queries work correctly
- Pagination follows connection pattern
- Mutations validate input
- Authorization checks present
- Proper error responses

**Required Tests**:
```bash
# Test all queries
cargo test query_tests

# Test pagination
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ notes(limit: 5, offset: 10) { edges { node { title } } pageInfo { hasNextPage totalCount } } }"
  }'

# Test mutations
cargo test mutation_tests

# Test input validation
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createNote(input: { title: \"\", content: \"test\", author: \"user\" }) { id } }"
  }'
# Should return validation error
```

**Authorization Tests**:
```bash
# In demo mode - should work without auth
cargo run --features demo

# In production mode - should require auth
ENVIRONMENT=production cargo run
# Queries without auth header should fail
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 2 Review Results

### Query Implementation
- [ ] note(id) query works: [YES/NO - test result]
- [ ] notes pagination works: [YES/NO - test result]
- [ ] notesByAuthor filters correctly: [YES/NO]
- [ ] searchNotes returns relevant results: [YES/NO]
- [ ] health query always accessible: [YES/NO]

### Pagination Review
- [ ] Connection type structure correct: [YES/NO]
- [ ] Cursors properly encoded: [YES/NO]
- [ ] Page info accurate: [YES/NO]
- [ ] Limits enforced (1-100): [YES/NO]
- [ ] Offset limits enforced (0-10000): [YES/NO]

### Mutation Implementation
- [ ] createNote validates input: [YES/NO - test output]
- [ ] updateNote checks ownership: [YES/NO]
- [ ] deleteNote removes record: [YES/NO]
- [ ] All mutations return updated data: [YES/NO]

### Input Validation
- [ ] Title length enforced (1-200): [YES/NO]
- [ ] Content length enforced (1-10000): [YES/NO]
- [ ] Author validated (1-100): [YES/NO]
- [ ] Empty updates rejected: [YES/NO]
- [ ] Clear validation messages: [YES/NO - examples]

### Authorization
- [ ] Demo mode skips auth: [YES/NO]
- [ ] Production requires auth: [YES/NO]
- [ ] Ownership checks work: [YES/NO]
- [ ] Clear auth error messages: [YES/NO]

### TDD Verification
- [ ] Query tests written first: [YES/NO]
- [ ] Mutation tests comprehensive: [YES/NO]
- [ ] Edge cases tested: [YES/NO]
- [ ] Authorization tested: [YES/NO]

### Issues Found
[Detailed findings with remediation]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: Subscriptions and Security Review

**What You're Reviewing**: WebSocket subscriptions and security controls

**Key Specifications to Verify**:
- Subscriptions work over WebSocket
- Events properly broadcast
- Security limits enforced
- DataLoader prevents N+1
- Complexity calculation works

**Required Tests**:
```bash
# Test subscriptions with wscat
npm install -g wscat
wscat -c ws://localhost:8080/graphql/ws -s graphql-ws

# Send connection init
{"type":"connection_init"}

# Subscribe to note creation
{
  "id": "1",
  "type": "start",
  "payload": {
    "query": "subscription { noteCreated { id title author } }"
  }
}

# In another terminal, create a note
# Should receive event in wscat
```

**Security Limit Tests**:
```bash
# Test depth limit
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ notes { edges { node { related { edges { node { related { edges { node { related { edges { node { title } } } } } } } } } } } }"
  }'
# Should fail with depth error

# Test complexity limit
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ notes(limit: 1001) { edges { node { id } } } }"
  }'
# Should fail with complexity error
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 3 Review Results

### Subscription Implementation
- [ ] WebSocket endpoint works: [YES/NO]
- [ ] Connection init handled: [YES/NO]
- [ ] noteCreated events delivered: [YES/NO]
- [ ] noteUpdated with filter works: [YES/NO]
- [ ] noteDeleted returns ID: [YES/NO]

### Event Broadcasting
- [ ] Mutations trigger events: [YES/NO - verified how]
- [ ] Events reach subscribers: [YES/NO]
- [ ] Multiple subscribers supported: [YES/NO]
- [ ] Disconnection handled cleanly: [YES/NO]

### Security Controls
- [ ] Depth limit enforced: [YES/NO - test output]
- [ ] Complexity limit enforced: [YES/NO - test output]
- [ ] Limits configurable: [YES/NO]
- [ ] Clear error messages: [YES/NO - examples]
- [ ] Introspection blocked in prod: [YES/NO]

### Performance Optimizations
- [ ] DataLoader implemented: [YES/NO]
- [ ] Batch loading works: [YES/NO]
- [ ] N+1 queries prevented: [YES/NO - evidence]
- [ ] Subscription limits enforced: [YES/NO]

### Connection Management
- [ ] Max subscriptions limited: [YES/NO]
- [ ] Idle timeout implemented: [YES/NO]
- [ ] Resource cleanup on disconnect: [YES/NO]
- [ ] Connection count tracked: [YES/NO]

### TDD Verification
- [ ] Subscription tests written first: [YES/NO]
- [ ] Security tests comprehensive: [YES/NO]
- [ ] DataLoader tests present: [YES/NO]
- [ ] Edge cases covered: [YES/NO]

### Issues Found
[List security or performance issues]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 4: Complete Phase 3 System Review

**What You're Reviewing**: Complete GraphQL implementation

**Key Specifications to Verify**:
- All Phase 3 "Done Criteria" met
- Integration with server complete
- Metrics collection working
- Production-ready code quality

**Required Comprehensive Tests**:
```bash
# Run verification script
./scripts/verify-phase-3.sh
# Should complete all checks

# Test complete flow
cargo test --test graphql_integration

# Check metrics
curl http://localhost:8080/metrics | grep graphql_
# Should see:
# - graphql_request_total
# - graphql_request_duration_seconds
# - graphql_field_resolution_duration_seconds
# - graphql_active_subscriptions

# Load test
npm install -g artillery
artillery quick --count 10 --num 10 \
  -p '{"query":"{ notes { edges { node { title } } } }"}' \
  http://localhost:8080/graphql
```

**Production Readiness Tests**:
```bash
# Test with production settings
ENVIRONMENT=production \
GRAPHQL_MAX_DEPTH=10 \
GRAPHQL_MAX_COMPLEXITY=500 \
cargo run

# Verify:
# - No playground at /graphql
# - No introspection
# - Limits enforced
# - Proper error messages
```

**Your Review Output Should Include**:
```markdown
## Phase 3 Complete System Review

### Done Criteria Verification
- [ ] GraphQL playground accessible (demo): [YES/NO]
- [ ] All queries functional: [YES/NO - list tested]
- [ ] All mutations functional: [YES/NO - list tested]
- [ ] All subscriptions functional: [YES/NO - list tested]
- [ ] Security controls enforced: [YES/NO - evidence]
- [ ] Error handling proper: [YES/NO - examples]
- [ ] Schema export works (demo): [YES/NO]
- [ ] N+1 prevented: [YES/NO - how verified]
- [ ] Metrics track operations: [YES/NO - output]

### Integration Review
- [ ] Routes properly configured: [YES/NO]
- [ ] Context propagation works: [YES/NO]
- [ ] Database integration smooth: [YES/NO]
- [ ] Health check includes GraphQL: [YES/NO]
- [ ] Logging includes operations: [YES/NO]

### Security Verification
Query Security:
- [ ] Depth limit works: [YES/NO - tested at depth ___]
- [ ] Complexity limit works: [YES/NO - tested at ___]
- [ ] Introspection disabled (prod): [YES/NO]
- [ ] Playground disabled (prod): [YES/NO]
- [ ] Malicious queries rejected: [YES/NO]

Input Security:
- [ ] All inputs validated: [YES/NO]
- [ ] Injection attempts blocked: [YES/NO]
- [ ] Clear validation errors: [YES/NO]

### Performance Review
- [ ] DataLoader reduces queries: [YES/NO - before/after]
- [ ] Subscription limits work: [YES/NO - tested limit]
- [ ] Query timeouts enforced: [YES/NO]
- [ ] Response times acceptable: [YES/NO - p99: ___ms]
- [ ] Memory usage stable: [YES/NO]

### Metrics Analysis
Observed metrics:
- Total requests: ___
- Average duration: ___ms
- Error rate: ___%
- Active subscriptions: ___
- Field resolution times: [List slowest]

### Code Quality
- [ ] All resolvers documented: [YES/NO]
- [ ] Error messages helpful: [YES/NO]
- [ ] No .unwrap() in resolvers: [YES/NO]
- [ ] Consistent code style: [YES/NO]
- [ ] Tests comprehensive: [YES/NO - coverage: ___%]

### Documentation Review
- [ ] API documentation complete: [YES/NO]
- [ ] Security configuration documented: [YES/NO]
- [ ] Performance tuning guide present: [YES/NO]
- [ ] Example queries provided: [YES/NO]

### Outstanding Issues
[Any issues for Phase 4 consideration]

### Recommendations for Phase 4
[Suggestions based on GraphQL implementation]

### Decision: [APPROVED FOR PHASE 4 / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 3 Status: [COMPLETE / INCOMPLETE]
```

## How to Handle Issues

When you find issues during review:

1. **Categorize by severity**:
   - **CRITICAL**: Security vulnerabilities, data exposure
   - **HIGH**: Missing functionality, performance issues
   - **MEDIUM**: Incomplete features, missing tests
   - **LOW**: Documentation, code style

2. **Test security thoroughly**:
   - Can queries exceed depth limit? (CRITICAL)
   - Can complexity be bypassed? (CRITICAL)
   - Is introspection available in production? (HIGH)
   - Are errors exposing internals? (HIGH)

3. **Provide specific fixes**:
   ```markdown
   Issue: Query depth limit not enforced
   Severity: CRITICAL
   Fix: Add depth validation to schema builder:
   ```rust
   let schema = Schema::build(Query, Mutation, Subscription)
       .limit_depth(15)
       .finish();
   ```

## Review Decision Framework

### APPROVED
Grant approval when:
- All Done Criteria met
- Security controls enforced
- Performance acceptable
- Integration complete
- Only LOW severity issues

### CHANGES REQUIRED
Require changes when:
- Security controls missing or bypassable
- Core functionality not working
- Performance issues (N+1, slow queries)
- Integration broken
- Any CRITICAL or HIGH issues

## Testing GraphQL Security

Critical security scenarios that MUST be tested:

1. **Query Depth Attack**
   ```graphql
   # Create deeply nested query
   # Should be rejected at configured depth
   ```

2. **Query Complexity Attack**
   ```graphql
   # Request large result sets
   # Should be rejected over complexity limit
   ```

3. **Alias Bombing**
   ```graphql
   # Many aliases for same field
   # Should be limited
   ```

4. **Introspection in Production**
   ```graphql
   # __schema query
   # Should fail in production
   ```

## Final Review Checklist

Before submitting your review:
- [ ] Tested all GraphQL operations
- [ ] Verified security controls work
- [ ] Checked performance optimizations
- [ ] Validated error messages
- [ ] Tested WebSocket subscriptions
- [ ] Reviewed metrics output
- [ ] Made clear APPROVED/CHANGES REQUIRED decision
- [ ] Included specific remediation if needed

## Template for Review Submission

```markdown
# Phase 3 - Checkpoint [N] Review

**Reviewer**: [Your designation]
**Date**: [Current date]
**Implementation Agent**: [Agent who requested review]

## Review Summary
[2-3 sentences summarizing GraphQL implementation state]

## Detailed Findings
[Your complete review output for this checkpoint]

## Security Assessment
- Query limits: [Enforced/Issues found]
- Input validation: [Complete/Gaps noted]
- Authorization: [Working/Problems found]
- Error handling: [Secure/Leaks information]

## Performance Assessment
- N+1 prevention: [Working/Issues found]
- Query execution: [Fast/Slow - metrics]
- Subscription delivery: [Reliable/Issues]
- Resource usage: [Acceptable/Concerns]

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

Remember: GraphQL introduces unique security challenges. Be thorough in testing query limits, input validation, and authorization. The API's security depends on proper implementation of these controls.
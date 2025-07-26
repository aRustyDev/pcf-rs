# Phase 3 Review Plan - GraphQL Implementation

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 3 checkpoints. Phase 3 implements a complete GraphQL API with queries, mutations, and subscriptions, building upon the foundation established in Phases 1 & 2.

## Review Context

Phase 3 adds the GraphQL layer to the PCF-RS API, requiring careful review of:
- Schema design and type safety
- Security controls (depth/complexity limits)
- Performance optimizations (DataLoader, N+1 prevention)
- Real-time capabilities (WebSocket subscriptions)
- Integration with existing infrastructure

## Core Review Principles

### Test-Driven Development (TDD) Verification
At every checkpoint, MUST verify TDD practices by checking:
1. **GraphQL tests exist before resolvers**
2. **Security tests precede security implementation**
3. **Subscription tests written before WebSocket code**
4. **DataLoader tests before batching logic**
5. **All edge cases covered** - invalid queries, authorization failures, connection drops

### Documentation Standards
GraphQL code MUST have comprehensive documentation including:
1. **Schema documentation** - All types, fields, and arguments
2. **Resolver logic explained** - Why decisions were made
3. **Security controls documented** - Limits and their rationale
4. **WebSocket lifecycle** - Connection, subscription, cleanup
5. **Performance considerations** - DataLoader usage, query optimization

### Code Quality Requirements
1. **NO .unwrap() or .expect() in production code paths** - test code and compile-time constants MAY use these with justification
2. **All futures properly handled** - No forgotten awaits in resolvers
3. **Resource cleanup guaranteed** - WebSocket connections, subscriptions
4. **Metrics properly scoped** - Avoid high cardinality in GraphQL metrics
5. **Security controls enforced** - Depth/complexity limits active

## Review Process Flow

1. **Receive checkpoint artifacts**:
   - Checkpoint number and description
   - All code files created/modified
   - Test output showing TDD approach
   - Any questions in `api/.claude/.reviews/checkpoint-X-questions.md`

2. **Execute review checklist** - MUST complete ALL items before proceeding

3. **Test the GraphQL implementation** using provided test commands

4. **Document findings** in `api/.claude/.reviews/checkpoint-X-feedback.md`

5. **Write progress notes** in `api/.claude/.reviews/checkpoint-X-review-vY.md`

6. **Answer any questions** found in `api/.claude/.reviews/checkpoint-X-questions.md`

7. **Provide clear decision**:
   - APPROVED: All requirements met
   - APPROVED WITH CONDITIONS: Minor issues that can be fixed in parallel
   - CHANGES REQUIRED: Critical issues blocking progress

## Review Scope Requirements

**MANDATORY**: Reviews are scoped to ONLY:
- The current checkpoint being reviewed
- Previously completed checkpoints in this phase
- Integration with Phase 1 & 2 components

**DO NOT** review or suggest changes for future checkpoints.

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: GraphQL Foundation Review

**What You're Reviewing**: Schema setup, context, error mapping, and basic GraphQL infrastructure

**Key Specifications to Verify**:
- GraphQL schema builds successfully with type safety
- Context includes database and session properly
- Errors map correctly from AppError to GraphQL errors
- Security configurations in place (introspection, playground)
- Health query works as expected

**Required Tests** (MUST execute all and verify output):
```bash
# Test schema builds
cargo test graphql::tests::test_schema_builds_successfully

# Test introspection control
ENVIRONMENT=production cargo test test_introspection_disabled

# Verify error mapping
cargo test graphql::errors

# Check context access
cargo test context::tests
```

**Critical Code Reviews**:
- Verify schema builder configuration matches security requirements
- Check context propagation for all requests
- Ensure demo mode bypass is properly gated
- Validate error codes follow GraphQL conventions

**Review Checklist**:
```markdown
## Checkpoint 1 Review - GraphQL Foundation

### Schema Configuration
- [ ] GraphQL schema builds without errors
- [ ] Type registry properly initialized
- [ ] Depth limit set to 15 (configurable)
- [ ] Complexity limit set to 1000 (configurable)
- [ ] Introspection disabled in production

### Context Implementation
- [ ] Database service accessible in context
- [ ] Session properly extracted and stored
- [ ] Request ID generated for tracing
- [ ] Demo mode flag correctly set
- [ ] Context extension trait works

### Error Handling
- [ ] AppError maps to GraphQL errors correctly
- [ ] Error codes follow convention (UNAUTHENTICATED, NOT_FOUND, etc.)
- [ ] Field-level errors supported
- [ ] No internal details leaked in errors

### Security Setup
- [ ] Playground only available in demo mode
- [ ] Schema export only available in demo mode
- [ ] Production mode disables introspection
- [ ] Authentication bypass only in demo mode

### Integration Points
- [ ] Integrates with Phase 1 error types
- [ ] Uses Phase 2 database service
- [ ] Follows Phase 1 configuration patterns
- [ ] Respects existing health check system

### TDD Verification
- [ ] Schema tests written before implementation
- [ ] Error mapping tests comprehensive
- [ ] Security tests cover all modes
- [ ] Test structure follows Phase 1 patterns

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 2: Query Implementation Review

**What You're Reviewing**: All query resolvers, DataLoader implementation, and pagination

**Key Specifications to Verify**:
- All queries work correctly (note, notes, notesByAuthor)
- DataLoader prevents N+1 queries effectively
- Pagination follows Relay cursor specification
- Authorization checks on all queries
- Error handling is consistent

**Required Tests** (MUST execute all and verify output):
```bash
# Test individual note query
cargo test query_tests::test_note_by_id_query

# Test pagination
cargo test query_tests::test_notes_pagination

# Verify DataLoader batching
cargo test query_tests::test_notes_by_author_with_dataloader -- --nocapture
# Should see only 2 DB queries for 3 GraphQL queries

# Test authorization
cargo test query_authorization_tests
```

**Performance Verification**:
```bash
# Run N+1 query detection
cargo run --example detect_n_plus_one

# Benchmark query performance
cargo bench query_performance
```

**Review Checklist**:
```markdown
## Checkpoint 2 Review - Query Implementation

### Query Resolvers
- [ ] note(id) query returns correct data
- [ ] notes pagination works with cursors
- [ ] notesByAuthor filters correctly
- [ ] All queries require authentication
- [ ] Null handling is correct

### DataLoader Implementation
- [ ] AuthorNotesLoader batches requests
- [ ] No N+1 queries for nested data
- [ ] Proper error handling in loader
- [ ] Cache invalidation considered
- [ ] All keys return values (empty if none)

### Pagination
- [ ] Follows Relay Connection specification
- [ ] Cursors are opaque (base64 encoded)
- [ ] PageInfo accurate (hasNext/Previous)
- [ ] Limits enforced (max 100)
- [ ] Works with after/before cursors

### Authorization
- [ ] All queries check authentication
- [ ] Demo mode bypass works
- [ ] Proper error messages for unauthorized
- [ ] No data leakage on auth failures

### Performance
- [ ] DataLoader prevents N+1 queries
- [ ] Query timeouts configured
- [ ] Pagination limits reasonable
- [ ] No unnecessary database calls

### TDD Verification
- [ ] Query tests written first
- [ ] DataLoader tests verify batching
- [ ] Pagination tests comprehensive
- [ ] Edge cases tested

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: Mutation Implementation Review

**What You're Reviewing**: CRUD mutations, input validation, and authorization

**Key Specifications to Verify**:
- Create, Update, Delete mutations work correctly
- Input validation with Garde is comprehensive
- Authorization prevents unauthorized modifications
- Events are emitted for subscriptions
- Error messages are helpful

**Required Tests** (MUST execute all and verify output):
```bash
# Test create mutation
cargo test mutation_tests::test_create_note_mutation

# Test update authorization
cargo test mutation_tests::test_update_note_authorization

# Test input validation
cargo test mutation_validation_tests

# Test event emission
cargo test mutation_event_tests -- --nocapture
# Should see event logs
```

**Security Validation**:
- Input sanitization removes dangerous content
- Length limits enforced
- Authorization checks cannot be bypassed
- No SQL injection possible

**Review Checklist**:
```markdown
## Checkpoint 3 Review - Mutation Implementation

### Mutation Operations
- [ ] createNote creates with all fields
- [ ] updateNote modifies only provided fields
- [ ] deleteNote removes from database
- [ ] All mutations return updated data
- [ ] Timestamps updated correctly

### Input Validation
- [ ] GraphQL validators match Garde validators
- [ ] Title length 1-200 enforced
- [ ] Content length 1-10000 enforced
- [ ] Tags array max 10 items
- [ ] Individual tag length <= 50

### Authorization Checks
- [ ] Users can only update own notes
- [ ] Users can only delete own notes
- [ ] Create uses session user as author
- [ ] Admin bypass if implemented
- [ ] Clear error messages

### Event Broadcasting
- [ ] NoteCreated event on create
- [ ] NoteUpdated event on update
- [ ] NoteDeleted event on delete
- [ ] Events contain correct data
- [ ] No events if operation fails

### Data Sanitization
- [ ] Input strings trimmed
- [ ] Script tags prevented
- [ ] No XSS vulnerabilities
- [ ] Safe for database storage

### TDD Verification
- [ ] Mutation tests written first
- [ ] Authorization tests comprehensive
- [ ] Validation tests cover edge cases
- [ ] Event tests verify broadcasting

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 4: Subscription Implementation Review

**What You're Reviewing**: WebSocket support, subscription resolvers, event filtering

**Key Specifications to Verify**:
- WebSocket connections established properly
- Subscriptions receive real-time events
- Event filtering works correctly
- Connection limits enforced (max 1000)
- Proper cleanup on disconnect

**Required Tests** (MUST execute all and verify output):
```bash
# Test basic subscription
cargo test subscription_tests::test_note_created_subscription

# Test filtered subscription
cargo test subscription_tests::test_filtered_subscription

# Test connection limits
cargo test websocket_connection_limits

# Manual WebSocket test
websocat -t ws://localhost:8080/graphql/ws
# Send: {"type":"connection_init"}
# Should receive: {"type":"connection_ack"}
```

**WebSocket Lifecycle Tests**:
```bash
# Test connection lifecycle
./scripts/test-websocket-lifecycle.sh

# Monitor active connections
curl http://localhost:8080/metrics | grep subscription_connections
```

**Review Checklist**:
```markdown
## Checkpoint 4 Review - Subscription Implementation

### WebSocket Protocol
- [ ] Supports graphql-ws protocol
- [ ] Supports graphql-transport-ws protocol
- [ ] Connection init/ack works
- [ ] Ping/pong keepalive implemented
- [ ] Clean disconnect handling

### Subscription Resolvers
- [ ] noteCreated receives all creates
- [ ] noteUpdated receives all updates
- [ ] notesByAuthor filters by author
- [ ] Authorization enforced
- [ ] Stream terminates on error

### Event Broadcasting
- [ ] EventBroadcaster distributes events
- [ ] Subscriber count tracked
- [ ] No events sent if no subscribers
- [ ] Memory usage bounded
- [ ] Cleanup on drop

### Connection Management
- [ ] Connection limit enforced (1000)
- [ ] Connections tracked properly
- [ ] Resources cleaned on disconnect
- [ ] No memory leaks
- [ ] Metrics updated correctly

### Filtering Logic
- [ ] Author filtering accurate
- [ ] No events leak to wrong users
- [ ] Admin can subscribe to any
- [ ] Performance acceptable
- [ ] Edge cases handled

### TDD Verification
- [ ] Subscription tests use streams
- [ ] WebSocket tests comprehensive
- [ ] Connection limit tests work
- [ ] Cleanup tests verify resources

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 5: Security & Complete Integration Review

**What You're Reviewing**: Security controls, metrics, and full system integration

**Key Specifications to Verify**:
- Query depth limited to 15 levels
- Query complexity limited to 1000 points
- All metrics collected properly
- Full integration with Phase 1 & 2
- Verification script passes

**Required Tests** (MUST execute all and verify output):
```bash
# Run complete verification
./scripts/verify-phase-3.sh

# Test depth limiting
cargo test security_tests::test_query_depth_limit

# Test complexity limiting
cargo test security_tests::test_query_complexity_limit

# Verify metrics
curl http://localhost:8080/metrics | grep graphql_
# Should see all GraphQL metrics
```

**Security Penetration Tests**:
```bash
# Test malicious queries
./scripts/test-graphql-security.sh

# Test resource exhaustion
./scripts/test-query-limits.sh
```

**Review Checklist**:
```markdown
## Checkpoint 5 Review - Security & Integration

### Security Controls
- [ ] Depth limit enforced at 15 levels
- [ ] Complexity limit enforced at 1000
- [ ] Introspection disabled in production
- [ ] Playground disabled in production
- [ ] No security bypasses found

### Metrics Implementation
- [ ] graphql_request_duration_seconds collected
- [ ] graphql_request_total counted
- [ ] graphql_field_resolution_duration_seconds tracked
- [ ] Labels have low cardinality
- [ ] No PII in metrics

### Complete Integration
- [ ] All queries work end-to-end
- [ ] All mutations work end-to-end
- [ ] All subscriptions work end-to-end
- [ ] Phase 1 error handling maintained
- [ ] Phase 2 database integration solid

### Performance Validation
- [ ] Response times acceptable (<100ms p99)
- [ ] No memory leaks detected
- [ ] CPU usage reasonable
- [ ] WebSocket connections stable
- [ ] DataLoader prevents N+1

### Production Readiness
- [ ] All tests pass
- [ ] Documentation complete
- [ ] Logs structured properly
- [ ] Metrics enable monitoring
- [ ] No debug code remains

### TDD Summary
- [ ] Test coverage ≥80% overall
- [ ] Critical paths ≥95% coverage
- [ ] Integration tests comprehensive
- [ ] Security tests thorough
- [ ] No untested code paths

### Code Quality Final Check
- [ ] No .unwrap() or .expect() in production paths
- [ ] All TODOs addressed
- [ ] No debug prints
- [ ] Code formatted (cargo fmt)
- [ ] Linting passes (cargo clippy)

### Decision: [APPROVED FOR PHASE 4 / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 3 Status: [COMPLETE / INCOMPLETE]
```

## Issue Severity Definitions

**CRITICAL**: Blocks phase completion, MUST fix before approval
- Security vulnerabilities (injection, auth bypass)
- System crashes or panics
- Data loss or corruption
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

## How to Test GraphQL-Specific Scenarios

### Query Depth Testing
```bash
# Create nested query
cat << EOF | curl -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d @-
{
  "query": "{ notes { edges { node { author { notes { edges { node { id } } } } } } } }"
}
EOF
# Count nesting levels - should fail if >15
```

### DataLoader Verification
```bash
# Enable query logging
RUST_LOG=sqlx=debug cargo run

# Run query with multiple same-author requests
# Should see batched SQL queries, not individual ones
```

### WebSocket Connection Test
```bash
# Test subscription
websocat -t ws://localhost:8080/graphql/ws << EOF
{"id":"1","type":"connection_init"}
{"id":"2","type":"subscribe","payload":{"query":"subscription { noteCreated { id title } }"}}
EOF

# In another terminal, create a note
# Should see event in first terminal
```

## Common Issues to Watch For

### GraphQL-Specific Issues
1. **Nullable vs Non-Nullable**: Incorrect null handling in schema
2. **Resolver Panics**: Unhandled errors causing panics
3. **Infinite Recursion**: Circular type references
4. **Missing Context**: Context not properly propagated
5. **Event Leaks**: Subscriptions receiving wrong events

### Integration Issues
1. **Auth Middleware**: GraphQL bypassing auth checks
2. **Error Format**: GraphQL errors not matching spec
3. **Metrics Missing**: GraphQL operations not tracked
4. **Health Check**: GraphQL health not included
5. **Configuration**: GraphQL config not following patterns

## Review Decision Framework

### APPROVED
MUST grant approval ONLY when ALL of the following are met:
- All checklist items pass
- No CRITICAL or HIGH issues remain
- Tests achieve required coverage
- Security controls verified working
- Performance meets requirements

### APPROVED WITH CONDITIONS
MAY grant conditional approval when:
- All critical requirements met
- Only MEDIUM or LOW severity issues remain
- Clear remediation plan provided
- Timeline for fixes agreed upon

### CHANGES REQUIRED
MUST require changes when:
- Any CRITICAL issues found
- Multiple HIGH issues present
- Security vulnerabilities exist
- Core functionality broken
- Integration with Phase 1/2 fails

## Final Phase 3 Validation

Before approving Phase 3 completion:
1. Run complete GraphQL introspection (demo mode)
2. Execute all example queries/mutations/subscriptions
3. Verify metrics show expected values
4. Test error scenarios comprehensively
5. Validate security limits cannot be bypassed
6. Ensure smooth Phase 4 transition path

## Recovery Guidance for Reviewers

### When Tests Fail
1. Check if GraphQL server is running
2. Verify database is accessible
3. Ensure demo feature flag is set
4. Check for port conflicts

### When Security Tests Fail
1. Verify extensions are added to schema
2. Check depth/complexity calculation logic
3. Test with simpler queries first
4. Review security configuration

### When Performance is Poor
1. Check if DataLoader is properly configured
2. Verify database indexes exist
3. Look for missing async/await
4. Profile resolver execution times

Remember: Phase 3 establishes the API interface that clients will depend on. Be thorough in review, as schema changes after release are difficult.
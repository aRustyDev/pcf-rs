# Phase 6 Review Plan - Guidelines for Reviewing Agents

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 6 checkpoints. As a reviewing agent, you are responsible for ensuring the performance optimizations meet all specifications, achieve target metrics, and maintain code quality.

## MANDATORY Review Scope

**IMPORTANT**: You MUST limit your review to:
1. The current checkpoint being reviewed
2. Any previously completed checkpoints in this phase
3. Do NOT review or comment on future checkpoints

This ensures focused, relevant feedback without overwhelming the implementation agent.

## Your Responsibilities as Reviewer

1. **First, check for questions** in `api/.claude/.reviews/checkpoint-X-questions.md` and answer them in the same file
2. **Thoroughly examine all provided artifacts**
3. **Run performance benchmarks and verify results**
4. **Test N+1 query prevention with specific scenarios**
5. **Verify cache hit rates and isolation**
6. **Test timeout cascade behavior**
7. **Run load tests at target RPS**
8. **Profile for performance bottlenecks**
9. **Verify TDD practices were followed** (or documented exceptions for spikes)
10. **Check code documentation and comments** are comprehensive
11. **Look for code cleanliness** - no leftover stubs, TODOs, or test artifacts
12. **Verify junior dev resources were used** and check if they were helpful
13. **Write feedback** to `api/.claude/.reviews/checkpoint-X-feedback.md`
14. **Document your review process** in `api/.claude/.reviews/checkpoint-X-review-vY.md` (where Y is version number)
15. **Give clear approval, conditional approval, or rejection**

**Time Limits**: If unable to complete full review within 24 hours, provide preliminary feedback with timeline for completion.

## Core Review Principles

### Performance Target Verification
At every checkpoint, verify performance targets are met:
1. **P99 latency < 200ms** - Under normal load
2. **1000 RPS sustained** - With 99% success rate
3. **No N+1 queries** - In any code path
4. **Cache hit rate > 50%** - For common queries
5. **Linear CPU scaling** - Performance scales with cores

### DataLoader Verification
Ensure N+1 prevention is comprehensive:
1. **All relationships use DataLoader** - No direct queries in resolvers
2. **Batch sizes optimized** - Not too large (memory) or small (efficiency)
3. **Request scoping works** - Cache cleared between requests
4. **Metrics show batching** - Batch efficiency tracked
5. **No over-fetching** - Only requested data loaded

### Cache Security Verification
Check cache isolation and security:
1. **User data isolated** - No cross-user data leakage
2. **Cache keys secure** - Include user context
3. **TTLs appropriate** - Not too long for stale data
4. **Invalidation works** - Mutations clear affected entries
5. **Memory bounded** - Cache size limits enforced

### Test-Driven Development (TDD) Verification
Continue verifying TDD practices:
1. **Tests exist before implementation** - Check git history
2. **Performance tests comprehensive** - Load, stress, spike tests
3. **Benchmarks established** - Baseline measurements
4. **Edge cases covered** - Timeout scenarios, cache misses
5. **Code well documented** - All public functions have rustdoc comments
6. **Clean implementation** - No commented out code, test stubs, or TODO comments
7. **Junior resources utilized** - Evidence of consulting provided guides

## Junior Developer Resources

Direct the implementing agent to these guides when you find issues:
- **[DataLoader N+1 Tutorial](../../junior-dev-helper/dataloader-n1-tutorial.md)** - For N+1 query problems
- **[Caching Strategies Guide](../../junior-dev-helper/caching-strategies-guide.md)** - For cache isolation issues
- **[Performance Testing Tutorial](../../junior-dev-helper/performance-testing-tutorial.md)** - For inadequate load tests
- **[Performance Optimization Errors](../../junior-dev-helper/performance-optimization-errors.md)** - For common mistakes
- **[Timeout Management Guide](../../junior-dev-helper/timeout-management-guide.md)** - For timeout hierarchy issues

## Review Process

For each checkpoint review:

1. **Check for questions**: Read `api/.claude/.reviews/checkpoint-X-questions.md` and provide answers

2. **Receive from implementing agent**:
   - Link to Phase 6 REVIEW_PLAN.md
   - Link to Phase 6 WORK_PLAN.md
   - Specific checkpoint number
   - All artifacts listed for that checkpoint
   - Performance test results

3. **Perform the review** using checkpoint-specific checklist

4. **Run performance verification**:
   - Execute benchmarks
   - Run load tests
   - Check query patterns
   - Measure cache effectiveness

5. **Document your findings**:
   - Write feedback to `api/.claude/.reviews/checkpoint-X-feedback.md`
   - Save your review notes to `api/.claude/.reviews/checkpoint-X-review-vY.md`

6. **Provide clear decision**: APPROVED or CHANGES REQUIRED

7. **Stop and wait** for the implementing agent to address feedback before continuing

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: DataLoader Implementation Review

**What You're Reviewing**: N+1 query prevention with DataLoader (DO NOT review future checkpoints)

**Key Specifications to Verify**:
- DataLoader trait implemented correctly
- Batch loading works for all entities
- Request-scoped caching functional
- GraphQL context integration complete
- No N+1 queries in tests
- Code properly documented with rustdoc
- No leftover TODO comments or test stubs
- Evidence of using [DataLoader N+1 Tutorial](../../junior-dev-helper/dataloader-n1-tutorial.md)

**Required Tests**:
```bash
# Run N+1 detection tests
cargo test --test n_plus_one_detection

# Check specific resolver patterns
cargo test test_user_notes_relationship_batching
cargo test test_note_author_relationship_batching

# Verify batch metrics
curl http://localhost:8080/metrics | grep dataloader_batch

# Test with complex nested query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ 
      users(first: 100) { 
        id 
        name 
        notes { 
          id 
          title 
          tags { 
            name 
          } 
        } 
      } 
    }"
  }'

# Monitor database queries (should see batching)
# Enable query logging and count unique queries
```

**Batch Efficiency Verification**:
```bash
# Generate load with relationships
for i in {1..10}; do
  curl -X POST http://localhost:8080/graphql \
    -d '{"query":"{ users { notes { author { name } } } }"}'
done

# Check metrics for batch efficiency
curl http://localhost:8080/metrics | grep -E "dataloader_batch_size|dataloader_load_count"
# Batch efficiency = load_count / batch_count (should be > 5)
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 1 Review Results

### DataLoader Implementation
- [ ] Trait defined with proper types: [YES/NO]
- [ ] Batch loading logic correct: [YES/NO]
- [ ] Error handling comprehensive: [YES/NO]
- [ ] Configuration options available: [YES/NO]
  - [ ] Max batch size
  - [ ] Batch delay
  - [ ] Cache TTL

### N+1 Prevention
Tested Relationships:
- [ ] User -> Notes: [PREVENTED/STILL OCCURS/PARTIAL]
- [ ] Note -> Author: [PREVENTED/STILL OCCURS/PARTIAL]
- [ ] Note -> Tags: [PREVENTED/STILL OCCURS/PARTIAL]
- [ ] Custom relationships: [List any additional]

**For PARTIAL**: Explain which cases work and which don't

Query Analysis:
- [ ] Database query count for 100 users with notes: ___ (should be < 10)
- [ ] Batch sizes observed: min: ___, max: ___, avg: ___
- [ ] Cache hit rate within request: ___%

### Request Scoping
- [ ] Cache cleared between requests: [YES/NO]
- [ ] No data leakage between users: [YES/NO]
- [ ] Memory usage stable: [YES/NO]
- [ ] Concurrent request isolation: [YES/NO]

### Performance Impact
- [ ] Latency improvement: __% reduction
- [ ] Database load reduction: __% fewer queries
- [ ] Memory overhead: ___MB per request
- [ ] CPU overhead: __% increase

### Integration Testing
- [ ] Works with authorization: [YES/NO]
- [ ] Works with field-level permissions: [YES/NO]
- [ ] Error propagation correct: [YES/NO]
- [ ] Tracing spans created: [YES/NO]

### TDD Verification
- [ ] Tests written first: [YES/NO - evidence]
- [ ] Batch behavior tested: [YES/NO]
- [ ] Edge cases covered: [YES/NO]
- [ ] Performance benchmarks: [YES/NO]

### Code Quality
- [ ] All public functions have rustdoc comments: [YES/NO]
- [ ] No TODO comments remaining: [YES/NO]
- [ ] No commented out code: [YES/NO]
- [ ] No test stubs or debug prints: [YES/NO]

### Junior Developer Resources
- [ ] Evidence of consulting DataLoader tutorial: [YES/NO]
- [ ] Tutorial adequately covered the topic: [YES/NO]
- [ ] Any gaps in the tutorial: [List topics that should be added]

### Issues Found
[List with query examples showing problems]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]

[If APPROVED WITH CONDITIONS]
The implementing agent may proceed with the following conditions:
1. [Specific condition that must be met]
2. [Minor issues to address in next checkpoint]
3. Document completion in `api/.claude/.reviews/checkpoint-1-conditions-met.md`

### Required Files to Create
- [ ] Created `api/.claude/.reviews/checkpoint-1-feedback.md`
- [ ] Created `api/.claude/.reviews/checkpoint-1-review-v1.md`
- [ ] Answered questions in `api/.claude/.reviews/checkpoint-1-questions.md` (if any)
```

### 🛑 CHECKPOINT 2: Response Caching Review

**What You're Reviewing**: Query result caching with user isolation

**Key Specifications to Verify**:
- Cache key generation secure
- User isolation enforced
- TTL configuration works
- Invalidation strategy sound
- Hit rate meets targets

**Required Tests**:
```bash
# Test cache isolation
# As user1
curl -X POST http://localhost:8080/graphql \
  -H "Authorization: Bearer user1_token" \
  -d '{"query":"{ notes { id title } }"}'

# As user2 (should not see user1's cached data)
curl -X POST http://localhost:8080/graphql \
  -H "Authorization: Bearer user2_token" \
  -d '{"query":"{ notes { id title } }"}'

# Test cache effectiveness
# Run same query 100 times
for i in {1..100}; do
  curl -X POST http://localhost:8080/graphql \
    -H "Authorization: Bearer test_token" \
    -d '{"query":"{ users { id name } }"}' \
    -w "Time: %{time_total}s\n" -o /dev/null -s
done

# Check cache metrics
curl http://localhost:8080/metrics | grep -E "response_cache_hit|response_cache_miss"
# Hit rate should be > 50% after warmup
```

**Invalidation Testing**:
```bash
# Create a note
curl -X POST http://localhost:8080/graphql \
  -d '{"query":"mutation { createNote(input: {title: \"Test\"}) { id } }"}'

# Query should now return fresh data (cache invalidated)
curl -X POST http://localhost:8080/graphql \
  -d '{"query":"{ notes { id title } }"}'

# Verify specific invalidation patterns
# Only affected queries should be invalidated
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 2 Review Results

### Cache Implementation
- [ ] LRU eviction works: [YES/NO]
- [ ] Memory limits enforced: [YES/NO - limit: ___MB]
- [ ] TTL configuration: [YES/NO - default: ___s]
- [ ] Metrics exported: [YES/NO]

### Cache Key Generation
- [ ] Query normalization works: [YES/NO]
- [ ] Variables included in key: [YES/NO]
- [ ] User context included: [YES/NO]
- [ ] No sensitive data in keys: [YES/NO]
- [ ] Key collision tested: [YES/NO - probability: ___]

### User Isolation
- [ ] Different users get different results: [YES/NO]
- [ ] No cache sharing between users: [YES/NO]
- [ ] Admin queries not cached: [YES/NO]
- [ ] Anonymous queries handled: [YES/NO]

### Cache Performance
- [ ] Hit rate after warmup: __% (target > 50%)
- [ ] Cache lookup time: ___μs (target < 100μs)
- [ ] Memory usage per entry: ___KB
- [ ] Total cache size: ___MB
- [ ] Eviction rate: ___/minute

### Invalidation Strategy
Mutation Effects:
- [ ] createNote invalidates notes queries: [YES/NO]
- [ ] updateUser invalidates only that user: [YES/NO]
- [ ] deleteNote invalidates appropriately: [YES/NO]
- [ ] Bulk operations handled: [YES/NO]

Invalidation Scope:
- [ ] Over-invalidation avoided: [YES/NO]
- [ ] Pattern matching works: [YES/NO]
- [ ] Related queries cleared: [YES/NO]

### Cache Warming
- [ ] Startup warming implemented: [YES/NO]
- [ ] Common queries pre-cached: [YES/NO]
- [ ] Background refresh works: [YES/NO]

### TDD Verification
- [ ] Cache tests comprehensive: [YES/NO]
- [ ] Isolation tests exist: [YES/NO]
- [ ] Invalidation tests complete: [YES/NO]
- [ ] Performance benchmarks: [YES/NO]

### Issues Found
[Specific examples of cache problems]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: Timeout Implementation Review

**What You're Reviewing**: Hierarchical timeout management

**Key Specifications to Verify**:
- Timeout hierarchy correct (HTTP > GraphQL > DB)
- Timeouts cascade properly
- No hanging requests
- Graceful degradation
- Clear error messages

**Required Tests**:
```bash
# Test timeout cascade
# Create slow query that will timeout
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ slowQuery(delay: 35) }"}' \
  -m 40

# Should timeout at HTTP level (30s) with proper error

# Test timeout propagation
# Monitor logs to see timeout budget passed down
RUST_LOG=debug cargo run

# Make request and check logs for:
# - HTTP timeout: 30s
# - GraphQL timeout: 25s  
# - Database timeout: 20s

# Test multiple concurrent slow requests
for i in {1..10}; do
  curl -X POST http://localhost:8080/graphql \
    -d '{"query":"{ slowQuery(delay: 25) }"}' &
done
wait

# All should timeout cleanly without hanging
```

**Timeout Budget Verification**:
```bash
# Test remaining budget calculation
# Use debug endpoint to check timeout context
curl http://localhost:8080/debug/timeout-context

# Test minimum timeout enforcement
# Even with 1s remaining, DB should get minimum timeout
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 3 Review Results

### Timeout Configuration
- [ ] HTTP timeout: 30s configured: [YES/NO]
- [ ] GraphQL timeout: 25s configured: [YES/NO]
- [ ] Database timeout: 20s configured: [YES/NO]
- [ ] Cascade order correct: [YES/NO]

### Timeout Propagation
- [ ] Context passed through layers: [YES/NO]
- [ ] Budget calculation correct: [YES/NO]
- [ ] Safety buffer maintained: [YES/NO]
- [ ] Minimum timeouts enforced: [YES/NO]

### Timeout Behavior
Request Handling:
- [ ] Requests timeout cleanly: [YES/NO]
- [ ] No goroutine/task leaks: [YES/NO]
- [ ] Resources cleaned up: [YES/NO]
- [ ] Connections returned to pool: [YES/NO]

Error Responses:
- [ ] HTTP 408 returned: [YES/NO]
- [ ] Error message clear: [YES/NO]
- [ ] No internal details leaked: [YES/NO]
- [ ] Trace ID included: [YES/NO]

### Edge Cases
- [ ] Zero remaining budget handled: [YES/NO]
- [ ] Clock skew handled: [YES/NO]
- [ ] Concurrent timeouts work: [YES/NO]
- [ ] Graceful shutdown works: [YES/NO]

### Performance Impact
- [ ] Timeout check overhead: ___μs
- [ ] Context creation cost: ___μs
- [ ] No busy waiting: [YES/NO]
- [ ] Timer efficiency: [YES/NO]

### Integration
- [ ] Works with DataLoader: [YES/NO]
- [ ] Works with caching: [YES/NO]
- [ ] Tracing includes timeout info: [YES/NO]
- [ ] Metrics track timeouts: [YES/NO]

### TDD Verification
- [ ] Timeout tests exist: [YES/NO]
- [ ] Cascade tests complete: [YES/NO]
- [ ] Edge cases tested: [YES/NO]
- [ ] Load tests include timeouts: [YES/NO]

### Issues Found
[Examples of timeout failures or hangs]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 4: Complete Phase 6 System Review

**What You're Reviewing**: Full performance optimization system

**Key Specifications to Verify**:
- All performance targets met
- System stable under load
- Optimizations work together
- Monitoring in place
- Documentation complete

**Required Load Tests**:
```bash
# Run sustained load test
./scripts/load-test.sh --rps 1000 --duration 300s

# During test, monitor:
# 1. Response times
watch -n 1 'curl -s http://localhost:8080/metrics | grep -E "graphql_request_duration_seconds{quantile=\"0.99\"}"'

# 2. Error rate
watch -n 1 'curl -s http://localhost:8080/metrics | grep -E "graphql_errors_total"'

# 3. System resources
htop

# 4. Database connections
# Monitor connection pool metrics

# Run stress test (beyond target)
./scripts/load-test.sh --rps 1500 --duration 60s
# Should degrade gracefully
```

**N+1 Query Verification**:
```bash
# Run N+1 detection across all queries
./scripts/detect-n-plus-one.sh

# Test specific problematic patterns
# 1. Deep nesting
curl -X POST http://localhost:8080/graphql -d '{
  "query": "{
    users {
      notes {
        author {
          notes {
            tags {
              notes {
                author {
                  name
                }
              }
            }
          }
        }
      }
    }
  }"
}'

# Check query count is still reasonable
```

**Your Review Output Should Include**:
```markdown
## Phase 6 Complete System Review

### Done Criteria Verification
- [ ] No N+1 queries detected in tests: [YES/NO]
- [ ] P99 response times under 200ms: [YES/NO - actual: ___ms]
- [ ] Timeouts cascade properly without hanging: [YES/NO]
- [ ] Cache hit rate > 50% for common queries: [YES/NO - rate: __%]
- [ ] Load tests pass at 1000 RPS: [YES/NO - actual: ___RPS]

### Load Test Results
Test Configuration:
- Duration: 300 seconds
- Target RPS: 1000
- Connections: 100
- Query mix: [describe distribution]

Results:
- [ ] Achieved RPS: ___ (target: 1000)
- [ ] Success rate: ___% (target: 99%)
- [ ] P50 latency: ___ms
- [ ] P95 latency: ___ms
- [ ] P99 latency: ___ms (target: < 200ms)
- [ ] Max latency: ___ms

Error Breakdown:
- [ ] Timeouts: ___
- [ ] 5xx errors: ___
- [ ] Connection errors: ___

### Resource Usage
Under Load:
- [ ] CPU usage: ___% (should scale linearly)
- [ ] Memory usage: ___MB (should be stable)
- [ ] Connection pool usage: ___% 
- [ ] File descriptors: ___ (well below limit)
- [ ] Connection retry behavior verified: [YES/NO]
  - [ ] Initial retry: 1s [YES/NO]
  - [ ] Exponential backoff: [YES/NO]
  - [ ] Max retry interval: 60s [YES/NO]

Scaling Behavior:
- [ ] Performance scales with CPU cores: [YES/NO]
- [ ] No resource leaks detected: [YES/NO]
- [ ] Memory stable over time: [YES/NO]

### Component Integration
- [ ] DataLoader + Caching: [WORKS/CONFLICTS]
- [ ] Caching + Timeouts: [WORKS/CONFLICTS]
- [ ] All + Authorization: [WORKS/CONFLICTS]
- [ ] All + Observability: [WORKS/CONFLICTS]

### Cache Effectiveness
Common Queries:
- [ ] User list: __% hit rate
- [ ] Note queries: __% hit rate
- [ ] Nested queries: __% hit rate
- [ ] Overall: __% hit rate (target > 50%)

### N+1 Prevention
Verified Patterns:
- [ ] Simple relationships: [PREVENTED/FOUND N+1]
- [ ] Nested relationships: [PREVENTED/FOUND N+1]
- [ ] Conditional loading: [PREVENTED/FOUND N+1]
- [ ] Pagination + relations: [PREVENTED/FOUND N+1]

### Monitoring & Alerts
- [ ] Performance dashboards created: [YES/NO]
- [ ] SLO alerts configured: [YES/NO]
- [ ] Runbooks documented: [YES/NO]
- [ ] Metrics exported: [YES/NO]
- [ ] Metric cardinality < 1000 per name: [YES/NO]
  - [ ] If NO, list problematic metrics: ___________

### Documentation Review
- [ ] Performance tuning guide: [YES/NO]
- [ ] Configuration reference: [YES/NO]
- [ ] Troubleshooting guide: [YES/NO]
- [ ] Architecture diagrams: [YES/NO]

### Outstanding Issues
[Any issues for Phase 7 consideration]

### Performance Recommendations
[Suggestions for future optimization]

### Decision: [APPROVED FOR PHASE 7 / APPROVED WITH CONDITIONS / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 6 Status: [COMPLETE / INCOMPLETE]
```

## How to Handle Issues

When you find issues during review:

1. **Categorize by severity**:
   - **CRITICAL**: Fails load test, N+1 queries present
   - **HIGH**: Misses performance targets, cache leaks
   - **MEDIUM**: Suboptimal but functional
   - **LOW**: Documentation, minor improvements

2. **Test performance thoroughly**:
   - Does it meet 1000 RPS? (CRITICAL)
   - Is P99 < 200ms? (CRITICAL)
   - Are N+1 queries eliminated? (HIGH)
   - Is cache effective? (HIGH)

3. **Provide specific fixes**:
   ```markdown
   Issue: N+1 queries in user->notes relationship
   Severity: HIGH
   Fix: Add DataLoader to notes resolver:
   ```rust
   async fn notes(&self, ctx: &Context<'_>) -> Result<Vec<Note>> {
       let loader = ctx.data::<DataLoader<NoteLoader>>()?;
       loader.load_many_by_user(self.id).await
   }
   ```

## Review Decision Framework

### APPROVED
Grant approval when:
- All Done Criteria met
- Load test passes at 1000 RPS
- P99 latency under 200ms
- No N+1 queries found
- Only LOW severity issues

### CHANGES REQUIRED
Require changes when:
- Load test fails
- Performance targets missed
- N+1 queries detected
- Memory leaks present
- Any CRITICAL or HIGH issues

## Performance Testing Guide

Critical performance tests to run:

1. **Baseline Performance**
   ```bash
   # Single request latency
   ab -n 1000 -c 1 http://localhost:8080/graphql
   ```

2. **Concurrent Load**
   ```bash
   # Target load
   ab -n 10000 -c 100 http://localhost:8080/graphql
   ```

3. **Stress Test**
   ```bash
   # Beyond target to find limits
   ab -n 20000 -c 200 http://localhost:8080/graphql
   ```

4. **N+1 Detection**
   ```bash
   # Enable query logging in your database
   export DATABASE_LOG_QUERIES=true
   
   # Run test with relationships
   curl -X POST http://localhost:8080/graphql \
     -H "Content-Type: application/json" \
     -d '{"query":"{ users(first: 10) { notes { author { name } } } }"}' \
     2>&1 | tee query.log
   
   # Count unique queries (should be ~3, not 30+)
   grep "SELECT" query.log | sort | uniq | wc -l
   ```

5. **Metric Cardinality Verification**
   ```bash
   # Check cardinality per metric
   for metric in $(curl -s http://localhost:8080/metrics | grep -E "^graphql_" | cut -d'{' -f1 | sort | uniq); do
     count=$(curl -s http://localhost:8080/metrics | grep "^$metric" | wc -l)
     echo "$metric: $count labels"
     if [ $count -gt 1000 ]; then
       echo "  WARNING: Exceeds cardinality limit!"
     fi
   done
   ```

6. **Connection Pool Health Check**
   ```bash
   # Monitor pool metrics during load
   watch -n 1 'curl -s http://localhost:8080/metrics | grep -E "db_pool_|db_connection_retries"'
   
   # Simulate connection failures
   docker stop postgres_container
   sleep 5
   docker start postgres_container
   
   # Verify retry behavior matches SPEC.md (1s, 2s, 4s, ... up to 60s)
   ```

## Final Review Checklist

Before submitting your review:
- [ ] Ran all performance tests
- [ ] Verified no N+1 queries
- [ ] Checked cache effectiveness
- [ ] Tested timeout behavior
- [ ] Ran sustained load test
- [ ] Profiled for bottlenecks
- [ ] Made clear APPROVED/CHANGES REQUIRED decision
- [ ] Included specific remediation if needed

## Template for Review Submission

```markdown
# Phase 6 - Checkpoint [N] Review

**Reviewer**: [Your designation]
**Date**: [Current date]
**Implementation Agent**: [Agent who requested review]

## Review Summary
[2-3 sentences summarizing performance optimization state]

## Detailed Findings
[Your complete review output for this checkpoint]

## Performance Assessment
- Target RPS: 1000 / Achieved: ___
- Target P99: 200ms / Achieved: ___ms
- N+1 Queries: [Found/None]
- Cache Hit Rate: __% (target > 50%)

## Resource Usage
- CPU efficiency: [Good/Poor]
- Memory stability: [Stable/Leaking]
- Connection pools: [Optimized/Issues]

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

## Feedback File Template

When creating `api/.claude/.reviews/checkpoint-X-feedback.md`:

```markdown
# Checkpoint X Feedback

## Overall Assessment
[Brief summary of the checkpoint implementation quality]

## Strengths
- [What was done well]
- [Good practices observed]

## Issues Requiring Attention
### High Priority
1. [Critical issue with specific location and suggested fix]

### Medium Priority
1. [Important but not blocking issue]

### Low Priority
1. [Nice to have improvements]

## Code Quality Observations
- Documentation: [Adequate/Needs improvement]
- Test Coverage: [Comprehensive/Gaps noted]
- Code Cleanliness: [Clean/Issues found]

## Junior Developer Resources Assessment
- Resources Used: [List which guides were consulted]
- Effectiveness: [How helpful were they]
- Gaps Identified: [What additional guidance would help]

## Recommendations
1. [Specific actionable recommendations]

## Questions Answered
[If any questions were in checkpoint-X-questions.md, note that you answered them]
```

## Review Notes File Template

When creating `api/.claude/.reviews/checkpoint-X-review-vY.md`:

```markdown
# Checkpoint X Review Notes - Version Y

## Review Process
1. [Step-by-step what you reviewed]
2. [Tests you ran]
3. [Metrics you collected]

## Test Results
### Performance Tests
- [Specific test]: [Result]

### Code Analysis
- Lines of code reviewed: ___
- Test coverage: ___%
- Cyclomatic complexity: ___

## Time Spent
- Code review: ___ minutes
- Testing: ___ minutes
- Documentation: ___ minutes

## Tools Used
- [List any tools, scripts, or commands used]

## Raw Test Output
[Include relevant test output for future reference]
```

Remember: Performance is critical for user experience. Be thorough in testing all optimization paths and ensuring targets are met under realistic load.
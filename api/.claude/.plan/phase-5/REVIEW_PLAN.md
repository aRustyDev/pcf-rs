# Phase 5 Review Plan - Guidelines for Reviewing Agents

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 5 checkpoints. As a reviewing agent, you are responsible for ensuring the observability implementation meets all specifications, maintains security standards, and minimizes performance impact.

## Your Responsibilities as Reviewer

1. **Thoroughly examine all provided artifacts**
2. **Test metrics endpoint and verify Prometheus format**
3. **Verify cardinality controls are working**
4. **Check log sanitization for security**
5. **Validate distributed tracing implementation**
6. **Measure performance overhead**
7. **Test Grafana dashboards functionality**
8. **Verify TDD practices were followed**
9. **Provide explicit feedback on findings**
10. **Give clear approval or rejection**

## Core Review Principles

### Cardinality Control Verification
At every checkpoint, verify cardinality limits are enforced:
1. **Operation name limiting** - Max 50 unique operations
2. **Label bucketing** - Status codes grouped (2xx, 3xx, etc.)
3. **Hash-based distribution** - High cardinality fields bucketed
4. **Dynamic monitoring** - Alerts on cardinality growth
5. **"other" fallback** - Overflow handled gracefully

### Security Verification
Ensure no sensitive data is exposed:
1. **No PII in metrics** - User IDs, emails never as labels
2. **Log sanitization works** - All sensitive fields redacted
3. **Metrics endpoint secured** - IP allowlist or network restriction
4. **Trace sampling safe** - No sensitive data in spans
5. **Error messages generic** - No internal details leaked

### Performance Verification
Check observability overhead is acceptable:
1. **Metrics collection < 1% overhead** - Use benchmarks
2. **Logging non-blocking** - Async writers used
3. **Trace sampling configured** - Not 100% in production
4. **No memory leaks** - Stable under load
5. **Histogram buckets optimized** - Reasonable bucket sizes

### Test-Driven Development (TDD) Verification
Continue verifying TDD practices:
1. **Tests exist before implementation** - Check git history
2. **Tests fail first, then pass** - Red-Green-Refactor
3. **Tests cover edge cases** - Cardinality limits, sanitization
4. **Performance tests included** - Overhead measurements

## Review Process

For each checkpoint review:

1. **Receive from implementing agent**:
   - Link to Phase 5 REVIEW_PLAN.md
   - Link to Phase 5 WORK_PLAN.md
   - Specific checkpoint number
   - All artifacts listed for that checkpoint
   - Evidence of TDD approach

2. **Perform the review** using checkpoint-specific checklist

3. **Test critical observability features**:
   - Metrics endpoint returns valid Prometheus format
   - Cardinality limits prevent explosion
   - Logs are properly sanitized
   - Traces propagate correctly
   - Performance overhead is acceptable

4. **Document your findings** in structured format

5. **Provide clear decision**: APPROVED or CHANGES REQUIRED

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: Metrics Implementation Review

**What You're Reviewing**: Prometheus metrics and cardinality control

**Key Specifications to Verify**:
- /metrics endpoint returns Prometheus format
- All required metrics implemented
- Cardinality limiter prevents explosion
- No sensitive data in labels
- Performance sampling works

**Required Tests**:
```bash
# Test metrics endpoint
curl http://localhost:8080/metrics | promtool check metrics

# Verify cardinality limiting
# Make requests with 100 different operation names
for i in {1..100}; do
  curl -X POST http://localhost:8080/graphql \
    -H "Content-Type: application/json" \
    -d "{\"query\":\"{ operation_$i { id } }\"}"
done

# Check metrics - should see max 50 operations + "other"
curl http://localhost:8080/metrics | grep graphql_request_total | wc -l

# Test metric values
curl http://localhost:8080/metrics | grep -E "graphql_request_total|graphql_request_duration_seconds"
```

**Cardinality Verification**:
```bash
# Calculate total cardinality
curl http://localhost:8080/metrics | grep -E "^[^#]" | wc -l

# Check specific metric cardinality
curl http://localhost:8080/metrics | grep graphql_request_total | sort | uniq | wc -l
# Should not exceed: 3 types × 50 operations × 2 statuses = 300

# Verify bucketing works
# Make requests with various status codes
for code in 200 201 400 404 500 503; do
  # Simulate response with status code
done
# Check metrics show bucketed values (2xx, 4xx, 5xx)
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 1 Review Results

### Metrics Endpoint
- [ ] Endpoint accessible at /metrics: [YES/NO]
- [ ] Valid Prometheus format: [YES/NO - promtool output]
- [ ] All required metrics present: [YES/NO - list missing]
- [ ] Global labels correct: [YES/NO - service, environment, version]

### Cardinality Control
- [ ] Operation limiter works: [YES/NO - tested with __ operations]
- [ ] "other" label used for overflow: [YES/NO]
- [ ] Status code bucketing: [YES/NO - buckets: ___]
- [ ] Total cardinality within limits: [YES/NO - count: ___]
- [ ] Warning logs for exceeded limits: [YES/NO]

### Required Metrics
GraphQL Metrics:
- [ ] graphql_request_total: [YES/NO]
- [ ] graphql_request_duration_seconds: [YES/NO]
- [ ] graphql_errors_total: [YES/NO]
- [ ] graphql_active_subscriptions: [YES/NO]
- [ ] graphql_field_resolution_duration_seconds: [YES/NO]

HTTP Metrics:
- [ ] http_request_total: [YES/NO]
- [ ] http_request_duration_seconds: [YES/NO]

Database Metrics:
- [ ] database_connection_pool_size: [YES/NO]
- [ ] database_query_total: [YES/NO]
- [ ] database_query_duration_seconds: [YES/NO]

System Metrics:
- [ ] process_open_fds: [YES/NO]
- [ ] process_resident_memory_bytes: [YES/NO]

### Security Review
- [ ] No user IDs in labels: [YES/NO]
- [ ] No email addresses in labels: [YES/NO]
- [ ] No sensitive paths exposed: [YES/NO]
- [ ] Metrics endpoint can be restricted: [YES/NO]

### Performance Impact
- [ ] Metric collection overhead: __% (target < 1%)
- [ ] Memory usage stable: [YES/NO]
- [ ] No blocking operations: [YES/NO]
- [ ] Sampling implemented for expensive metrics: [YES/NO]

### TDD Verification
- [ ] Tests written first: [YES/NO - evidence]
- [ ] Cardinality tests comprehensive: [YES/NO]
- [ ] Performance tests included: [YES/NO]
- [ ] Edge cases covered: [YES/NO]

### Issues Found
[List with severity and fixes]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 2: Logging Implementation Review

**What You're Reviewing**: Structured logging with sanitization

**Key Specifications to Verify**:
- JSON format in production
- All sensitive data sanitized
- Trace IDs in every log
- Performance acceptable
- Log levels properly configured

**Required Tests**:
```bash
# Test log output format
RUST_LOG=debug cargo run &
sleep 5

# Make request to generate logs
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ user(id: \"user_123\") { email } }"}'

# Check log sanitization
tail -n 50 logs.json | jq . | grep -E "user_123|email"
# Should see <REDACTED> or <USER_ID> instead of actual values

# Verify trace ID propagation
tail -n 50 logs.json | jq '.trace_id' | sort | uniq -c
# All logs for one request should have same trace_id

# Test different log levels
RUST_LOG=info cargo run
# Should not see debug logs

RUST_LOG=pcf_api=debug,tower_http=info cargo run
# Should see module-specific levels
```

**Security Verification**:
```bash
# Test various sensitive patterns
curl -X POST http://localhost:8080/graphql \
  -d '{"query":"{ user(email: \"test@example.com\", password: \"secret123\") { id } }"}'

# Check logs don't contain:
tail -n 100 logs.json | grep -E "test@example.com|secret123"
# Should return nothing

# Verify sanitization rules
tail -n 100 logs.json | jq . | grep -E "password|email|token|key"
# Should only see <REDACTED> values
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 2 Review Results

### Log Format
- [ ] JSON format in production: [YES/NO]
- [ ] Pretty format in development: [YES/NO]
- [ ] All required fields present: [YES/NO]
  - [ ] timestamp
  - [ ] level
  - [ ] message
  - [ ] trace_id
  - [ ] span_id
  - [ ] target (module)

### Sanitization
- [ ] User IDs sanitized: [YES/NO - pattern: ___]
- [ ] Emails sanitized: [YES/NO]
- [ ] Passwords never logged: [YES/NO]
- [ ] Tokens/keys sanitized: [YES/NO]
- [ ] Custom patterns work: [YES/NO]

### Trace Correlation
- [ ] Trace ID in every log: [YES/NO]
- [ ] Consistent within request: [YES/NO]
- [ ] Format valid: [YES/NO - example: ___]
- [ ] Propagates to child spans: [YES/NO]

### Performance
- [ ] Non-blocking logging: [YES/NO]
- [ ] Async writer used: [YES/NO]
- [ ] Buffer size appropriate: [YES/NO - size: ___]
- [ ] No log loss under load: [YES/NO]

### Configuration
- [ ] RUST_LOG env var works: [YES/NO]
- [ ] Module-level control: [YES/NO]
- [ ] Config file override: [YES/NO]
- [ ] Runtime level changes: [YES/NO - if applicable]

### TDD Verification
- [ ] Sanitization tests complete: [YES/NO]
- [ ] Format tests for both modes: [YES/NO]
- [ ] Performance tests exist: [YES/NO]
- [ ] Trace ID tests: [YES/NO]

### Issues Found
[Detailed findings with examples]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: Tracing Implementation Review

**What You're Reviewing**: OpenTelemetry distributed tracing

**Key Specifications to Verify**:
- Spans created for all operations
- Context propagation works
- External services traced
- Sampling configured
- Performance overhead acceptable

**Required Tests**:
```bash
# Start OTLP collector (or Jaeger)
docker run -d \
  -p 4317:4317 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest

# Run application with tracing
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 cargo run

# Make various requests
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ notes { id title author } }"}'

# Check Jaeger UI
open http://localhost:16686
# Should see traces with proper hierarchy

# Test trace propagation to external services
# Make request that calls database/auth
# Verify spans show full call chain
```

**Span Verification**:
```bash
# Check span attributes
# In Jaeger UI, verify spans contain:
# - operation.type (query/mutation)
# - operation.name
# - user.id (if authenticated)
# - db.operation (for database calls)
# - http.method, http.status_code

# Test sampling
# Make 100 requests
for i in {1..100}; do
  curl -X POST http://localhost:8080/graphql \
    -d '{"query":"{ health }"}'
done

# Check trace count in Jaeger
# With 10% sampling, should see ~10 traces
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 3 Review Results

### OpenTelemetry Setup
- [ ] OTLP exporter configured: [YES/NO]
- [ ] Service name set correctly: [YES/NO - name: ___]
- [ ] Resource attributes complete: [YES/NO]
- [ ] Batch processor configured: [YES/NO]

### Span Creation
GraphQL Operations:
- [ ] Query spans created: [YES/NO]
- [ ] Mutation spans created: [YES/NO]
- [ ] Subscription spans created: [YES/NO]
- [ ] Field resolver spans: [YES/NO - for slow fields]

Database Operations:
- [ ] Database spans created: [YES/NO]
- [ ] Proper parent-child relationship: [YES/NO]
- [ ] Connection pool instrumented: [YES/NO]

External Services:
- [ ] HTTP client instrumented: [YES/NO]
- [ ] SpiceDB calls traced: [YES/NO]
- [ ] Cache operations traced: [YES/NO]

### Context Propagation
- [ ] Trace context in headers: [YES/NO]
- [ ] W3C TraceContext format: [YES/NO]
- [ ] Propagates through async: [YES/NO]
- [ ] Works across service boundaries: [YES/NO]

### Sampling
- [ ] Sampling rate configured: [YES/NO - rate: ___]
- [ ] TraceIdRatioBased sampler: [YES/NO]
- [ ] Head-based sampling: [YES/NO]
- [ ] No data loss for sampled traces: [YES/NO]

### Performance
- [ ] Tracing overhead: __% (target < 3%)
- [ ] Span creation non-blocking: [YES/NO]
- [ ] Batch export working: [YES/NO]
- [ ] Memory usage stable: [YES/NO]

### Span Attributes
- [ ] No sensitive data: [YES/NO]
- [ ] Consistent naming: [YES/NO]
- [ ] Semantic conventions followed: [YES/NO]
- [ ] Custom attributes documented: [YES/NO]

### TDD Verification
- [ ] Span creation tests: [YES/NO]
- [ ] Context propagation tests: [YES/NO]
- [ ] Sampling tests: [YES/NO]
- [ ] Integration tests: [YES/NO]

### Issues Found
[List with trace examples]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 4: Complete Phase 5 System Review

**What You're Reviewing**: Full observability integration

**Key Specifications to Verify**:
- All components work together
- Dashboards functional
- Alerts configured
- Performance acceptable
- Documentation complete

**Required Integration Tests**:
```bash
# Run load test with observability
./scripts/load-test.sh

# During load test, verify:
# 1. Metrics update in real-time
curl http://localhost:8080/metrics | grep graphql_request_total

# 2. Logs flow without drops
tail -f logs.json | jq '.level'

# 3. Traces complete without errors
# Check Jaeger for complete traces

# 4. No memory leaks
ps aux | grep pcf-api
# Monitor RSS over time

# Test Grafana dashboards
docker run -d -p 3000:3000 grafana/grafana
# Import dashboards and verify data flows
```

**Dashboard Verification**:
```bash
# Check each dashboard:
# 1. Overview Dashboard
#    - Request rate graph
#    - Error rate graph
#    - P95 latency graph
#    - Active connections

# 2. GraphQL Dashboard
#    - Operations by type
#    - Slow queries
#    - Error breakdown
#    - Field resolution times

# 3. System Dashboard
#    - CPU usage
#    - Memory usage
#    - File descriptors
#    - GC metrics

# Verify alerts in Prometheus
# Check alert rules are loaded
curl http://localhost:9090/api/v1/rules
```

**Your Review Output Should Include**:
```markdown
## Phase 5 Complete System Review

### Done Criteria Verification
- [ ] /metrics endpoint returns valid Prometheus format: [YES/NO]
- [ ] All operations emit structured logs with trace IDs: [YES/NO]
- [ ] Distributed tracing spans created for all operations: [YES/NO]
- [ ] No sensitive data in logs: [YES/NO]
- [ ] Monitoring dashboards created: [YES/NO]

### Integration Testing
Metrics + Logs:
- [ ] Metrics correlate with logs: [YES/NO]
- [ ] Same trace_id in both: [YES/NO]
- [ ] Timing matches: [YES/NO]

Metrics + Traces:
- [ ] Span duration matches metrics: [YES/NO]
- [ ] Error counts align: [YES/NO]
- [ ] Operation names consistent: [YES/NO]

Logs + Traces:
- [ ] Log entries appear in trace: [YES/NO]
- [ ] Error logs match span errors: [YES/NO]
- [ ] Context properly attached: [YES/NO]

### Performance Analysis
- [ ] Total overhead: __% (target < 5%)
  - [ ] Metrics: __% 
  - [ ] Logging: __%
  - [ ] Tracing: __%
- [ ] Memory usage increase: __MB
- [ ] CPU usage increase: __%
- [ ] No goroutine/task leaks: [YES/NO]
- [ ] Stable under load: [YES/NO]

### Cardinality Analysis
- [ ] Total metrics: ___ (target < 10K)
- [ ] Largest metric cardinality: ___
- [ ] Growth rate acceptable: [YES/NO]
- [ ] Limits enforced: [YES/NO]
- [ ] Alerts configured: [YES/NO]

### Dashboard Review
Overview Dashboard:
- [ ] All panels have data: [YES/NO]
- [ ] Queries optimized: [YES/NO]
- [ ] Variables work: [YES/NO]
- [ ] Time ranges appropriate: [YES/NO]

GraphQL Dashboard:
- [ ] Operation breakdown accurate: [YES/NO]
- [ ] Slow query tracking works: [YES/NO]
- [ ] Error attribution correct: [YES/NO]

System Dashboard:
- [ ] Resource metrics accurate: [YES/NO]
- [ ] Trends visible: [YES/NO]
- [ ] Thresholds marked: [YES/NO]

### Alert Review
- [ ] All critical alerts defined: [YES/NO]
- [ ] Thresholds reasonable: [YES/NO]
- [ ] No false positives during test: [YES/NO]
- [ ] Alert descriptions clear: [YES/NO]
- [ ] Runbooks referenced: [YES/NO]

### Documentation Review
- [ ] Metrics documented: [YES/NO]
- [ ] Dashboard guide written: [YES/NO]
- [ ] Troubleshooting guide: [YES/NO]
- [ ] Configuration options: [YES/NO]

### Security Audit
- [ ] No PII in any telemetry: [YES/NO]
- [ ] Metrics endpoint secured: [YES/NO]
- [ ] Trace headers validated: [YES/NO]
- [ ] Log access controlled: [YES/NO]

### Outstanding Issues
[Any issues for Phase 6 consideration]

### Recommendations for Phase 6
[Suggestions based on performance data]

### Decision: [APPROVED FOR PHASE 6 / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 5 Status: [COMPLETE / INCOMPLETE]
```

## How to Handle Issues

When you find issues during review:

1. **Categorize by severity**:
   - **CRITICAL**: Security exposure, data leaks
   - **HIGH**: Missing cardinality control, no sanitization
   - **MEDIUM**: Performance overhead, missing metrics
   - **LOW**: Dashboard improvements, documentation

2. **Test security thoroughly**:
   - Can PII leak into metrics? (CRITICAL)
   - Are logs properly sanitized? (CRITICAL)
   - Is cardinality unbounded? (HIGH)
   - Is performance impact too high? (MEDIUM)

3. **Provide specific fixes**:
   ```markdown
   Issue: User emails appearing in logs
   Severity: CRITICAL
   Fix: Add sanitization rule:
   ```rust
   SanitizationRule::regex(r"[\w.-]+@[\w.-]+\.\w+", "<EMAIL>")
   ```

## Review Decision Framework

### APPROVED
Grant approval when:
- All Done Criteria met
- Security requirements satisfied
- Performance overhead < 5%
- Cardinality controlled
- Only LOW severity issues

### CHANGES REQUIRED
Require changes when:
- Sensitive data exposed
- Cardinality unbounded
- Major metrics missing
- Performance overhead > 5%
- Any CRITICAL or HIGH issues

## Testing Observability Overhead

Critical performance tests:

1. **Baseline Performance**
   ```bash
   # Without observability
   cargo build --release --no-default-features
   ./scripts/load-test.sh > baseline.txt
   ```

2. **With Full Observability**
   ```bash
   # With all features
   cargo build --release
   ./scripts/load-test.sh > with-observability.txt
   ```

3. **Compare Results**
   - Request latency increase
   - Throughput decrease
   - Memory usage increase
   - CPU usage increase

4. **Profile Hot Paths**
   ```bash
   # Use flamegraph to find overhead
   cargo flamegraph --release
   ```

## Final Review Checklist

Before submitting your review:
- [ ] Tested all endpoints and formats
- [ ] Verified security requirements
- [ ] Measured performance impact
- [ ] Checked cardinality limits
- [ ] Reviewed all dashboards
- [ ] Validated alert rules
- [ ] Made clear APPROVED/CHANGES REQUIRED decision
- [ ] Included specific remediation if needed

## Template for Review Submission

```markdown
# Phase 5 - Checkpoint [N] Review

**Reviewer**: [Your designation]
**Date**: [Current date]
**Implementation Agent**: [Agent who requested review]

## Review Summary
[2-3 sentences summarizing observability implementation state]

## Detailed Findings
[Your complete review output for this checkpoint]

## Security Assessment
- Data sanitization: [Complete/Gaps found]
- Metric labels: [Safe/PII exposed]
- Access controls: [Adequate/Insufficient]
- Error handling: [Secure/Information leaks]

## Performance Assessment
- Metrics overhead: __% 
- Logging overhead: __%
- Tracing overhead: __%
- Total overhead: __% (target < 5%)

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

Remember: Observability is critical for production operations. Be thorough in testing all telemetry paths and ensuring no sensitive data leaks.
# Phase 5: Observability & Monitoring - Work Plan

> **Implementation Note**: This plan provides structured guidance while allowing flexibility for real-world constraints. If you encounter blockers, see the "Troubleshooting and Escalation" section. Security requirements are non-negotiable, but implementation approaches can vary with proper documentation.

## Prerequisites

Before starting Phase 5, ensure you have:
- **Completed Phases 1-4**: Server foundation, database layer, GraphQL implementation, and authorization operational
- **Metrics Knowledge**: Understanding of Prometheus metrics types and cardinality control
- **Tracing Experience**: Familiarity with distributed tracing concepts and OpenTelemetry
- **Logging Best Practices**: Understanding of structured logging and security considerations
- **Performance Analysis**: Experience with profiling and bottleneck identification

## Quick Reference - Essential Resources

### Example Files
Example files in `/api/.claude/.spec/examples/`:
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Test examples
- **Metrics Patterns** - If file missing, use inline examples in this document
- **Tracing Patterns** - If file missing, refer to Task 5.3.3 examples

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[metrics.md](../../.spec/metrics.md)** - Complete metrics specification
- **[logging.md](../../.spec/logging.md)** - Logging and sanitization requirements
- **[SPEC.md](../../SPEC.md)** - Observability requirements (lines 54-63)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 5 objectives (lines 124-155)

### Junior Developer Resources
Comprehensive guides in `/api/.claude/junior-dev-helper/`:
- **[Observability Tutorial](../../junior-dev-helper/observability-tutorial.md)** - Overview of metrics, logs, and traces
- **[Prometheus Metrics Guide](../../junior-dev-helper/prometheus-metrics-guide.md)** - How to implement and use metrics
- **[Structured Logging Guide](../../junior-dev-helper/structured-logging-guide.md)** - JSON logging and sanitization
- **[OpenTelemetry Tracing Guide](../../junior-dev-helper/opentelemetry-tracing-guide.md)** - Distributed tracing concepts
- **[Common Observability Errors](../../junior-dev-helper/observability-common-errors.md)** - Troubleshooting guide
- **[Cardinality Control Guide](../../junior-dev-helper/cardinality-control-guide.md)** - Preventing metric explosion
- **[Observability TDD Examples](../../junior-dev-helper/observability-tdd-examples.md)** - Test-driven development for observability

### Quick Links
- **Verification Script**: `scripts/verify-phase-5.sh` 
  - If missing, create using template in Appendix A
  - Or use manual verification steps in each checkpoint
- **Metrics Test Suite**: `scripts/test-metrics.sh`
  - If missing, use `just test-metrics`
- **Load Test Script**: `scripts/load-test.sh`
  - If missing, see Performance Testing section

## Overview
This work plan implements comprehensive observability with Prometheus metrics, structured logging, and distributed tracing. The focus is on cardinality control, performance impact minimization, and security-conscious data collection. Each checkpoint represents a natural boundary for review.

## Build and Test Commands

Continue using `just` as the command runner:
- `just test` - Run all tests including observability tests
- `just test-metrics` - Run only metrics-related tests
- `just test-tracing` - Run only tracing-related tests
- `just metrics-up` - Start local Prometheus for testing
- `just grafana-up` - Start Grafana with dashboards

Always use these commands instead of direct cargo commands to ensure consistency.

## IMPORTANT: Review Process

**This plan includes 4 mandatory review checkpoints where work should pause for external review.**

At each checkpoint:
1. **PAUSE work** and commit your code
2. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
3. **Wait for approval** (maximum 24 hours)

**If no review response within 24 hours**:
- Document any concerns in `api/.claude/.reviews/checkpoint-X-pending.md`
- Proceed cautiously to next section
- Flag implementation as "pending review"
- DO NOT skip security or cardinality controls

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Follow TDD practices with these allowances:

1. **Write tests FIRST** - Before implementation
   - Exception: Exploration spikes allowed (max 2 hours)
   - MUST delete spike code and restart with tests

2. **Run tests to see them FAIL** - Confirms test is valid
   - If test passes immediately, verify it's testing the right thing

3. **Write minimal code to make tests PASS**
   - OK to refactor immediately if obvious improvements

4. **REFACTOR** - Clean up while keeping tests green
   - Performance optimizations allowed here

5. **Document as you go** - Add rustdoc comments
   - Can be brief initially, expand during refactor

## Done Criteria Checklist

### Required Observability Coverage
- [ ] /metrics endpoint returns valid Prometheus format
- [ ] User-facing operations emit structured logs with trace IDs:
  - All GraphQL queries/mutations
  - All HTTP endpoints
  - Background job initiation
- [ ] Infrastructure operations have basic logging:
  - Health checks (INFO level only)
  - Metric collection (DEBUG level)
- [ ] Distributed tracing spans for significant operations:
  - GraphQL resolver execution (> 10ms)
  - Database queries
  - External service calls
  - Authorization checks
- [ ] No sensitive data in logs
- [ ] Monitoring dashboards created
- [ ] Cardinality limits enforced
- [ ] Performance impact within acceptable range:
  - Target: < 5% overhead
  - Acceptable: < 10% with documented justification
  - Unacceptable: ≥ 10% without optimization plan
- [ ] All code has corresponding tests written first

### Optional Coverage
- [ ] Field-level resolver tracing (for optimization)
- [ ] Cache operation tracing (if performance concern)
- [ ] Internal utility function logging

## Work Breakdown with Review Checkpoints

### 5.1 Metrics Implementation (3-4 work units)

**Work Unit Context:**
- **Complexity**: Medium - Cardinality control and performance considerations
- **Scope**: ~800 lines across 6-7 files
- **Key Components**: 
  - Prometheus recorder setup (~150 lines)
  - Core metrics implementation (~300 lines)
  - Cardinality limiter (~200 lines)
  - HTTP metrics endpoint (~100 lines)
  - Performance sampling (~50 lines)
- **Patterns**: Label limiting, atomic counters, histogram sampling

#### Task 5.1.1: Write Metrics Tests First
**💡 Junior Dev Tip**: Start by reading the [Prometheus Metrics Guide](../../junior-dev-helper/prometheus-metrics-guide.md) to understand metric types. The [Observability TDD Examples](../../junior-dev-helper/observability-tdd-examples.md) shows how to test metrics.

Create `src/observability/metrics.rs` with comprehensive test module:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use metrics::{counter, histogram, gauge};
    
    #[tokio::test]
    async fn test_graphql_request_metrics() {
        // Test that GraphQL requests increment counters
        let recorder = setup_test_recorder();
        
        // Execute a GraphQL query
        counter!("graphql_request_total",
            "operation_type" => "query",
            "operation_name" => "getUser",
            "status" => "success"
        ).increment(1);
        
        // Verify metric was recorded
        let metrics = recorder.render();
        assert!(metrics.contains("graphql_request_total"));
        assert!(metrics.contains("operation_type=\"query\""));
    }
    
    #[test]
    fn test_cardinality_limiter() {
        // See "Cardinality Limit Guidelines" section for choosing the right limit
        let limiter = CardinalityLimiter::new(50); // Default 50, adjust based on your operation count
        
        // Test under limit
        for i in 0..50 {
            let label = limiter.get_operation_label(&format!("operation_{}", i));
            assert_eq!(label, format!("operation_{}", i));
        }
        
        // Test over limit - should return "other"
        let label = limiter.get_operation_label("operation_51");
        assert_eq!(label, "other");
    }
    
    #[test]
    fn test_status_code_bucketing() {
        assert_eq!(bucket_status_code(200), "2xx");
        assert_eq!(bucket_status_code(404), "4xx");
        assert_eq!(bucket_status_code(503), "5xx");
    }
}
```

#### Task 5.1.2: Implement Prometheus Recorder
Create metrics initialization with cardinality controls:
```rust
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

pub struct MetricsManager {
    handle: PrometheusHandle,
    cardinality_limiter: Arc<CardinalityLimiter>,
}

impl MetricsManager {
    pub fn new(config: &Config) -> Result<Self> {
        // Initialize with cardinality limits
        let limiter = Arc::new(CardinalityLimiter::new(
            config.metrics.max_operation_labels
        ));
        
        let builder = PrometheusBuilder::new()
            .with_http_listener(([0, 0, 0, 0], config.metrics.port))
            .add_global_label("service", "pcf-api")
            .add_global_label("environment", &config.environment)
            .add_global_label("version", env!("CARGO_PKG_VERSION"));
            
        let handle = builder.install_recorder()?;
        
        Ok(Self { handle, cardinality_limiter: limiter })
    }
}
```

#### Task 5.1.3: Core Metrics Implementation
**⚠️ Cardinality Warning**: Before implementing, read the [Cardinality Control Guide](../../junior-dev-helper/cardinality-control-guide.md) to avoid metric explosion!

Implement cardinality controls with these guidelines:

#### Cardinality Limit Guidelines

The default limit of 50 unique operations is based on typical microservice patterns. Here's how to determine the right limit for your system:

**Cardinality Limit Recommendations by System Size:**

| System Type | Endpoints | Recommended Operation Limit | Total Series Estimate |
|------------|-----------|---------------------------|---------------------|
| Microservice | 10-15 | 50 operations | ~2,500 series |
| Small API | 20-30 | 75 operations | ~5,000 series |
| Monolith | 50-100 | 100-150 operations | ~10,000 series |
| Large System | 100+ | 200+ operations (with grouping) | ~20,000 series |

**Calculating Your Safe Limit:**
```
Safe Operation Limit = (Available Memory MB / 3KB) / (Label Combinations × Safety Factor)

Where:
- Available Memory = Total RAM allocated to Prometheus
- 3KB = Approximate memory per series
- Label Combinations = methods × statuses × other labels
- Safety Factor = 2 (leaves room for growth)

Example: 4GB RAM, 5 methods, 5 statuses, 3 user types
Safe Limit = (4096 MB / 0.003 MB) / (5 × 5 × 3 × 2) = ~9,000 operations
```

**When Higher Limits Are Acceptable:**
1. **GraphQL APIs**: May have 100+ unique operations
   - Group by operation type (query/mutation) and domain
   - Example: "user_queries", "order_mutations"
   
2. **REST APIs with dynamic paths**: 
   - Normalize paths: `/users/123` → `/users/:id`
   - Group by resource: "user_operations", "order_operations"

3. **Multi-tenant systems**:
   - Don't use tenant_id as label!
   - Use tenant tier instead: "free", "premium", "enterprise"

**Implementation Strategy for High Cardinality:**
- Default limit: 50 unique operations
- Maximum safe limit: 100 operations  
- If your application has more operations:
  1. Document why in `api/.claude/observability/cardinality-justification.md`
  2. Implement hierarchical grouping (e.g., by module)
  3. Add monitoring for cardinality growth
  4. Consider these patterns:
     ```rust
     // Pattern 1: Domain grouping
     let operation_group = match operation_name {
         name if name.starts_with("user") => "user_operations",
         name if name.starts_with("order") => "order_operations",
         name if name.starts_with("admin") => "admin_operations",
         _ => "other_operations"
     };
     
     // Pattern 2: Importance-based limiting
     let operation_label = if is_critical_operation(name) {
         name // Keep exact name for critical ops
     } else if operation_count < 40 {
         name // Keep name while under soft limit
     } else {
         "non_critical_other" // Group non-critical ops
     };
     ```

Implement all required metrics from specification:
```rust
// GraphQL metrics
pub fn record_graphql_request(
    operation_type: &str,
    operation_name: &str,
    duration: Duration,
    status: RequestStatus,
) {
    // Apply cardinality limiting
    let operation_name = CARDINALITY_LIMITER.get_operation_label(operation_name);
    
    counter!("graphql_request_total",
        "operation_type" => operation_type,
        "operation_name" => operation_name,
        "status" => status.as_str()
    ).increment(1);
    
    histogram!("graphql_request_duration_seconds",
        "operation_type" => operation_type,
        "operation_name" => operation_name
    ).record(duration.as_secs_f64());
}
```

#### Task 5.1.4: Metrics Endpoint
Create `/metrics` endpoint with security considerations:
```rust
pub async fn metrics_handler(
    State(metrics): State<Arc<MetricsManager>>,
    headers: HeaderMap,
) -> Result<String, StatusCode> {
    // Optional IP allowlist check
    if let Some(allowlist) = &metrics.ip_allowlist {
        let client_ip = extract_client_ip(&headers);
        if !allowlist.contains(&client_ip) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    
    Ok(metrics.handle.render())
}
```

### 🛑 CHECKPOINT 1: Metrics Implementation Review
**Deliverables**:
- Prometheus metrics endpoint working
- All GraphQL metrics implemented
- Cardinality limiting functional
- Tests passing with TDD approach
- No sensitive data exposed

---

### 5.2 Structured Logging (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Security-conscious logging with performance
- **Scope**: ~600 lines across 4-5 files
- **Key Components**: 
  - Tracing subscriber setup (~200 lines)
  - Log sanitization layer (~150 lines)
  - Trace ID injection (~100 lines)
  - Format switching (dev/prod) (~100 lines)
  - Performance sampling (~50 lines)
- **Patterns**: Layer composition, async logging, field sanitization

#### Task 5.2.1: Write Logging Tests First
**🔒 Security First**: Review the [Structured Logging Guide](../../junior-dev-helper/structured-logging-guide.md) to understand log sanitization. Never log passwords or PII!

Create comprehensive logging tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{info, debug, error};
    use tracing_test::traced_test;
    
    #[traced_test]
    #[test]
    fn test_log_sanitization() {
        // Test that sensitive data is sanitized
        let user_id = "user_12345";
        let email = "test@example.com";
        
        info!(user_id = %user_id, email = %email, "User login");
        
        // Verify logs contain sanitized values
        assert!(logs_contain("user_id=<REDACTED>"));
        assert!(logs_contain("email=<REDACTED>"));
    }
    
    #[tokio::test]
    async fn test_trace_id_propagation() {
        let trace_id = "test-trace-123";
        
        info!(trace_id = %trace_id, "Request started");
        
        // Verify all logs contain trace_id
        assert!(logs_contain("trace_id=test-trace-123"));
    }
}
```

#### Task 5.2.2: Implement Tracing Subscriber
Create layered tracing setup:
```rust
pub fn init_tracing(config: &Config) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.logging.level));
        
    let fmt_layer = if config.is_production() {
        // JSON format for production
        tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
    } else {
        // Pretty format for development
        tracing_subscriber::fmt::layer()
            .pretty()
            .with_thread_names(true)
    };
    
    // Add sanitization layer
    let sanitize_layer = SanitizationLayer::new(vec![
        SanitizationRule::regex(r"user_\d+", "<USER_ID>"),
        SanitizationRule::field("email", "<EMAIL>"),
        SanitizationRule::field("password", "<REDACTED>"),
    ]);
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(sanitize_layer)
        .init();
        
    Ok(())
}
```

#### Task 5.2.3: Log Sanitization Implementation
Create security-conscious sanitization:
```rust
pub struct SanitizationLayer {
    rules: Vec<SanitizationRule>,
}

impl<S> Layer<S> for SanitizationLayer
where
    S: Subscriber,
{
    // Implement field visitor that applies rules
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = SanitizingVisitor::new(&self.rules);
        event.record(&mut visitor);
        // Forward sanitized event
    }
}
```

### 🛑 CHECKPOINT 2: Logging Implementation Review
**Deliverables**:
- Structured JSON logging in production
- Pretty logging in development
- All sensitive data sanitized
- Trace ID in every log entry
- Performance impact minimal

---

### 5.3 Distributed Tracing (2-3 work units)

**Work Unit Context:**
- **Complexity**: High - Cross-service tracing with context propagation
- **Scope**: ~700 lines across 5-6 files
- **Key Components**: 
  - OpenTelemetry setup (~200 lines)
  - Span creation helpers (~150 lines)
  - Context propagation (~150 lines)
  - GraphQL span enrichment (~100 lines)
  - HTTP trace headers (~100 lines)
- **Patterns**: Async context, span attributes, trace sampling

#### Task 5.3.1: Write Tracing Tests First
**🗺️ Distributed Systems**: The [OpenTelemetry Tracing Guide](../../junior-dev-helper/opentelemetry-tracing-guide.md) explains how traces work across services. Start there if tracing is new to you!

Create distributed tracing tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{Tracer, SpanKind};
    
    #[tokio::test]
    async fn test_span_creation() {
        let tracer = init_test_tracer();
        
        let span = tracer
            .span_builder("test_operation")
            .with_kind(SpanKind::Internal)
            .start(&tracer);
            
        span.set_attribute("test.attribute", "value");
        span.end();
        
        // Verify span was exported
        let spans = get_exported_spans();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, "test_operation");
    }
    
    #[tokio::test]
    async fn test_trace_context_propagation() {
        // Test that trace context propagates through async operations
        let trace_id = create_trace_id();
        
        with_trace_context(trace_id, async {
            let current_trace = current_trace_id();
            assert_eq!(current_trace, trace_id);
        }).await;
    }
}
```

#### Task 5.3.2: OpenTelemetry Integration
Setup OpenTelemetry with OTLP exporter:
```rust
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;

pub fn init_tracing(config: &Config) -> Result<()> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.tracing.otlp_endpoint)
        )
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::TraceIdRatioBased(config.tracing.sample_rate))
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "pcf-api"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
        
    global::set_tracer_provider(tracer.provider().unwrap());
    
    Ok(())
}
```

#### Task 5.3.3: Span Instrumentation
Add spans to all operations:
```rust
#[tracing::instrument(
    skip(ctx, input),
    fields(
        operation.type = "mutation",
        operation.name = "createNote",
        user.id = %extract_user_id(ctx)
    )
)]
pub async fn create_note(
    &self,
    ctx: &Context<'_>,
    input: CreateNoteInput,
) -> Result<Note> {
    let span = Span::current();
    span.set_attribute("input.title_length", input.title.len() as i64);
    
    // Check authorization with span
    let _auth_span = span.in_scope(|| {
        info_span!("authorization_check", 
            resource = %format!("note:new"),
            action = "create"
        )
    });
    is_authorized(ctx, "note:new", "create").await?;
    
    // Database operation with span
    let _db_span = span.in_scope(|| {
        info_span!("database_operation",
            db.operation = "insert",
            db.table = "notes"
        )
    });
    
    let note = self.db.create_note(input).await?;
    
    span.set_attribute("result.note_id", &note.id);
    Ok(note)
}
```

### 🛑 CHECKPOINT 3: Tracing Implementation Review
**Deliverables**:
- OpenTelemetry integration complete
- Spans for all GraphQL operations
- Trace context propagation working
- External service calls traced
- Sampling configured properly

---

### 5.4 Integration and Dashboards (1-2 work units)

**Work Unit Context:**
- **Complexity**: Low - Integration and visualization
- **Scope**: ~300 lines plus dashboard configs
- **Key Components**: 
  - Metrics correlations (~100 lines)
  - Health check metrics (~50 lines)
  - Dashboard JSON files (~5 dashboards)
  - Alert rule definitions (~50 rules)
  - Documentation (~100 lines)
- **Patterns**: PromQL queries, Grafana templates

#### Task 5.4.1: Complete Integration Tests
Write tests verifying all observability components work together:
```rust
#[tokio::test]
async fn test_full_observability_integration() {
    let app = setup_test_app().await;
    
    // Make GraphQL request
    let response = app.graphql_request(
        r#"{ user(id: "123") { name } }"#
    ).await;
    
    // Verify metrics recorded
    let metrics = app.get_metrics().await;
    assert!(metrics.contains("graphql_request_total"));
    
    // Verify logs emitted
    let logs = app.get_logs();
    assert!(logs.iter().any(|log| log.contains("GraphQL query")));
    
    // Verify spans created
    let spans = app.get_spans().await;
    assert!(!spans.is_empty());
}
```

#### Task 5.4.2: Create Grafana Dashboards
Create comprehensive monitoring dashboards:
```json
{
  "dashboard": {
    "title": "PCF API Overview",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [{
          "expr": "sum(rate(graphql_request_total[5m])) by (operation_type)"
        }]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "sum(rate(graphql_errors_total[5m])) / sum(rate(graphql_request_total[5m]))"
        }]
      },
      {
        "title": "P95 Latency",
        "targets": [{
          "expr": "histogram_quantile(0.95, rate(graphql_request_duration_seconds_bucket[5m]))"
        }]
      }
    ]
  }
}
```

#### Task 5.4.3: Configure Alerts
Define Prometheus alert rules:
```yaml
groups:
  - name: pcf-api-alerts
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(graphql_errors_total[5m])) / 
          sum(rate(graphql_request_total[5m])) > 0.05
        for: 5m
        annotations:
          summary: "GraphQL error rate above 5%"
          
      - alert: HighCardinality
        expr: count(count by (operation_name)(graphql_request_total)) > 50
        for: 10m
        annotations:
          summary: "Too many unique operations being tracked"
```

### 🛑 CHECKPOINT 4: Complete Phase 5 System Review
**Deliverables**:
- All observability components integrated
- Grafana dashboards functional
- Alert rules configured
- Performance overhead < 5%
- Documentation complete

---

## Common Troubleshooting

**📚 See Also**: The [Common Observability Errors](../../junior-dev-helper/observability-common-errors.md) guide has detailed solutions for all these issues and more!

### Issue: Metrics endpoint returns empty
**Solution**: Ensure PrometheusHandle is stored and not dropped

### Issue: High cardinality warnings
**Solution**: Check cardinality limiter is working, reduce label dimensions

### Issue: Missing trace IDs in logs
**Solution**: Verify tracing subscriber is initialized before logging

### Issue: Spans not appearing
**Solution**: Check OTLP endpoint configuration and network connectivity

## Performance Considerations

1. **Metric Collection**: 
   - MUST use atomic counters for hot paths
   - Target < 1% overhead per metric
   - If overhead > 1%, implement sampling

2. **Log Sampling**: 
   - MUST sample verbose logs in production
   - Minimum: ERROR and WARN at 100%
   - INFO at 10-100% based on volume
   - DEBUG/TRACE at 1% maximum

3. **Trace Sampling**: 
   - Use ratio-based sampling (e.g., 10%)
   - Critical operations (errors, slow queries) at 100%
   - Normal operations at configured rate

4. **Async Operations**: 
   - Ensure metrics don't block request processing
   - Use fire-and-forget for non-critical telemetry

## Security Requirements (NON-NEGOTIABLE)

These requirements MUST be met, but we provide recovery paths:

1. **PII Protection in Metrics**:
   - MUST NOT use raw user IDs or emails as labels
   - Acceptable alternatives:
     - Hashed user IDs (with salt)
     - User type/tier classifications
     - Anonymized identifiers
   - If you need user correlation, document approach in security review

2. **Log Sanitization**:
   - MUST sanitize before output: passwords, tokens, API keys
   - SHOULD sanitize: emails, IPs, user IDs
   - Sanitization failures MUST fail closed (redact if unsure)
   - Test with `cargo test --features=sanitization-strict`

3. **Metrics Endpoint Security**:
   - MUST restrict access via network or IP allowlist
   - If allowlist not feasible, implement authentication
   - Document security approach in `api/.claude/observability/security.md`

4. **Trace Headers**:
   - MUST validate format to prevent injection
   - Use standard W3C TraceContext format
   - Reject malformed headers with 400 Bad Request

### Recovery from Security Mistakes
If sensitive data is accidentally logged:
1. Immediately add sanitization rule
2. Document incident in `api/.claude/security/incidents.md`
3. Run `just purge-sensitive-logs` if available
4. Add test to prevent recurrence

## Success Criteria

By the end of Phase 5:
1. Complete observability of all API operations
2. Actionable metrics for monitoring and alerting
3. Structured logs suitable for analysis
4. Distributed traces for debugging
5. Minimal performance impact
6. No security vulnerabilities from observability

## Junior Developer Learning Path

If you're new to observability, follow this progression:

1. **Start with Concepts** - Read [Observability Tutorial](../../junior-dev-helper/observability-tutorial.md) to understand the three pillars
2. **Learn Metrics** - Study [Prometheus Metrics Guide](../../junior-dev-helper/prometheus-metrics-guide.md) for metric types and patterns
3. **Understand Cardinality** - **Critical**: Read [Cardinality Control Guide](../../junior-dev-helper/cardinality-control-guide.md) before implementing any metrics
4. **Master Logging** - Review [Structured Logging Guide](../../junior-dev-helper/structured-logging-guide.md) for security and best practices
5. **Explore Tracing** - Learn distributed tracing with [OpenTelemetry Tracing Guide](../../junior-dev-helper/opentelemetry-tracing-guide.md)
6. **Practice TDD** - Follow examples in [Observability TDD Examples](../../junior-dev-helper/observability-tdd-examples.md)
7. **Debug Issues** - Keep [Common Observability Errors](../../junior-dev-helper/observability-common-errors.md) handy for troubleshooting

**Remember**: 
- Observability can kill your service if done wrong (high cardinality = OOM)
- Never log sensitive data (passwords, tokens, PII)
- Test your observability code - it needs to work when everything else is broken
- Start simple, add complexity gradually

## Troubleshooting and Escalation

### When You're Stuck

If blocked for more than 2 hours:
1. Document the issue in `api/.claude/observability/blockers.md`
2. Try alternative approach from guides
3. Continue with next task if possible
4. Mark blocked item for review

### Common Blockers and Recovery

1. **Cardinality Explosion**
   - Implement emergency limits
   - Document in review notes
   - Plan proper fix for next iteration

2. **Performance Over Target**
   - Ship with feature flag
   - Document optimization plan
   - Set up monitoring for impact

3. **Missing Dependencies**
   - Use inline examples
   - Create minimal version
   - Document what's needed

4. **Test Failures**
   - Skip after 3 fix attempts
   - Document in known-issues.md
   - MUST not skip security tests

## Next Phase Preview

Phase 6 will focus on Performance Optimization:
- DataLoader implementation for N+1 prevention
- Response caching strategies
- Connection pooling optimization
- Load testing and tuning

## Appendix A: Verification Script Template

If `scripts/verify-phase-5.sh` is missing, create it with:

```bash
#!/bin/bash
set -e

echo "=== Phase 5 Observability Verification ==="

# Check metrics endpoint
echo "1. Checking metrics endpoint..."
if curl -s http://localhost:9090/metrics | grep -q "graphql_request_total"; then
    echo "✓ Metrics endpoint working"
else
    echo "✗ Metrics endpoint not responding correctly"
    exit 1
fi

# Check cardinality
echo "2. Checking cardinality limits..."
CARDINALITY=$(curl -s http://localhost:9090/metrics | grep -v '^#' | wc -l)
echo "   Total series: $CARDINALITY"
if [ $CARDINALITY -gt 10000 ]; then
    echo "⚠️  Warning: High cardinality detected"
fi

# Check log format
echo "3. Checking log format..."
if [ "$ENVIRONMENT" = "production" ]; then
    # Should be JSON in production
    if tail -1 logs/app.log | jq . > /dev/null 2>&1; then
        echo "✓ JSON logging enabled"
    else
        echo "✗ Logs not in JSON format"
    fi
fi

echo "=== Basic verification complete ==="
```

Make executable with: `chmod +x scripts/verify-phase-5.sh`
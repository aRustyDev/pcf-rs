# Phase 5: Observability & Monitoring - Work Plan

## Prerequisites

Before starting Phase 5, ensure you have:
- **Completed Phases 1-4**: Server foundation, database layer, GraphQL implementation, and authorization operational
- **Metrics Knowledge**: Understanding of Prometheus metrics types and cardinality control
- **Tracing Experience**: Familiarity with distributed tracing concepts and OpenTelemetry
- **Logging Best Practices**: Understanding of structured logging and security considerations
- **Performance Analysis**: Experience with profiling and bottleneck identification

## Quick Reference - Essential Resources

### Example Files
All example files are located in `/api/.claude/.spec/examples/`:
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Comprehensive test examples following TDD
- **[Metrics Patterns](../../.spec/examples/metrics-patterns.rs)** - Metrics implementation patterns (to be created)
- **[Tracing Patterns](../../.spec/examples/tracing-patterns.rs)** - Distributed tracing examples (to be created)

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[metrics.md](../../.spec/metrics.md)** - Complete metrics specification
- **[logging.md](../../.spec/logging.md)** - Logging and sanitization requirements
- **[SPEC.md](../../SPEC.md)** - Observability requirements (lines 54-63)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 5 objectives (lines 124-155)

### Quick Links
- **Verification Script**: `scripts/verify-phase-5.sh` (to be created)
- **Metrics Test Suite**: `scripts/test-metrics.sh` (to be created)
- **Load Test Script**: `scripts/load-test.sh` (to be created)

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

**This plan includes 4 mandatory review checkpoints where work MUST stop for external review.**

At each checkpoint:
1. **STOP all work** and commit your code
2. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
3. **Wait for approval** before continuing to next section

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Continue following TDD practices from previous phases:
1. **Write tests FIRST** - Before any implementation
2. **Run tests to see them FAIL** - Confirms test is valid
3. **Write minimal code to make tests PASS** - No more than necessary
4. **REFACTOR** - Clean up while keeping tests green
5. **Document as you go** - Add rustdoc comments and inline explanations

## Done Criteria Checklist
- [ ] /metrics endpoint returns valid Prometheus format
- [ ] All operations emit structured logs with trace IDs
- [ ] Distributed tracing spans created for all operations
- [ ] No sensitive data in logs
- [ ] Monitoring dashboards created
- [ ] Cardinality limits enforced
- [ ] Performance impact < 5% overhead
- [ ] All code has corresponding tests written first

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
        let limiter = CardinalityLimiter::new(50); // Max 50 unique operations
        
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

### Issue: Metrics endpoint returns empty
**Solution**: Ensure PrometheusHandle is stored and not dropped

### Issue: High cardinality warnings
**Solution**: Check cardinality limiter is working, reduce label dimensions

### Issue: Missing trace IDs in logs
**Solution**: Verify tracing subscriber is initialized before logging

### Issue: Spans not appearing
**Solution**: Check OTLP endpoint configuration and network connectivity

## Performance Considerations

1. **Metric Collection**: Use atomic counters for hot paths
2. **Log Sampling**: Sample verbose logs in production
3. **Trace Sampling**: Use ratio-based sampling (e.g., 10%)
4. **Async Operations**: Ensure metrics don't block request processing

## Security Requirements

1. **No PII in Metrics**: Never use user IDs or emails as labels
2. **Log Sanitization**: Apply before any output
3. **Metrics Endpoint**: Restrict access via network or IP allowlist
4. **Trace Headers**: Validate format to prevent injection

## Success Criteria

By the end of Phase 5:
1. Complete observability of all API operations
2. Actionable metrics for monitoring and alerting
3. Structured logs suitable for analysis
4. Distributed traces for debugging
5. Minimal performance impact
6. No security vulnerabilities from observability

## Next Phase Preview

Phase 6 will focus on Performance Optimization:
- DataLoader implementation for N+1 prevention
- Response caching strategies
- Connection pooling optimization
- Load testing and tuning
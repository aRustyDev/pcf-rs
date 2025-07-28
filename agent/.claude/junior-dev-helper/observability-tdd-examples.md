# Observability TDD Examples

## Test-Driven Development for Observability

When implementing observability, we follow TDD principles:
1. Write the test first (RED)
2. Make it pass with minimal code (GREEN)
3. Refactor while keeping tests passing (REFACTOR)

## Metrics TDD Examples

### Example 1: Request Counter

#### Step 1: Write the Test (RED)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use metrics::counter;
    use metrics_util::debugging::{DebugController, DebuggingRecorder};
    
    #[test]
    fn test_request_counter_increments() {
        // Arrange
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        metrics::with_local_recorder(&recorder, || {
            // Act
            handle_request("GET", "/api/users", 200);
            
            // Assert
            let snapshot = snapshotter.snapshot();
            
            // Check counter exists
            let counter = snapshot.into_hashmap()
                .get(&format!("http_requests_total{{method=\"GET\",endpoint=\"/api/users\",status=\"2xx\"}}"))
                .expect("Counter should exist");
                
            assert_eq!(counter.into_counter().unwrap(), 1);
        });
    }
    
    // This test FAILS because handle_request doesn't exist yet!
}
```

#### Step 2: Implement Minimal Code (GREEN)
```rust
fn handle_request(method: &str, endpoint: &str, status_code: u16) {
    let status = match status_code {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    };
    
    counter!("http_requests_total",
        "method" => method,
        "endpoint" => endpoint,
        "status" => status
    ).increment(1);
}
```

#### Step 3: Add More Tests
```rust
#[test]
fn test_request_counter_groups_status_codes() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    
    metrics::with_local_recorder(&recorder, || {
        // Different status codes in same group
        handle_request("GET", "/api/users", 200);
        handle_request("GET", "/api/users", 201);
        handle_request("GET", "/api/users", 204);
        
        let snapshot = snapshotter.snapshot();
        let counter = snapshot.into_hashmap()
            .get(&format!("http_requests_total{{method=\"GET\",endpoint=\"/api/users\",status=\"2xx\"}}"))
            .unwrap();
            
        assert_eq!(counter.into_counter().unwrap(), 3);
    });
}

#[test]
fn test_request_counter_different_methods() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    
    metrics::with_local_recorder(&recorder, || {
        handle_request("GET", "/api/users", 200);
        handle_request("POST", "/api/users", 201);
        
        let snapshot = snapshotter.snapshot();
        
        // GET counter
        assert_eq!(
            snapshot.into_hashmap()
                .get(&format!("http_requests_total{{method=\"GET\",endpoint=\"/api/users\",status=\"2xx\"}}"))
                .unwrap()
                .into_counter()
                .unwrap(),
            1
        );
        
        // POST counter
        assert_eq!(
            snapshot.into_hashmap()
                .get(&format!("http_requests_total{{method=\"POST\",endpoint=\"/api/users\",status=\"2xx\"}}"))
                .unwrap()
                .into_counter()
                .unwrap(),
            1
        );
    });
}
```

### Example 2: Cardinality Limiter

#### Step 1: Write the Test (RED)
```rust
#[test]
fn test_cardinality_limiter_enforces_limit() {
    let limiter = CardinalityLimiter::new(3);
    
    // First 3 values should be accepted
    assert_eq!(limiter.check_and_limit("endpoint1"), "endpoint1");
    assert_eq!(limiter.check_and_limit("endpoint2"), "endpoint2");
    assert_eq!(limiter.check_and_limit("endpoint3"), "endpoint3");
    
    // 4th value should be limited
    assert_eq!(limiter.check_and_limit("endpoint4"), "other");
    
    // Existing values should still work
    assert_eq!(limiter.check_and_limit("endpoint1"), "endpoint1");
}
```

#### Step 2: Implement CardinalityLimiter (GREEN)
```rust
use std::sync::Mutex;
use std::collections::HashSet;

pub struct CardinalityLimiter {
    max_values: usize,
    seen_values: Mutex<HashSet<String>>,
}

impl CardinalityLimiter {
    pub fn new(max_values: usize) -> Self {
        Self {
            max_values,
            seen_values: Mutex::new(HashSet::new()),
        }
    }
    
    pub fn check_and_limit(&self, value: &str) -> &str {
        let mut seen = self.seen_values.lock().unwrap();
        
        if seen.contains(value) {
            return value;
        }
        
        if seen.len() < self.max_values {
            seen.insert(value.to_string());
            // SAFETY: We just inserted it, so we know it exists
            return seen.get(value).unwrap();
        }
        
        "other"
    }
}
```

#### Step 3: Refactor for Better API
```rust
// Add warning logs
pub fn check_and_limit(&self, value: &str) -> &str {
    let mut seen = self.seen_values.lock().unwrap();
    
    if seen.contains(value) {
        return value;
    }
    
    if seen.len() < self.max_values {
        seen.insert(value.to_string());
        return seen.get(value).unwrap();
    }
    
    // Log warning when limit exceeded
    static WARNED: std::sync::Once = std::sync::Once::new();
    WARNED.call_once(|| {
        tracing::warn!(
            max_values = self.max_values,
            "Cardinality limit exceeded, new values will be grouped as 'other'"
        );
    });
    
    "other"
}
```

### Example 3: Histogram with Buckets

#### Step 1: Test First (RED)
```rust
#[test]
fn test_response_time_histogram() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    
    metrics::with_local_recorder(&recorder, || {
        // Record various response times
        record_response_time("/api/users", 0.010);  // 10ms
        record_response_time("/api/users", 0.025);  // 25ms
        record_response_time("/api/users", 0.150);  // 150ms
        record_response_time("/api/users", 0.500);  // 500ms
        record_response_time("/api/users", 2.000);  // 2s
        
        let snapshot = snapshotter.snapshot();
        
        // Check histogram data
        let histogram = snapshot.into_hashmap()
            .get(&format!("http_response_duration_seconds{{endpoint=\"/api/users\"}}"))
            .unwrap()
            .into_histogram()
            .unwrap();
        
        // Should have 5 samples
        assert_eq!(histogram.count(), 5);
        
        // Check bucketing
        let buckets = histogram.buckets();
        assert!(buckets.iter().any(|(le, count)| *le <= 0.01 && *count >= 1));
        assert!(buckets.iter().any(|(le, count)| *le <= 0.1 && *count >= 2));
        assert!(buckets.iter().any(|(le, count)| *le <= 1.0 && *count >= 4));
    });
}
```

#### Step 2: Implement (GREEN)
```rust
use metrics::histogram;

fn record_response_time(endpoint: &str, duration_seconds: f64) {
    histogram!("http_response_duration_seconds",
        "endpoint" => endpoint
    ).record(duration_seconds);
}
```

## Logging TDD Examples

### Example 1: Log Sanitization

#### Step 1: Test Sensitive Data is Sanitized (RED)
```rust
use tracing_test::traced_test;

#[traced_test]
#[test]
fn test_sensitive_data_sanitization() {
    // Create a log with sensitive data
    log_user_action(
        "user@example.com",
        "user_12345",
        "password123",
        "Login"
    );
    
    // Check logs don't contain sensitive data
    assert!(!logs_contain("user@example.com"));
    assert!(!logs_contain("user_12345"));
    assert!(!logs_contain("password123"));
    
    // Check logs contain sanitized versions
    assert!(logs_contain("<EMAIL>"));
    assert!(logs_contain("<USER_ID>"));
    assert!(logs_contain("<REDACTED>"));
}
```

#### Step 2: Implement Sanitization (GREEN)
```rust
use tracing::{info, instrument};

#[instrument(
    skip(password),  // Never log password
    fields(
        email = %"<EMAIL>",  // Override with sanitized value
        user_id = %"<USER_ID>",
        action = %action
    )
)]
fn log_user_action(
    email: &str,
    user_id: &str, 
    password: &str,
    action: &str,
) {
    info!("User action performed");
}
```

#### Step 3: Refactor with Sanitization Layer
```rust
pub struct SanitizingLayer;

impl<S> Layer<S> for SanitizingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Create visitor to modify fields
        let mut visitor = SanitizingVisitor::default();
        event.record(&mut visitor);
        
        // Forward sanitized event
        // Implementation details...
    }
}

struct SanitizingVisitor {
    fields: Vec<(&'static str, String)>,
}

impl Visit for SanitizingVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        let sanitized = match field.name() {
            "email" => "<EMAIL>",
            "password" => "<REDACTED>",
            "user_id" if value.starts_with("user_") => "<USER_ID>",
            _ => value,
        };
        
        self.fields.push((field.name(), sanitized.to_string()));
    }
}
```

### Example 2: Trace ID Propagation

#### Step 1: Test Trace ID Present (RED)
```rust
#[traced_test]
#[tokio::test]
async fn test_trace_id_propagation() {
    let trace_id = "test-trace-123";
    
    // Process request with trace ID
    handle_request_with_trace(trace_id).await;
    
    // All logs should contain the trace ID
    let log_output = logs_string();
    let log_lines: Vec<&str> = log_output.lines().collect();
    
    assert!(log_lines.len() > 0, "Should have logs");
    
    for line in log_lines {
        assert!(
            line.contains(trace_id),
            "Log line missing trace_id: {}",
            line
        );
    }
}
```

#### Step 2: Implement Trace ID (GREEN)
```rust
use tracing::Span;

async fn handle_request_with_trace(trace_id: &str) {
    let span = tracing::info_span!(
        "request",
        trace_id = %trace_id
    );
    
    let _enter = span.enter();
    
    info!("Request started");
    process_request_internal().await;
    info!("Request completed");
}

async fn process_request_internal() {
    info!("Processing request");
}
```

## Tracing TDD Examples

### Example 1: Span Creation

#### Step 1: Test Spans are Created (RED)
```rust
use opentelemetry::sdk::export::trace::SpanData;
use opentelemetry::sdk::trace::{Tracer, TracerProvider};

#[tokio::test]
async fn test_graphql_operation_creates_span() {
    // Setup test tracer
    let (tracer, receiver) = create_test_tracer();
    
    // Execute operation
    execute_graphql_query(
        &tracer,
        "query GetUser { user(id: \"123\") { name } }"
    ).await;
    
    // Check span was created
    let spans = receiver.try_recv().unwrap();
    assert_eq!(spans.len(), 1);
    
    let span = &spans[0];
    assert_eq!(span.name, "graphql.query");
    assert_eq!(
        span.attributes.get("graphql.operation_name"),
        Some(&"GetUser".into())
    );
}
```

#### Step 2: Implement Span Creation (GREEN)
```rust
use opentelemetry::trace::{Tracer, SpanKind};
use opentelemetry::KeyValue;

async fn execute_graphql_query(
    tracer: &dyn Tracer,
    query: &str,
) {
    let operation_name = extract_operation_name(query)
        .unwrap_or("unknown");
    
    let span = tracer
        .span_builder("graphql.query")
        .with_kind(SpanKind::Internal)
        .with_attributes(vec![
            KeyValue::new("graphql.operation_name", operation_name),
            KeyValue::new("graphql.query", query),
        ])
        .start(tracer);
    
    // Simulate query execution
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    span.end();
}

fn extract_operation_name(query: &str) -> Option<&str> {
    // Simple extraction for the test
    query.split_whitespace()
        .skip_while(|&w| w != "query" && w != "mutation")
        .nth(1)
}
```

### Example 2: Context Propagation

#### Step 1: Test Parent-Child Relationship (RED)
```rust
#[tokio::test]
async fn test_span_parent_child_relationship() {
    let (tracer, receiver) = create_test_tracer();
    
    // Create parent span
    let parent = tracer.start("parent_operation");
    let parent_context = Context::current_with_span(parent);
    
    // Create child within parent context
    let _guard = parent_context.attach();
    let child = tracer.start("child_operation");
    child.end();
    
    // Get spans
    let spans = receiver.try_recv().unwrap();
    assert_eq!(spans.len(), 2);
    
    // Find parent and child
    let parent_span = spans.iter().find(|s| s.name == "parent_operation").unwrap();
    let child_span = spans.iter().find(|s| s.name == "child_operation").unwrap();
    
    // Verify relationship
    assert_eq!(
        child_span.parent_span_id,
        parent_span.span_context.span_id()
    );
}
```

## Integration Test Example

### Complete Observability Test
```rust
#[tokio::test]
async fn test_complete_observability_integration() {
    // Setup all observability components
    let recorder = DebuggingRecorder::new();
    let metrics_snapshot = recorder.snapshotter();
    
    let (tracer, span_receiver) = create_test_tracer();
    
    // Initialize tracing with test subscriber
    let (log_writer, log_reader) = tracing_test::internal::create_writer_and_reader();
    
    // Execute a complete operation
    metrics::with_local_recorder(&recorder, || {
        let span = info_span!("test_request", trace_id = "test-123");
        let _enter = span.enter();
        
        info!("Starting request");
        
        // Simulate GraphQL request
        let start = Instant::now();
        
        counter!("graphql_requests_total",
            "operation" => "query",
            "status" => "success"
        ).increment(1);
        
        histogram!("graphql_request_duration_seconds")
            .record(start.elapsed().as_secs_f64());
        
        info!("Request completed");
    });
    
    // Verify metrics
    let metrics = metrics_snapshot.snapshot();
    assert!(metrics.into_hashmap().contains_key(
        &"graphql_requests_total{operation=\"query\",status=\"success\"}".to_string()
    ));
    
    // Verify logs
    let logs = log_reader.to_string();
    assert!(logs.contains("trace_id=test-123"));
    assert!(logs.contains("Starting request"));
    assert!(logs.contains("Request completed"));
    
    // Verify spans (if using OpenTelemetry)
    // let spans = span_receiver.try_recv().unwrap();
    // assert!(!spans.is_empty());
}
```

## Performance Testing

### Test Observability Overhead
```rust
#[bench]
fn bench_metrics_overhead(b: &mut Bencher) {
    let recorder = DebuggingRecorder::new();
    
    metrics::with_local_recorder(&recorder, || {
        b.iter(|| {
            counter!("bench_counter",
                "label1" => "value1",
                "label2" => "value2"
            ).increment(1);
        });
    });
}

#[bench]
fn bench_logging_overhead(b: &mut Bencher) {
    // Setup null subscriber for baseline
    let subscriber = tracing_subscriber::fmt()
        .with_writer(std::io::sink())
        .finish();
    
    tracing::subscriber::with_default(subscriber, || {
        b.iter(|| {
            info!(
                operation = "benchmark",
                value = 42,
                "Benchmark log message"
            );
        });
    });
}
```

## Test Utilities

### Helper Functions
```rust
/// Create a test metrics recorder
pub fn test_recorder() -> (DebuggingRecorder, Snapshotter) {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    (recorder, snapshotter)
}

/// Create a test tracer with span collector
pub fn create_test_tracer() -> (
    Box<dyn Tracer>,
    Receiver<Vec<SpanData>>
) {
    // Implementation depends on your OpenTelemetry setup
    todo!()
}

/// Assert metric exists with value
pub fn assert_metric_value(
    snapshot: &Snapshot,
    name: &str,
    labels: &[(&str, &str)],
    expected: u64,
) {
    let key = format_metric_key(name, labels);
    let metric = snapshot.into_hashmap()
        .get(&key)
        .expect(&format!("Metric {} not found", key));
    
    match metric {
        Metric::Counter(c) => assert_eq!(*c, expected),
        _ => panic!("Expected counter metric"),
    }
}

fn format_metric_key(name: &str, labels: &[(&str, &str)]) -> String {
    if labels.is_empty() {
        name.to_string()
    } else {
        let label_str = labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}{{{}}}", name, label_str)
    }
}
```

Remember: Always test your observability code! It's critical infrastructure that must work when you need it most.
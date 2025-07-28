# OpenTelemetry Tracing Guide

## What is Distributed Tracing?

Imagine following a single request as it travels through multiple services. Distributed tracing creates a "map" of this journey, showing:
- Where the request went
- How long each step took
- What errors occurred
- How services interacted

## Key Concepts

### Trace
A trace represents the entire journey of a request. It has:
- **Trace ID**: Unique identifier for the entire journey
- **Multiple Spans**: Individual operations within the journey

### Span
A span represents a single operation within a trace:
- **Span ID**: Unique identifier for this operation
- **Parent Span ID**: Links to the span that called this one
- **Start/End Time**: When the operation began and ended
- **Attributes**: Key-value pairs with extra information
- **Events**: Things that happened during the span
- **Status**: Success or error

### Context Propagation
How trace information passes between services:
```
Service A → [Trace-ID: 123] → Service B → [Trace-ID: 123] → Service C
```

## Setting Up OpenTelemetry

### 1. Add Dependencies

```toml
[dependencies]
# OpenTelemetry core
opentelemetry = { version = "0.21", features = ["trace"] }
opentelemetry_sdk = { version = "0.21", features = ["trace", "rt-tokio"] }

# OTLP exporter (sends to collectors)
opentelemetry-otlp = { version = "0.14", features = ["tonic"] }

# Integration with tracing crate
tracing-opentelemetry = "0.22"
opentelemetry-semantic-conventions = "0.13"
```

### 2. Initialize Tracing

```rust
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    runtime,
    trace::{self, RandomIdGenerator, Sampler},
    Resource,
};
use opentelemetry_otlp::WithExportConfig;

pub fn init_tracing(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Set up propagator for W3C Trace Context
    global::set_text_map_propagator(TraceContextPropagator::new());
    
    // Create OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint);
    
    // Configure trace pipeline
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::TraceIdRatioBased(0.1)) // Sample 10%
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(32)
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    KeyValue::new("deployment.environment", "production"),
                ])),
        )
        .install_batch(runtime::Tokio)?;
    
    // Set as global provider
    global::set_tracer_provider(tracer_provider);
    
    Ok(())
}
```

### 3. Connect with Tracing Crate

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry
    init_tracing("pcf-api", "http://localhost:4317")?;
    
    // Create OpenTelemetry layer for tracing
    let otel_layer = tracing_opentelemetry::layer();
    
    // Combine with other layers
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .init();
    
    Ok(())
}
```

## Creating Spans

### Basic Span Creation

```rust
use tracing::{info_span, instrument};

// Method 1: Using instrument macro
#[instrument(
    name = "process_order",
    fields(
        order.id = %order_id,
        customer.id = %customer_id,
        otel.kind = "internal"
    )
)]
async fn process_order(order_id: &str, customer_id: &str) -> Result<()> {
    info!("Processing order");
    
    // Span automatically created and closed
    validate_order(order_id).await?;
    charge_payment(order_id).await?;
    ship_order(order_id).await?;
    
    Ok(())
}

// Method 2: Manual span creation
async fn manual_span_example() {
    let span = info_span!(
        "manual_operation",
        operation = "data_processing",
        items_count = 42
    );
    
    let _enter = span.enter();
    // Do work within span
    info!("Processing data");
} // Span ends when _enter is dropped
```

### Nested Spans

```rust
#[instrument]
async fn handle_graphql_request(query: String) -> Result<Response> {
    // Parent span: handle_graphql_request
    
    // Child span 1: Parse
    let parsed = {
        let _span = info_span!("parse_query").entered();
        parse_graphql(&query)?
    };
    
    // Child span 2: Validate  
    let validated = {
        let _span = info_span!("validate_query").entered();
        validate_query(parsed)?
    };
    
    // Child span 3: Execute
    execute_query(validated).await
}

#[instrument(skip(query))]
async fn execute_query(query: ValidatedQuery) -> Result<Response> {
    // Nested under handle_graphql_request
    
    // Further nesting for each resolver
    let data = fetch_data(&query).await?;
    Ok(build_response(data))
}
```

## Adding Span Attributes

### Semantic Conventions

Use standard attribute names from OpenTelemetry semantic conventions:

```rust
use opentelemetry::trace::Span;
use opentelemetry_semantic_conventions as semcov;
use tracing::Span as TracingSpan;

#[instrument(fields(
    http.method = %req.method(),
    http.url = %req.uri(),
    http.target = %req.uri().path(),
    // Standard HTTP attributes
))]
async fn handle_http_request(req: Request<Body>) -> Response<Body> {
    let span = TracingSpan::current();
    
    // Process request...
    let response = process(req).await;
    
    // Add response attributes
    span.record("http.status_code", &response.status().as_u16());
    
    response
}

// Database operations
#[instrument(fields(
    db.system = "surrealdb",
    db.operation = "select",
    db.statement = %query,
))]
async fn database_query(query: &str) -> Result<Vec<Record>> {
    let span = TracingSpan::current();
    
    let start = Instant::now();
    let result = execute_query(query).await;
    
    // Record additional attributes
    span.record("db.rows_affected", &result.as_ref().map(|r| r.len()).unwrap_or(0));
    
    result
}
```

### Custom Attributes

```rust
use tracing::field;

#[instrument(fields(
    // Static attributes
    feature_flag = "new_algorithm",
    version = "v2",
    // Dynamic attributes (recorded later)
    processing_time_ms = field::Empty,
    items_processed = field::Empty,
))]
async fn process_batch(items: Vec<Item>) -> Result<()> {
    let span = Span::current();
    let start = Instant::now();
    
    // Process items...
    let processed_count = process_items(&items).await?;
    
    // Record dynamic values
    span.record("processing_time_ms", &start.elapsed().as_millis());
    span.record("items_processed", &processed_count);
    
    Ok(())
}
```

## Context Propagation

### HTTP Headers

```rust
use opentelemetry::global;
use opentelemetry::propagation::TextMapPropagator;
use tracing_opentelemetry::OpenTelemetrySpanExt;

// Inject trace context into outgoing request
async fn make_http_request(url: &str) -> Result<Response> {
    let mut headers = HeaderMap::new();
    
    // Get current span context
    let context = Span::current().context();
    
    // Inject into headers
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut HeaderInjector(&mut headers))
    });
    
    // Make request with headers
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await?;
    
    Ok(response)
}

// Extract trace context from incoming request
async fn handle_incoming_request(req: Request<Body>) -> Response<Body> {
    // Extract context from headers
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    });
    
    // Create span with parent context
    let span = info_span!(
        "handle_request",
        otel.kind = "server",
        http.method = %req.method(),
    );
    span.set_parent(parent_context);
    
    // Process within span
    async move {
        process_request(req).await
    }
    .instrument(span)
    .await
}
```

### Context Helpers

```rust
// Header injection helper
struct HeaderInjector<'a>(&'a mut HeaderMap);

impl<'a> opentelemetry::propagation::Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            self.0.insert(key, header_value);
        }
    }
}

// Header extraction helper
struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> opentelemetry::propagation::Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }
    
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
```

## Advanced Patterns

### Async Context Preservation

```rust
use tracing::Instrument;

// Ensure span context is preserved across async boundaries
async fn concurrent_operations() {
    let span = info_span!("batch_process");
    
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            // Each task gets its own span
            let task_span = info_span!(parent: &span, "process_item", item_id = i);
            
            tokio::spawn(
                async move {
                    process_item(i).await
                }
                .instrument(task_span)
            )
        })
        .collect();
    
    // Wait for all tasks
    let _results = futures::future::join_all(tasks).await;
}
```

### Error Recording

```rust
use opentelemetry::trace::{Status, StatusCode};

#[instrument]
async fn fallible_operation() -> Result<Data, Error> {
    let span = Span::current();
    
    match perform_operation().await {
        Ok(data) => {
            span.record("result", "success");
            Ok(data)
        }
        Err(e) => {
            // Record error details
            span.record("error", true);
            span.record("error.type", &format!("{:?}", e));
            span.record("error.message", &e.to_string());
            
            // Set span status
            span.set_status(Status::error(e.to_string()));
            
            error!(error = %e, "Operation failed");
            Err(e)
        }
    }
}
```

### Sampling Strategies

```rust
use opentelemetry_sdk::trace::{Sampler, SamplingDecision, SamplingResult};

// Custom sampler for important operations
struct PrioritySampler {
    base_rate: f64,
}

impl opentelemetry_sdk::trace::ShouldSample for PrioritySampler {
    fn should_sample(
        &self,
        parent_context: Option<&Context>,
        trace_id: TraceId,
        name: &str,
        span_kind: &SpanKind,
        attributes: &[KeyValue],
        links: &[Link],
    ) -> SamplingResult {
        // Always sample errors
        if attributes.iter().any(|kv| kv.key.as_str() == "error") {
            return SamplingResult {
                decision: SamplingDecision::RecordAndSample,
                attributes: vec![],
                trace_state: TraceState::default(),
            };
        }
        
        // Always sample slow operations
        if name.contains("critical") {
            return SamplingResult {
                decision: SamplingDecision::RecordAndSample,
                attributes: vec![],
                trace_state: TraceState::default(),
            };
        }
        
        // Otherwise use base rate
        let should_sample = (trace_id.to_bytes()[15] as f64 / 255.0) < self.base_rate;
        
        SamplingResult {
            decision: if should_sample {
                SamplingDecision::RecordAndSample
            } else {
                SamplingDecision::Drop
            },
            attributes: vec![],
            trace_state: TraceState::default(),
        }
    }
}
```

## Testing Traces

```rust
#[cfg(test)]
mod tests {
    use opentelemetry::sdk::trace::Tracer;
    use opentelemetry::trace::TracerProvider;
    
    #[tokio::test]
    async fn test_span_creation() {
        // Set up test tracer
        let provider = opentelemetry::sdk::trace::TracerProvider::builder()
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build();
        
        let tracer = provider.tracer("test");
        
        // Create and test span
        let span = tracer
            .span_builder("test_operation")
            .with_attributes(vec![
                KeyValue::new("test.name", "span_test"),
            ])
            .start(&tracer);
        
        // Add events
        span.add_event("test_event", vec![
            KeyValue::new("event.data", "test_value"),
        ]);
        
        span.end();
        
        // In real tests, verify span was exported correctly
    }
    
    #[tokio::test]
    async fn test_context_propagation() {
        let span = tracing::info_span!("parent_span");
        let _guard = span.enter();
        
        // Create child span
        let child_span = tracing::info_span!("child_span");
        
        // Verify parent relationship
        // (In real implementation, check span parent_id)
    }
}
```

## Common Issues and Solutions

### Issue: Spans not appearing in backend
**Solution**: Check exporter configuration:
```rust
// Verify endpoint is correct
println!("OTLP endpoint: {}", otlp_endpoint);

// Enable debug logging
std::env::set_var("OTEL_LOG_LEVEL", "debug");

// Check for export errors
let result = tracer_provider.force_flush();
if let Err(e) = result {
    eprintln!("Failed to export spans: {}", e);
}
```

### Issue: Missing parent-child relationships
**Solution**: Ensure context propagation:
```rust
// Always use .instrument() for async functions
async_function()
    .instrument(tracing::info_span!("span_name"))
    .await

// Or enter span before calling
let span = info_span!("operation");
let _enter = span.enter();
other_function().await;
```

### Issue: High overhead
**Solution**: Adjust sampling and span creation:
```rust
// Reduce sampling rate
.with_sampler(Sampler::TraceIdRatioBased(0.01)) // 1%

// Create fewer spans
if should_trace() {
    let span = info_span!("operation");
    // ...
}

// Limit attributes
.with_max_attributes_per_span(16)
```

## Production Checklist

1. ✅ Sampling configured (not 100%)
2. ✅ Sensitive data not in span attributes
3. ✅ Context propagation working
4. ✅ Error spans marked correctly
5. ✅ Semantic conventions followed
6. ✅ Span names descriptive
7. ✅ Critical paths always sampled
8. ✅ Export batching enabled
9. ✅ Timeouts configured
10. ✅ Resource attributes set

Remember: Traces are most valuable when they cross service boundaries!
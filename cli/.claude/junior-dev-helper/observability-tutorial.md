# Observability Tutorial - The Three Pillars

## What is Observability?

Observability is your ability to understand what's happening inside your application by looking at its outputs. Think of it like being a detective - you need clues (data) to solve problems.

The three pillars of observability are:
1. **Metrics** - Numbers that tell you "how much" or "how many"
2. **Logs** - Messages that tell you "what happened"
3. **Traces** - Maps that show you "where things went"

## Why Do We Need All Three?

Imagine debugging a slow API request:
- **Metrics** tell you: "95% of requests are taking > 2 seconds"
- **Logs** tell you: "User 123 made a request at 3:45pm that failed"
- **Traces** tell you: "The request spent 1.8s waiting for the database"

Each pillar answers different questions!

## Metrics - The Speedometer

Metrics are like the dashboard in your car. They give you numbers:
- Request count (how many requests?)
- Error rate (what percentage failed?)
- Response time (how fast are we?)

### Types of Metrics

1. **Counter** - Only goes up (like an odometer)
   ```rust
   counter!("api_requests_total").increment(1);
   ```

2. **Gauge** - Goes up and down (like a fuel gauge)
   ```rust
   gauge!("active_connections").set(42.0);
   ```

3. **Histogram** - Tracks distribution (like a speedometer history)
   ```rust
   histogram!("request_duration_seconds").record(1.5);
   ```

### Prometheus Format

Metrics are exposed in a special text format:
```
# HELP api_requests_total Total API requests
# TYPE api_requests_total counter
api_requests_total{method="GET",status="200"} 1234
api_requests_total{method="POST",status="500"} 5

# HELP request_duration_seconds Request latency
# TYPE request_duration_seconds histogram
request_duration_seconds_bucket{le="0.1"} 100
request_duration_seconds_bucket{le="0.5"} 450
request_duration_seconds_bucket{le="1.0"} 492
request_duration_seconds_bucket{le="+Inf"} 500
```

## Logs - The Diary

Logs are timestamped messages about events. In production, we use structured JSON logs:

```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "level": "INFO",
  "message": "User login successful",
  "trace_id": "abc123",
  "user_id": "<REDACTED>",
  "duration_ms": 145
}
```

### Log Levels

- **TRACE** - Very detailed debugging info (usually disabled)
- **DEBUG** - Debugging info for development
- **INFO** - General information about normal operations
- **WARN** - Something unexpected but handled
- **ERROR** - Something failed

### Security in Logs

NEVER log sensitive data:
```rust
// BAD - Exposes password!
info!("Login attempt: user={}, pass={}", username, password);

// GOOD - Safe logging
info!("Login attempt: user={}", username);
```

## Traces - The GPS

Traces show the journey of a request through your system. Each trace has:
- **Trace ID** - Unique identifier for the entire journey
- **Spans** - Individual steps in the journey
- **Parent-Child relationships** - How steps connect

### Trace Example

```
[Trace ID: xyz789]
├─ GraphQL Request (2.1s)
│  ├─ Parse Query (0.01s)
│  ├─ Validate Query (0.02s)
│  ├─ Authorization Check (0.15s)
│  │  └─ SpiceDB Call (0.14s)
│  └─ Execute Query (1.92s)
│     ├─ Database Query 1 (0.8s)
│     └─ Database Query 2 (1.1s)
```

## Putting It All Together

Here's how observability helps in practice:

### Scenario: API is slow

1. **Check Metrics Dashboard**
   - See p95 latency jumped from 200ms to 2s at 3:30pm
   - Error rate is normal (so it's not failing, just slow)

2. **Search Logs**
   ```
   level:WARN timestamp:[2024-01-15T15:30:00 TO 2024-01-15T15:35:00]
   ```
   - Find: "Database connection pool exhausted"

3. **Examine Traces**
   - Look at slow request traces
   - See all requests waiting for database connections
   - One trace shows a query taking 30s!

4. **Root Cause**: One bad query is hogging all connections

## Implementation in Our API

### 1. Metrics Setup

```rust
use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;

// Initialize Prometheus exporter
let builder = PrometheusBuilder::new();
let handle = builder.install_recorder()?;

// Record metrics
counter!("graphql_requests_total", 
    "operation" => "query",
    "status" => "success"
).increment(1);

// Track request duration
let start = Instant::now();
// ... handle request ...
histogram!("request_duration_seconds")
    .record(start.elapsed().as_secs_f64());
```

### 2. Structured Logging

```rust
use tracing::{info, error, instrument};

#[instrument(skip(password))] // Don't log password!
async fn login(username: &str, password: &str) -> Result<User> {
    info!("Login attempt starting");
    
    match validate_credentials(username, password).await {
        Ok(user) => {
            info!(user_id = %user.id, "Login successful");
            Ok(user)
        }
        Err(e) => {
            error!(error = %e, "Login failed");
            Err(e)
        }
    }
}
```

### 3. Distributed Tracing

```rust
use opentelemetry::trace::{Tracer, SpanKind};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[instrument]
async fn handle_graphql_request(query: String) -> Result<Response> {
    let span = tracing::Span::current();
    span.set_attribute("graphql.query", query.clone());
    
    // Parse phase
    let parse_result = span.in_scope(|| {
        info_span!("parse_query").in_scope(|| {
            parse_graphql_query(&query)
        })
    })?;
    
    // Execute phase
    let result = span.in_scope(|| {
        info_span!("execute_query").in_scope(|| async {
            execute_query(parse_result).await
        })
    }).await?;
    
    Ok(result)
}
```

## Common Patterns

### 1. Request Tracking

Every request should have a trace ID:
```rust
async fn handle_request(req: Request) -> Response {
    let trace_id = Uuid::new_v4().to_string();
    
    // Add to all logs in this request
    let span = info_span!("request", trace_id = %trace_id);
    let _enter = span.enter();
    
    info!("Request started");
    // ... handle request ...
    info!("Request completed");
}
```

### 2. Error Context

Always add context to errors:
```rust
db.query("SELECT * FROM users")
    .await
    .map_err(|e| {
        error!(
            error = %e,
            query = "SELECT * FROM users",
            "Database query failed"
        );
        e
    })?
```

### 3. Performance Tracking

Measure important operations:
```rust
async fn expensive_operation() -> Result<()> {
    let _timer = histogram!("expensive_operation_duration").start_timer();
    
    // When _timer drops, it records the duration
    do_expensive_work().await
}
```

## Best Practices

1. **Be Consistent**
   - Use the same metric names everywhere
   - Follow naming conventions (e.g., `service_component_unit`)
   - Keep log messages clear and searchable

2. **Control Cardinality**
   - Don't use unbounded values as labels (like user IDs)
   - Group similar things (HTTP 200/201/204 → "2xx")
   - Limit unique label combinations

3. **Sample Wisely**
   - Not every request needs a trace (sample 10%)
   - Debug logs can be sampled in production
   - Keep critical metrics at 100%

4. **Think About Queries**
   - Will someone search for this log?
   - Can this metric be graphed usefully?
   - Does this trace span add value?

## Testing Observability

Always test your observability code:

```rust
#[cfg(test)]
mod tests {
    use metrics_util::debugging::DebuggingRecorder;
    
    #[test]
    fn test_metrics_recorded() {
        let recorder = DebuggingRecorder::new();
        metrics::with_recorder(&recorder, || {
            counter!("test_counter").increment(1);
        });
        
        let metrics = recorder.collect();
        assert_eq!(metrics["test_counter"], 1);
    }
    
    #[test]
    fn test_log_sanitization() {
        // Test that sensitive data is redacted
        let output = capture_logs(|| {
            info!(email = "test@example.com", "User action");
        });
        
        assert!(!output.contains("test@example.com"));
        assert!(output.contains("<REDACTED>"));
    }
}
```

## Debugging Tips

1. **Missing Metrics?**
   - Check the recorder is initialized
   - Verify /metrics endpoint is accessible
   - Ensure labels are correct

2. **No Logs Appearing?**
   - Check RUST_LOG environment variable
   - Verify tracing subscriber is set up
   - Look for filters blocking your module

3. **Broken Traces?**
   - Ensure trace context propagates
   - Check span relationships
   - Verify exporter configuration

Remember: Observability is like insurance - you'll be glad you have it when things go wrong!
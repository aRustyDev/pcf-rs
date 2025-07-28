# Prometheus Metrics Guide

## What is Prometheus?

Prometheus is a monitoring system that collects numeric data (metrics) from your application. Think of it as a specialized database for time-series data - numbers that change over time.

## Understanding Metric Types

### 1. Counter - The Odometer
A counter only goes up (until restart). Use for counting things:

```rust
use metrics::counter;

// Increment by 1
counter!("http_requests_total").increment(1);

// With labels
counter!("http_requests_total",
    "method" => "POST",
    "endpoint" => "/graphql",
    "status" => "200"
).increment(1);
```

**Good for**: Request counts, error counts, bytes processed
**Not for**: Things that go down (use gauge)

### 2. Gauge - The Thermometer
A gauge can go up or down. Use for current values:

```rust
use metrics::gauge;

// Set current value
gauge!("memory_usage_bytes").set(1048576.0);

// Increment/decrement
gauge!("active_connections").increment(1.0);
gauge!("active_connections").decrement(1.0);
```

**Good for**: Active connections, queue size, temperature
**Not for**: Rates or totals (use counter)

### 3. Histogram - The Statistician
A histogram tracks distributions and calculates percentiles:

```rust
use metrics::histogram;

// Record a value
let duration = 0.125; // seconds
histogram!("request_duration_seconds").record(duration);

// With timer
let timer = histogram!("db_query_duration_seconds").start_timer();
// ... do work ...
drop(timer); // Automatically records elapsed time
```

**Good for**: Latencies, request sizes, processing times
**Not for**: Counts (use counter)

## Setting Up Prometheus Metrics

### 1. Initialize the Recorder

```rust
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;

pub fn init_metrics(port: u16) -> Result<PrometheusHandle, Box<dyn std::error::Error>> {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    
    let builder = PrometheusBuilder::new()
        .with_http_listener(addr)
        .add_global_label("service", "pcf-api")
        .add_global_label("environment", "production");
    
    let handle = builder.install_recorder()?;
    
    Ok(handle)
}
```

### 2. Create the /metrics Endpoint

```rust
use axum::{routing::get, Router, response::IntoResponse};

async fn metrics_handler(State(handle): State<PrometheusHandle>) -> impl IntoResponse {
    handle.render()
}

pub fn create_metrics_router(handle: PrometheusHandle) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(handle)
}
```

### 3. Record Metrics in Your Code

```rust
use metrics::{counter, histogram};
use std::time::Instant;

async fn handle_graphql_request(
    query: GraphQLQuery,
) -> Result<GraphQLResponse, Error> {
    let start = Instant::now();
    let operation_type = query.operation_type(); // "query" or "mutation"
    let operation_name = query.operation_name()
        .unwrap_or("unnamed");
    
    // Execute the query
    let result = match execute_query(query).await {
        Ok(response) => {
            counter!("graphql_requests_total",
                "type" => operation_type,
                "name" => operation_name,
                "status" => "success"
            ).increment(1);
            Ok(response)
        }
        Err(e) => {
            counter!("graphql_requests_total",
                "type" => operation_type,
                "name" => operation_name,
                "status" => "error"
            ).increment(1);
            
            counter!("graphql_errors_total",
                "type" => operation_type,
                "error_type" => classify_error(&e)
            ).increment(1);
            
            Err(e)
        }
    };
    
    // Record duration
    let duration = start.elapsed().as_secs_f64();
    histogram!("graphql_request_duration_seconds",
        "type" => operation_type
    ).record(duration);
    
    result
}
```

## Cardinality Control - CRITICAL!

Cardinality is the number of unique label combinations. High cardinality will crash Prometheus!

### The Problem

```rust
// BAD - Unbounded cardinality!
counter!("user_requests",
    "user_id" => user_id,  // Could be millions!
    "request_id" => request_id  // Unique every time!
).increment(1);
```

Each unique combination creates a new time series. With 1M users and 1M requests, that's 1 trillion time series! 💥

### The Solution

```rust
use std::collections::HashSet;
use std::sync::RwLock;

pub struct CardinalityLimiter {
    max_values: usize,
    seen_values: RwLock<HashSet<String>>,
}

impl CardinalityLimiter {
    pub fn new(max_values: usize) -> Self {
        Self {
            max_values,
            seen_values: RwLock::new(HashSet::new()),
        }
    }
    
    pub fn limit_label(&self, value: String) -> String {
        let mut seen = self.seen_values.write().unwrap();
        
        if seen.contains(&value) {
            return value;
        }
        
        if seen.len() < self.max_values {
            seen.insert(value.clone());
            value
        } else {
            "other".to_string()
        }
    }
}

// Usage
static OPERATION_LIMITER: Lazy<CardinalityLimiter> = 
    Lazy::new(|| CardinalityLimiter::new(50));

let operation_name = OPERATION_LIMITER.limit_label(
    query.operation_name().unwrap_or("unnamed").to_string()
);

counter!("graphql_requests_total",
    "operation" => &operation_name  // Limited to 50 unique values
).increment(1);
```

### Cardinality Best Practices

1. **Never use IDs as labels**
   ```rust
   // BAD
   counter!("requests", "user_id" => user_id);
   
   // GOOD - Use static categories
   counter!("requests", "user_type" => user.account_type());
   ```

2. **Bucket continuous values**
   ```rust
   fn bucket_status_code(code: u16) -> &'static str {
       match code {
           200..=299 => "2xx",
           300..=399 => "3xx",
           400..=499 => "4xx",
           500..=599 => "5xx",
           _ => "other",
       }
   }
   
   counter!("http_requests",
       "status" => bucket_status_code(status_code)
   ).increment(1);
   ```

3. **Limit operation names**
   ```rust
   // Limit GraphQL operation names to prevent explosion
   let operation = if KNOWN_OPERATIONS.contains(&operation_name) {
       operation_name
   } else {
       "other"
   };
   ```

## Common Metrics Patterns

### 1. RED Method (Request, Errors, Duration)

```rust
pub struct REDMetrics {
    requests: &'static str,
    errors: &'static str,
    duration: &'static str,
}

impl REDMetrics {
    pub fn record_success(&self, duration: f64, labels: &[(&str, &str)]) {
        counter!(self.requests, labels).increment(1);
        histogram!(self.duration, labels).record(duration);
    }
    
    pub fn record_error(&self, duration: f64, labels: &[(&str, &str)]) {
        counter!(self.requests, labels).increment(1);
        counter!(self.errors, labels).increment(1);
        histogram!(self.duration, labels).record(duration);
    }
}

// Usage
static GRAPHQL_METRICS: REDMetrics = REDMetrics {
    requests: "graphql_requests_total",
    errors: "graphql_errors_total",
    duration: "graphql_request_duration_seconds",
};
```

### 2. Connection Pool Metrics

```rust
pub fn record_pool_metrics(pool: &ConnectionPool) {
    gauge!("db_pool_connections_active").set(pool.active() as f64);
    gauge!("db_pool_connections_idle").set(pool.idle() as f64);
    gauge!("db_pool_connections_total").set(pool.size() as f64);
    
    if let Some(wait_time) = pool.avg_wait_time() {
        histogram!("db_pool_wait_duration_seconds")
            .record(wait_time.as_secs_f64());
    }
}
```

### 3. Business Metrics

```rust
// Track business events
counter!("notes_created_total").increment(1);
counter!("users_registered_total").increment(1);
histogram!("note_content_length_bytes").record(note.content.len() as f64);

// Track feature usage
counter!("feature_used_total",
    "feature" => "dark_mode",
    "user_type" => "premium"
).increment(1);
```

## Performance Considerations

### 1. Pre-compute Labels

```rust
// BAD - Allocates on every call
counter!("requests", 
    "endpoint" => format!("/api/v1{}", path)
).increment(1);

// GOOD - Static string
let endpoint = match path {
    "/users" => "/api/v1/users",
    "/notes" => "/api/v1/notes",
    _ => "/api/v1/other",
};
counter!("requests", "endpoint" => endpoint).increment(1);
```

### 2. Use Atomic Operations

```rust
use std::sync::atomic::{AtomicU64, Ordering};

// For hot paths, consider atomic counters
static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);

// Increment atomically
REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);

// Periodically sync to Prometheus
fn sync_metrics() {
    let count = REQUEST_COUNT.swap(0, Ordering::Relaxed);
    counter!("requests_total").increment(count);
}
```

### 3. Sample Expensive Metrics

```rust
use rand::Rng;

// Only record expensive metrics for 10% of requests
if rand::thread_rng().gen_ratio(1, 10) {
    histogram!("expensive_calculation_duration")
        .record(duration);
}
```

## Testing Metrics

```rust
#[cfg(test)]
mod tests {
    use metrics_util::debugging::{DebuggingRecorder, Snapshotter};
    use metrics::{counter, histogram};
    
    #[test]
    fn test_request_metrics() {
        // Set up test recorder
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        metrics::set_recorder(recorder).unwrap();
        
        // Your code that records metrics
        handle_request("GET", "/api/users", 200, 0.1);
        
        // Verify metrics
        let snapshot = snapshotter.snapshot();
        
        assert_eq!(
            snapshot.counter_value(
                "http_requests_total",
                &[("method", "GET"), ("status", "2xx")]
            ),
            Some(1)
        );
        
        let histogram_data = snapshot.histogram_data(
            "request_duration_seconds",
            &[("method", "GET")]
        ).unwrap();
        
        assert_eq!(histogram_data.count(), 1);
        assert_eq!(histogram_data.sum(), 0.1);
    }
}
```

## Querying Metrics

Once metrics are exposed at `/metrics`, you can query them:

```bash
# Get raw metrics
curl http://localhost:9090/metrics

# In Prometheus, useful queries:
# Request rate (requests per second)
rate(graphql_requests_total[5m])

# Error rate percentage  
100 * sum(rate(graphql_errors_total[5m])) 
  / sum(rate(graphql_requests_total[5m]))

# 95th percentile latency
histogram_quantile(0.95, 
  rate(graphql_request_duration_seconds_bucket[5m]))

# Cardinality check - number of series
count(count by (__name__)({__name__=~".+"}))
```

## Common Issues and Solutions

### Issue: Metrics not appearing
**Solution**: Ensure the recorder is initialized and stored:
```rust
// Store handle to prevent drop
let metrics_handle = init_metrics(9090)?;

// Add to app state
let app_state = AppState {
    metrics: metrics_handle,
    // ...
};
```

### Issue: High memory usage
**Solution**: Check cardinality:
```bash
# Count unique label combinations
curl http://localhost:9090/metrics | grep -v "^#" | cut -d'{' -f1 | sort | uniq -c | sort -rn | head -20
```

### Issue: Metrics drift over time
**Solution**: Use consistent label values:
```rust
// Define constants for label values
const STATUS_SUCCESS: &str = "success";
const STATUS_ERROR: &str = "error";
const OP_TYPE_QUERY: &str = "query";
const OP_TYPE_MUTATION: &str = "mutation";
```

Remember: Keep cardinality low, label values consistent, and always test your metrics!
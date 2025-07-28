# Cardinality Control Guide

## What is Cardinality?

Cardinality is the number of unique combinations of label values for a metric. It's the #1 cause of Prometheus performance problems!

### Example: How Cardinality Explodes

```rust
// Metric: api_requests_total
// Labels: method, endpoint, user_id, status

// If you have:
// - 5 methods (GET, POST, PUT, DELETE, PATCH)
// - 20 endpoints
// - 10,000 users
// - 5 status codes (200, 400, 404, 500, 503)

// Total cardinality = 5 × 20 × 10,000 × 5 = 5,000,000 series!
```

Each series uses ~3KB of memory. 5 million series = 15GB of RAM just for one metric! 💥

## Why High Cardinality is Bad

1. **Memory Usage**: Each series needs storage
2. **Query Performance**: More series = slower queries
3. **Ingestion Rate**: Can overwhelm Prometheus
4. **Cost**: Cloud providers charge by series
5. **Crashes**: OOM kills when memory exhausted

## Identifying High Cardinality

### Check Current Cardinality

```bash
# Total series count
curl -s http://localhost:9090/api/v1/query?query=prometheus_tsdb_symbol_table_size_bytes | jq

# Series per metric
curl -s http://localhost:9090/api/v1/label/__name__/values | jq -r '.data[]' | while read metric; do
  count=$(curl -s "http://localhost:9090/api/v1/query?query=count(count+by(__name__)($metric))" | jq '.data.result[0].value[1]' 2>/dev/null || echo 0)
  echo "$count $metric"
done | sort -rn | head -20

# Find problematic labels
curl -s http://localhost:9090/api/v1/query?query=count(count+by(label_name)(metric_name)) | jq
```

### Warning Signs

```rust
// 🚨 BAD: Unbounded cardinality
counter!("requests", 
    "user_id" => user_id,        // Could be millions
    "request_id" => request_id,   // Always unique
    "timestamp" => timestamp,     // Infinite values
    "full_path" => request.path() // Unlimited paths
);

// ✅ GOOD: Bounded cardinality  
counter!("requests",
    "method" => method,           // ~5 values
    "endpoint" => endpoint_group, // ~50 values
    "status" => status_bucket,    // ~5 values
    "user_type" => user_type     // ~3 values
);
```

## Cardinality Control Strategies

### 1. Label Limiting Pattern

```rust
use std::sync::Mutex;
use std::collections::HashSet;

pub struct LabelLimiter {
    max_values: usize,
    seen_values: Mutex<HashSet<String>>,
}

impl LabelLimiter {
    pub fn new(max_values: usize) -> Self {
        Self {
            max_values,
            seen_values: Mutex::new(HashSet::new()),
        }
    }
    
    pub fn limit(&self, value: String) -> String {
        let mut seen = self.seen_values.lock().unwrap();
        
        // Already seen? Use it
        if seen.contains(&value) {
            return value;
        }
        
        // Under limit? Add it
        if seen.len() < self.max_values {
            seen.insert(value.clone());
            return value;
        }
        
        // Over limit? Return "other"
        warn!("Label limit exceeded for value: {}", value);
        "other".to_string()
    }
}

// Usage
static ENDPOINT_LIMITER: Lazy<LabelLimiter> = 
    Lazy::new(|| LabelLimiter::new(100));

let endpoint = ENDPOINT_LIMITER.limit(request.path().to_string());
counter!("requests", "endpoint" => &endpoint).increment(1);
```

### 2. Bucketing Pattern

```rust
/// Group continuous values into buckets
pub mod buckets {
    pub fn status_code(code: u16) -> &'static str {
        match code {
            200..=299 => "2xx",
            300..=399 => "3xx", 
            400..=499 => "4xx",
            500..=599 => "5xx",
            _ => "other",
        }
    }
    
    pub fn response_size(bytes: usize) -> &'static str {
        match bytes {
            0..=1_000 => "1KB",
            1_001..=10_000 => "10KB",
            10_001..=100_000 => "100KB",
            100_001..=1_000_000 => "1MB",
            _ => "1MB+",
        }
    }
    
    pub fn duration(ms: u64) -> &'static str {
        match ms {
            0..=10 => "10ms",
            11..=50 => "50ms",
            51..=100 => "100ms",
            101..=500 => "500ms",
            501..=1000 => "1s",
            1001..=5000 => "5s",
            _ => "5s+",
        }
    }
}

// Usage
histogram!("response_size_bytes",
    "size_bucket" => buckets::response_size(body.len())
).record(body.len() as f64);
```

### 3. Allowlist Pattern

```rust
/// Only track known values
pub struct AllowlistLabels {
    allowed_operations: HashSet<&'static str>,
    allowed_endpoints: HashSet<&'static str>,
}

impl AllowlistLabels {
    pub fn new() -> Self {
        let mut allowed_operations = HashSet::new();
        allowed_operations.insert("getUser");
        allowed_operations.insert("listNotes");
        allowed_operations.insert("createNote");
        // ... add all known operations
        
        let mut allowed_endpoints = HashSet::new();
        allowed_endpoints.insert("/api/v1/users");
        allowed_endpoints.insert("/api/v1/notes");
        // ... add all known endpoints
        
        Self {
            allowed_operations,
            allowed_endpoints,
        }
    }
    
    pub fn normalize_operation(&self, op: &str) -> &'static str {
        if self.allowed_operations.contains(op) {
            // Safe because we know it's in the static set
            self.allowed_operations.get(op).unwrap()
        } else {
            "unknown"
        }
    }
    
    pub fn normalize_endpoint(&self, endpoint: &str) -> &'static str {
        // Try exact match first
        if self.allowed_endpoints.contains(endpoint) {
            return self.allowed_endpoints.get(endpoint).unwrap();
        }
        
        // Try pattern matching
        if endpoint.starts_with("/api/v1/users/") {
            return "/api/v1/users/:id";
        }
        if endpoint.starts_with("/api/v1/notes/") {
            return "/api/v1/notes/:id";
        }
        
        "other"
    }
}
```

### 4. Dynamic Cardinality Monitoring

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CardinalityMonitor {
    limits: HashMap<String, usize>,
    counts: Mutex<HashMap<String, HashSet<String>>>,
    warnings_emitted: AtomicUsize,
}

impl CardinalityMonitor {
    pub fn new() -> Self {
        let mut limits = HashMap::new();
        limits.insert("endpoint".to_string(), 100);
        limits.insert("operation_name".to_string(), 50);
        limits.insert("error_type".to_string(), 20);
        
        Self {
            limits,
            counts: Mutex::new(HashMap::new()),
            warnings_emitted: AtomicUsize::new(0),
        }
    }
    
    pub fn check_and_record(&self, label: &str, value: &str) -> bool {
        let mut counts = self.counts.lock().unwrap();
        let values = counts.entry(label.to_string()).or_insert_with(HashSet::new);
        
        values.insert(value.to_string());
        
        if let Some(&limit) = self.limits.get(label) {
            if values.len() > limit {
                let warnings = self.warnings_emitted.fetch_add(1, Ordering::Relaxed);
                
                // Emit warning every 100 occurrences
                if warnings % 100 == 0 {
                    error!(
                        label = label,
                        cardinality = values.len(),
                        limit = limit,
                        "Cardinality limit exceeded!"
                    );
                    
                    // Emit metric about the cardinality issue
                    counter!("cardinality_limit_exceeded",
                        "label" => label
                    ).increment(1);
                }
                
                return false; // Don't use this value
            }
        }
        
        true
    }
    
    pub fn get_cardinality_metrics(&self) -> Vec<(String, usize)> {
        let counts = self.counts.lock().unwrap();
        counts.iter()
            .map(|(label, values)| (label.clone(), values.len()))
            .collect()
    }
}

// Global monitor
static CARDINALITY_MONITOR: Lazy<CardinalityMonitor> = 
    Lazy::new(CardinalityMonitor::new);

// Usage in metrics
let operation = if CARDINALITY_MONITOR.check_and_record("operation_name", &op_name) {
    op_name
} else {
    "other"
};
```

### 5. Hash-Based Limiting

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Deterministically limit cardinality using hashing
pub fn hash_limit(value: &str, max_buckets: u64) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    let hash = hasher.finish();
    
    let bucket = hash % max_buckets;
    format!("bucket_{}", bucket)
}

// Example: Limit user IDs to 100 buckets
let user_bucket = hash_limit(&user_id, 100);
counter!("user_activity",
    "user_bucket" => &user_bucket,
    "action" => action
).increment(1);
```

## Best Practices

### 1. Design Metrics Upfront

```rust
// Document your metrics and their expected cardinality
pub struct MetricDesign {
    name: &'static str,
    labels: Vec<LabelDesign>,
    expected_cardinality: usize,
}

pub struct LabelDesign {
    name: &'static str,
    max_values: usize,
    examples: Vec<&'static str>,
}

// Example
const METRICS: &[MetricDesign] = &[
    MetricDesign {
        name: "http_requests_total",
        labels: vec![
            LabelDesign {
                name: "method",
                max_values: 10,
                examples: vec!["GET", "POST", "PUT"],
            },
            LabelDesign {
                name: "endpoint",
                max_values: 50,
                examples: vec!["/api/users", "/api/notes"],
            },
            LabelDesign {
                name: "status",
                max_values: 5,
                examples: vec!["2xx", "4xx", "5xx"],
            },
        ],
        expected_cardinality: 10 * 50 * 5, // 2,500
    },
];
```

### 2. Regular Cardinality Audits

```rust
/// Run periodically to check cardinality health
pub async fn audit_cardinality(metrics_url: &str) -> Result<()> {
    let response = reqwest::get(format!("{}/metrics", metrics_url)).await?;
    let body = response.text().await?;
    
    let mut metric_counts: HashMap<String, usize> = HashMap::new();
    
    for line in body.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        
        if let Some(metric_name) = line.split('{').next() {
            *metric_counts.entry(metric_name.to_string()).or_insert(0) += 1;
        }
    }
    
    // Alert on high cardinality
    for (metric, count) in metric_counts {
        if count > 1000 {
            error!(
                metric = metric,
                cardinality = count,
                "High cardinality metric detected!"
            );
        }
    }
    
    Ok(())
}
```

### 3. Testing Cardinality

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cardinality_limits() {
        let limiter = LabelLimiter::new(3);
        
        assert_eq!(limiter.limit("a".to_string()), "a");
        assert_eq!(limiter.limit("b".to_string()), "b");
        assert_eq!(limiter.limit("c".to_string()), "c");
        assert_eq!(limiter.limit("d".to_string()), "other");
        assert_eq!(limiter.limit("e".to_string()), "other");
        
        // Verify existing values still work
        assert_eq!(limiter.limit("a".to_string()), "a");
    }
    
    #[test]
    fn test_expected_cardinality() {
        // Calculate worst-case cardinality
        let methods = 5;
        let endpoints = 50;
        let statuses = 5;
        let user_types = 3;
        
        let total = methods * endpoints * statuses * user_types;
        
        assert!(total < 10000, "Cardinality too high: {}", total);
    }
}
```

## Cardinality Reduction Techniques

### Before (High Cardinality)
```rust
counter!("api_requests",
    "user_id" => user_id,              // 1M+ values
    "session_id" => session_id,        // Always unique
    "full_path" => request.path(),     // Infinite
    "timestamp" => timestamp.to_string(), // Infinite
    "ip_address" => client_ip,         // 100K+ values
    "user_agent" => user_agent,        // 1000s of values
    "response_time_ms" => duration.as_millis().to_string() // Infinite
);
```

### After (Low Cardinality)
```rust
counter!("api_requests",
    "user_type" => classify_user(&user_id),     // 3 values: "free", "premium", "enterprise"
    "endpoint_group" => group_endpoint(path),    // 20 values: "/api/users", "/api/notes", etc
    "status_code" => bucket_status(status),      // 5 values: "2xx", "3xx", "4xx", "5xx", "other"
    "country" => geoip_country(&client_ip),      // ~200 values
    "client_type" => classify_user_agent(&ua),   // 10 values: "mobile", "desktop", "bot", etc
);

// Track response time as histogram, not label
histogram!("api_response_duration_seconds").record(duration.as_secs_f64());
```

## Emergency Cardinality Fixes

If Prometheus is already struggling:

```rust
// 1. Emergency cardinality cap
pub fn emergency_limit(label: &str, value: &str) -> &'static str {
    // Drastically reduce cardinality
    match label {
        "endpoint" => "all_endpoints",
        "operation" => "all_operations", 
        "user_id" => "all_users",
        _ => "other",
    }
}

// 2. Disable detailed metrics temporarily
if CARDINALITY_EMERGENCY.load(Ordering::Relaxed) {
    // Only count, no labels
    counter!("requests_total").increment(1);
    return;
}

// 3. Implement circuit breaker
struct MetricsCircuitBreaker {
    error_count: AtomicUsize,
    is_open: AtomicBool,
}

impl MetricsCircuitBreaker {
    fn record_metric(&self, f: impl FnOnce()) {
        if self.is_open.load(Ordering::Relaxed) {
            return; // Skip metrics
        }
        
        // Try to record
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(_) => {
                self.error_count.store(0, Ordering::Relaxed);
            }
            Err(_) => {
                let errors = self.error_count.fetch_add(1, Ordering::Relaxed);
                if errors > 10 {
                    self.is_open.store(true, Ordering::Relaxed);
                    error!("Metrics circuit breaker opened!");
                }
            }
        }
    }
}
```

## Monitoring Cardinality

Add these metrics to track cardinality:

```rust
// Track cardinality growth
gauge!("metric_cardinality",
    "metric_name" => "http_requests_total"
).set(current_cardinality as f64);

// Alert when approaching limits
if cardinality > limit * 0.8 {
    counter!("cardinality_warning",
        "metric" => metric_name,
        "severity" => "high"
    ).increment(1);
}
```

## Prometheus Queries for Cardinality

```promql
# Top 10 metrics by cardinality
topk(10, count by (__name__)({__name__=~".+"}))

# Growth rate of series
rate(prometheus_tsdb_symbol_table_size_bytes[5m])

# Memory usage per metric
topk(10, 
  count by (__name__)({__name__=~".+"}) 
  * avg(avg_over_time(prometheus_tsdb_bytes_per_sample[1h]))
)

# Find metrics with most label combinations
topk(10, count by (__name__)(
  label_cardinality({__name__=~".+"})
))
```

Remember: Every label value combination is a new time series. Keep cardinality under control from day one!
# Metrics Specification

## Core Principles

1. **Cardinality Control**: MUST limit unique label combinations to prevent metric explosion
2. **Performance**: Metric collection MUST NOT significantly impact request latency
3. **Security**: MUST NOT expose sensitive data in metric labels or values
4. **Completeness**: Every significant operation MUST be instrumented
5. **Actionability**: Metrics MUST enable effective alerting and debugging

## Metrics Export

**Endpoint**: `/metrics`
**Format**: Prometheus text format
**Access Control**:
- MUST NOT require authentication (for scraper compatibility)
- SHOULD be restricted via network policies or firewall rules
- MAY implement IP allowlist in application (configurable)
- MUST NOT expose sensitive business data

## Core Metrics

### GraphQL Request Metrics

**graphql_request_total**
- Type: Counter
- Labels: 
  - `operation_type`: query|mutation|subscription
  - `operation_name`: Limited to top 50 operations (others as "other")
  - `status`: success|error
- Cardinality limit: 300 (3 types × 50 operations × 2 statuses)
- Description: Total number of GraphQL requests

**graphql_request_duration_seconds**
- Type: Histogram
- Labels:
  - `operation_type`: query|mutation|subscription
  - `operation_name`: getNotes|createNote|etc
- Buckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]
- Description: GraphQL request duration in seconds

**graphql_field_resolution_duration_seconds**
- Type: Histogram
- Labels:
  - `field_name`: Limited to top 20 slowest fields (others as "other")
  - `parent_type`: Limited to concrete types (no interfaces)
- Buckets: [0.001, 0.005, 0.01, 0.05, 0.1, 0.5]
- Description: Time to resolve individual GraphQL fields
- Cardinality limit: 100 (20 fields × 5 parent types)
- Collection: Only for fields taking > 1ms

**graphql_active_subscriptions**
- Type: Gauge
- Labels:
  - `subscription_name`: noteCreated|noteUpdated|etc
- Description: Number of active GraphQL subscriptions

**graphql_errors_total**
- Type: Counter
- Labels:
  - `error_code`: Limited to defined error codes (max 15)
  - `operation_type`: query|mutation|subscription
  - `operation_name`: Limited to top 50 operations
- Description: Total GraphQL errors by type
- Cardinality limit: 2250 (15 codes × 3 types × 50 operations)
- MUST use "UNKNOWN" for undefined error codes

### HTTP Metrics

**http_request_total**
- Type: Counter
- Labels:
  - `method`: GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS
  - `path`: Limited to defined routes (max 10)
  - `status_code`: Bucketed as 2xx|3xx|4xx|5xx
- Cardinality limit: 280 (7 methods × 10 paths × 4 status buckets)
- Description: Total HTTP requests

**http_request_duration_seconds**
- Type: Histogram
- Labels:
  - `method`: GET|POST|etc
  - `path`: /graphql|/health|/metrics
- Buckets: [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1, 5]
- Description: HTTP request duration

### Database Metrics

**database_connection_pool_size**
- Type: Gauge
- Labels:
  - `database`: surrealdb|spicedb
- Description: Current size of connection pool

**database_connection_pool_active**
- Type: Gauge
- Labels:
  - `database`: surrealdb|spicedb
- Description: Active connections in pool

**database_connection_pool_idle**
- Type: Gauge
- Labels:
  - `database`: surrealdb|spicedb
- Description: Idle connections in pool

**database_query_total**
- Type: Counter
- Labels:
  - `database`: surrealdb|spicedb
  - `operation`: select|insert|update|delete
  - `status`: success|error
- Description: Total database queries

**database_query_duration_seconds**
- Type: Histogram
- Labels:
  - `database`: surrealdb|spicedb
  - `operation`: select|insert|update|delete
- Buckets: [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1, 5]
- Description: Database query duration

### External Service Metrics

**external_service_health**
- Type: Gauge
- Labels:
  - `service`: surrealdb|spicedb|hydra
- Values: 0 (unhealthy), 1 (healthy)
- Description: Health status of external services

**external_service_latency_seconds**
- Type: Histogram
- Labels:
  - `service`: surrealdb|spicedb|hydra
  - `operation`: health_check|query|mutation
- Buckets: [0.01, 0.05, 0.1, 0.5, 1, 2.5, 5, 10]
- Description: Latency of external service calls

### System Metrics

**process_open_fds**
- Type: Gauge
- Description: Number of open file descriptors

**process_resident_memory_bytes**
- Type: Gauge
- Description: Resident memory size in bytes

**process_cpu_seconds_total**
- Type: Counter
- Description: Total user and system CPU time spent

**process_start_time_seconds**
- Type: Gauge
- Description: Start time of the process since unix epoch

### Business Metrics (Demo)

**notes_created_total**
- Type: Counter
- Labels:
  - `author_bucket`: Hash-based buckets ("bucket_0" to "bucket_9")
- Description: Total notes created
- Cardinality limit: 10 buckets
- MUST NOT expose actual author names

**notes_deleted_total**
- Type: Counter
- Description: Total notes deleted

**notes_total**
- Type: Gauge
- Description: Current total number of notes

## Implementation Patterns

### Metric Collection
```rust
use metrics::{counter, histogram, gauge, Unit};

// In resolver
async fn create_note(&self, ctx: &Context<'_>, input: CreateNoteInput) -> Result<Note> {
    let start = Instant::now();
    
    counter!("graphql_request_total", 
        "operation_type" => "mutation",
        "operation_name" => "createNote",
        "status" => "started"
    ).increment(1);
    
    let result = match perform_create_note(ctx, input).await {
        Ok(note) => {
            counter!("notes_created_total",
                "author" => bucket_author(&note.author)
            ).increment(1);
            
            gauge!("notes_total").increment(1.0);
            
            Ok(note)
        }
        Err(e) => {
            counter!("graphql_errors_total",
                "error_code" => error_code(&e),
                "operation_type" => "mutation",
                "operation_name" => "createNote"
            ).increment(1);
            
            Err(e)
        }
    };
    
    histogram!("graphql_request_duration_seconds",
        "operation_type" => "mutation",
        "operation_name" => "createNote"
    ).record(start.elapsed().as_secs_f64());
    
    result
}
```

### Cardinality Control Implementation

```rust
use std::sync::RwLock;
use std::collections::HashSet;

// Track and limit operation names
struct MetricLimiter {
    operation_names: RwLock<HashSet<String>>,
    max_operations: usize,
}

impl MetricLimiter {
    fn get_operation_label(&self, operation: &str) -> &str {
        let names = self.operation_names.read().unwrap();
        if names.contains(operation) {
            return operation;
        }
        drop(names);
        
        let mut names = self.operation_names.write().unwrap();
        if names.len() < self.max_operations {
            names.insert(operation.to_string());
            operation
        } else {
            warn!("Operation {} exceeds cardinality limit", operation);
            "other"
        }
    }
}

// Hash-based bucketing for high-cardinality fields
fn bucket_by_hash(value: &str, num_buckets: u32) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    let bucket = (hasher.finish() % num_buckets as u64) as u32;
    format!("bucket_{}", bucket)
}

// Status code bucketing
fn bucket_status_code(status: u16) -> &'static str {
    match status {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    }
}

// Dynamic cardinality monitoring
struct CardinalityMonitor {
    limits: HashMap<&'static str, usize>,
    current: RwLock<HashMap<&'static str, HashSet<String>>>,
}

impl CardinalityMonitor {
    fn check_cardinality(&self, metric: &'static str, labels: &str) -> bool {
        let mut current = self.current.write().unwrap();
        let entries = current.entry(metric).or_insert_with(HashSet::new);
        
        if entries.len() >= self.limits[metric] {
            if !entries.contains(labels) {
                error!(
                    "Metric {} exceeds cardinality limit of {}",
                    metric, self.limits[metric]
                );
                return false;
            }
        }
        
        entries.insert(labels.to_string());
        true
    }
}
```

### Global Labels and Initialization

```rust
// MUST limit global labels to prevent cardinality explosion
const MAX_GLOBAL_LABELS: usize = 6;

// Set during initialization
fn init_metrics(config: &Config) -> Result<()> {
    // Validate cardinality limits
    let cardinality_limits = HashMap::from([
        ("graphql_request_total", 300),
        ("graphql_errors_total", 2250),
        ("http_request_total", 280),
        ("graphql_field_resolution_duration_seconds", 100),
    ]);
    
    let recorder = PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9090))
        .add_global_label("service", "pcf-api")
        .add_global_label("environment", &config.environment)
        .add_global_label("instance", &get_instance_id())
        .add_global_label("version", env!("CARGO_PKG_VERSION"))
        .add_global_label("region", &config.region.unwrap_or("unknown"))
        .install()?;
        
    // Initialize cardinality monitoring
    CARDINALITY_MONITOR.init(cardinality_limits);
    
    Ok(())
}

// Instance ID generation (stable across restarts)
fn get_instance_id() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("INSTANCE_ID"))
        .unwrap_or_else(|_| {
            // Hash MAC address for stable ID
            let macs = get_mac_addresses();
            if let Some(mac) = macs.first() {
                format!("{:x}", hash(mac) % 10000)
            } else {
                "unknown".to_string()
            }
        })
}
```

### Performance Considerations

```rust
// Use atomic metrics for hot paths
use std::sync::atomic::{AtomicU64, Ordering};

struct FastMetrics {
    requests: AtomicU64,
    errors: AtomicU64,
    // Flush to Prometheus periodically
}

// Sampling for expensive metrics
struct SampledHistogram {
    histogram: Histogram,
    sample_rate: f64,
}

impl SampledHistogram {
    fn record(&self, value: f64) {
        if rand::random::<f64>() < self.sample_rate {
            self.histogram.record(value * (1.0 / self.sample_rate));
        }
    }
}
```

## Monitoring Best Practices

### Cardinality Monitoring

```prometheus
# Alert on high cardinality
- alert: HighMetricCardinality
  expr: prometheus_tsdb_symbol_table_size_bytes > 10000000  # 10MB
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Prometheus cardinality is high"
    description: "Consider reducing label dimensions"

# Monitor specific metric cardinality
- alert: GraphQLOperationCardinality
  expr: count(count by (operation_name)(graphql_request_total)) > 50
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Too many unique GraphQL operations being tracked"
```

### Metric Lifecycle Management

```rust
// Periodically clean up old metrics
fn cleanup_stale_metrics() {
    let stale_threshold = Duration::from_hours(24);
    
    METRICS_REGISTRY.retain(|metric, last_updated| {
        if last_updated.elapsed() > stale_threshold {
            info!("Removing stale metric: {}", metric);
            false
        } else {
            true
        }
    });
}

// Run cleanup job
tokio::spawn(async {
    let mut interval = tokio::time::interval(Duration::from_hours(1));
    loop {
        interval.tick().await;
        cleanup_stale_metrics();
    }
});
```

## Alerting Examples

```yaml
groups:
  - name: pcf-api
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(graphql_errors_total[5m])) / 
          sum(rate(graphql_request_total[5m])) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High GraphQL error rate (>5%)"
          
      - alert: SlowRequests
        expr: |
          histogram_quantile(0.95, 
            rate(graphql_request_duration_seconds_bucket[5m])
          ) > 1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "95th percentile latency above 1s"
          
      - alert: DatabaseDown
        expr: external_service_health{service="surrealdb"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "SurrealDB has been down for 5 minutes"
          
      - alert: HighMemoryUsage
        expr: process_resident_memory_bytes > 1e9  # 1GB
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Process memory usage above 1GB"
```
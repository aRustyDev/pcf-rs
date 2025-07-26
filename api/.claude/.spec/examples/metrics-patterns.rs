/// Metrics Patterns - Phase 5 Implementation Examples
///
/// This file demonstrates metrics implementation patterns including
/// cardinality control, performance optimization, and security considerations.

use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use metrics::{counter, histogram, gauge, Unit, Label};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::atomic::{AtomicU64, Ordering};

/// Cardinality Limiter - Prevents metric explosion
pub struct CardinalityLimiter {
    /// Maximum number of unique values per label
    max_values: usize,
    /// Tracked label values
    known_values: RwLock<HashMap<String, HashSet<String>>>,
    /// Counter for rejected values
    rejected_count: AtomicU64,
}

impl CardinalityLimiter {
    pub fn new(max_values: usize) -> Self {
        Self {
            max_values,
            known_values: RwLock::new(HashMap::new()),
            rejected_count: AtomicU64::new(0),
        }
    }
    
    /// Get label value, returning "other" if limit exceeded
    pub fn get_label_value(&self, label_name: &str, value: &str) -> &str {
        let known = self.known_values.read().unwrap();
        
        // Check if already known
        if let Some(values) = known.get(label_name) {
            if values.contains(value) {
                return value;
            }
        }
        drop(known);
        
        // Try to add new value
        let mut known = self.known_values.write().unwrap();
        let values = known.entry(label_name.to_string())
            .or_insert_with(HashSet::new);
        
        if values.len() < self.max_values {
            values.insert(value.to_string());
            value
        } else {
            // Log warning on first rejection
            let rejected = self.rejected_count.fetch_add(1, Ordering::Relaxed);
            if rejected == 0 {
                tracing::warn!(
                    "Cardinality limit {} exceeded for label '{}', using 'other'",
                    self.max_values, label_name
                );
            }
            "other"
        }
    }
    
    /// Get cardinality statistics
    pub fn stats(&self) -> CardinalityStats {
        let known = self.known_values.read().unwrap();
        let mut stats = CardinalityStats {
            labels: HashMap::new(),
            total_rejected: self.rejected_count.load(Ordering::Relaxed),
        };
        
        for (label, values) in known.iter() {
            stats.labels.insert(label.clone(), values.len());
        }
        
        stats
    }
}

#[derive(Debug)]
pub struct CardinalityStats {
    pub labels: HashMap<String, usize>,
    pub total_rejected: u64,
}

/// Metrics Manager with Cardinality Control
pub struct MetricsManager {
    handle: PrometheusHandle,
    cardinality_limiter: Arc<CardinalityLimiter>,
    global_labels: Vec<Label>,
}

impl MetricsManager {
    pub fn new(config: &MetricsConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize cardinality limiter
        let cardinality_limiter = Arc::new(CardinalityLimiter::new(
            config.max_label_values
        ));
        
        // Build Prometheus exporter
        let builder = PrometheusBuilder::new();
        let handle = builder.install_recorder()?;
        
        // Set global labels (limited set)
        let global_labels = vec![
            Label::new("service", "pcf-api"),
            Label::new("environment", &config.environment),
            Label::new("version", env!("CARGO_PKG_VERSION")),
            Label::new("instance", &get_instance_id()),
        ];
        
        // Validate global label count
        if global_labels.len() > 6 {
            return Err("Too many global labels (max 6)".into());
        }
        
        Ok(Self {
            handle,
            cardinality_limiter,
            global_labels,
        })
    }
    
    /// Render metrics in Prometheus format
    pub fn render(&self) -> String {
        self.handle.render()
    }
    
    /// Get cardinality limiter for use in metric recording
    pub fn limiter(&self) -> Arc<CardinalityLimiter> {
        self.cardinality_limiter.clone()
    }
}

/// Configuration for metrics
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub environment: String,
    pub max_label_values: usize,
    pub sample_rate: f64,
}

/// GraphQL Metrics Recording
pub mod graphql_metrics {
    use super::*;
    
    /// Record GraphQL request with cardinality control
    pub fn record_request(
        limiter: &CardinalityLimiter,
        operation_type: &str,
        operation_name: &str,
        duration: Duration,
        status: RequestStatus,
    ) {
        // Apply cardinality limiting to operation name
        let operation_name = limiter.get_label_value("operation_name", operation_name);
        
        // Record counter
        counter!(
            "graphql_request_total",
            &[
                ("operation_type", operation_type),
                ("operation_name", operation_name),
                ("status", status.as_str()),
            ]
        )
        .increment(1);
        
        // Record histogram
        histogram!(
            "graphql_request_duration_seconds",
            &[
                ("operation_type", operation_type),
                ("operation_name", operation_name),
            ]
        )
        .record(duration.as_secs_f64());
        
        // Update active operations gauge
        if operation_type == "subscription" {
            match status {
                RequestStatus::Started => {
                    gauge!(
                        "graphql_active_subscriptions",
                        &[("subscription_name", operation_name)]
                    )
                    .increment(1.0);
                }
                RequestStatus::Completed => {
                    gauge!(
                        "graphql_active_subscriptions",
                        &[("subscription_name", operation_name)]
                    )
                    .decrement(1.0);
                }
                _ => {}
            }
        }
    }
    
    /// Record field resolution time (only for slow fields)
    pub fn record_field_resolution(
        limiter: &CardinalityLimiter,
        field_name: &str,
        parent_type: &str,
        duration: Duration,
    ) {
        // Only record if slow (> 1ms)
        if duration.as_millis() > 1 {
            let field_name = limiter.get_label_value("field_name", field_name);
            let parent_type = limiter.get_label_value("parent_type", parent_type);
            
            histogram!(
                "graphql_field_resolution_duration_seconds",
                &[
                    ("field_name", field_name),
                    ("parent_type", parent_type),
                ]
            )
            .record(duration.as_secs_f64());
        }
    }
    
    /// Record GraphQL errors with limited error codes
    pub fn record_error(
        limiter: &CardinalityLimiter,
        error_code: &str,
        operation_type: &str,
        operation_name: &str,
    ) {
        let error_code = match error_code {
            "UNAUTHORIZED" | "FORBIDDEN" | "NOT_FOUND" | "BAD_REQUEST" |
            "INTERNAL_ERROR" | "SERVICE_UNAVAILABLE" => error_code,
            _ => "UNKNOWN", // Limit to known codes
        };
        
        let operation_name = limiter.get_label_value("operation_name", operation_name);
        
        counter!(
            "graphql_errors_total",
            &[
                ("error_code", error_code),
                ("operation_type", operation_type),
                ("operation_name", operation_name),
            ]
        )
        .increment(1);
    }
    
    #[derive(Debug, Clone, Copy)]
    pub enum RequestStatus {
        Started,
        Completed,
        Failed,
    }
    
    impl RequestStatus {
        pub fn as_str(&self) -> &'static str {
            match self {
                RequestStatus::Started => "started",
                RequestStatus::Completed => "success",
                RequestStatus::Failed => "error",
            }
        }
    }
}

/// HTTP Metrics with Status Code Bucketing
pub mod http_metrics {
    use super::*;
    
    /// Record HTTP request with bucketed status codes
    pub fn record_request(
        method: &str,
        path: &str,
        status_code: u16,
        duration: Duration,
    ) {
        // Bucket status code
        let status_bucket = bucket_status_code(status_code);
        
        // Normalize path (remove IDs, etc.)
        let normalized_path = normalize_path(path);
        
        counter!(
            "http_request_total",
            &[
                ("method", method),
                ("path", normalized_path),
                ("status", status_bucket),
            ]
        )
        .increment(1);
        
        histogram!(
            "http_request_duration_seconds",
            &[
                ("method", method),
                ("path", normalized_path),
            ]
        )
        .record(duration.as_secs_f64());
    }
    
    /// Bucket status codes to reduce cardinality
    fn bucket_status_code(status: u16) -> &'static str {
        match status {
            200..=299 => "2xx",
            300..=399 => "3xx",
            400..=499 => "4xx",
            500..=599 => "5xx",
            _ => "other",
        }
    }
    
    /// Normalize paths to prevent cardinality explosion
    fn normalize_path(path: &str) -> &'static str {
        match path {
            "/health" => "/health",
            "/health/ready" => "/health/ready",
            "/metrics" => "/metrics",
            "/graphql" => "/graphql",
            p if p.starts_with("/api/") => "/api/*",
            _ => "/other",
        }
    }
}

/// Database Metrics
pub mod database_metrics {
    use super::*;
    
    /// Record database query
    pub fn record_query(
        database: &str,
        operation: &str,
        duration: Duration,
        success: bool,
    ) {
        let status = if success { "success" } else { "error" };
        
        counter!(
            "database_query_total",
            &[
                ("database", database),
                ("operation", operation),
                ("status", status),
            ]
        )
        .increment(1);
        
        histogram!(
            "database_query_duration_seconds",
            &[
                ("database", database),
                ("operation", operation),
            ]
        )
        .record(duration.as_secs_f64());
    }
    
    /// Update connection pool metrics
    pub fn update_pool_metrics(database: &str, active: usize, idle: usize, max: usize) {
        gauge!(
            "database_connection_pool_size",
            &[("database", database)]
        )
        .set(max as f64);
        
        gauge!(
            "database_connection_pool_active",
            &[("database", database)]
        )
        .set(active as f64);
        
        gauge!(
            "database_connection_pool_idle",
            &[("database", database)]
        )
        .set(idle as f64);
    }
}

/// Business Metrics with Privacy
pub mod business_metrics {
    use super::*;
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    /// Record note creation with hashed author bucketing
    pub fn record_note_created(author: &str) {
        let author_bucket = bucket_by_hash(author, 10);
        
        counter!(
            "notes_created_total",
            &[("author_bucket", &author_bucket)]
        )
        .increment(1);
        
        gauge!("notes_total").increment(1.0);
    }
    
    /// Record note deletion
    pub fn record_note_deleted() {
        counter!("notes_deleted_total").increment(1);
        gauge!("notes_total").decrement(1.0);
    }
    
    /// Hash-based bucketing for privacy
    fn bucket_by_hash(value: &str, num_buckets: u32) -> String {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let bucket = (hasher.finish() % num_buckets as u64) as u32;
        format!("bucket_{}", bucket)
    }
}

/// Performance Optimizations
pub mod performance {
    use super::*;
    use rand::Rng;
    
    /// Sampled histogram for expensive metrics
    pub struct SampledHistogram {
        name: &'static str,
        labels: Vec<(&'static str, String)>,
        sample_rate: f64,
    }
    
    impl SampledHistogram {
        pub fn new(
            name: &'static str,
            labels: Vec<(&'static str, String)>,
            sample_rate: f64,
        ) -> Self {
            Self {
                name,
                labels,
                sample_rate,
            }
        }
        
        /// Record value with sampling
        pub fn record(&self, value: f64) {
            if rand::thread_rng().gen::<f64>() < self.sample_rate {
                // Adjust value for sampling rate
                let adjusted_value = value * (1.0 / self.sample_rate);
                
                histogram!(self.name, &self.labels).record(adjusted_value);
            }
        }
    }
    
    /// Fast metrics for hot paths using atomics
    pub struct FastCounter {
        value: AtomicU64,
        name: &'static str,
        labels: Vec<(&'static str, String)>,
    }
    
    impl FastCounter {
        pub fn new(name: &'static str, labels: Vec<(&'static str, String)>) -> Self {
            Self {
                value: AtomicU64::new(0),
                name,
                labels,
            }
        }
        
        /// Increment counter (very fast)
        pub fn increment(&self) {
            self.value.fetch_add(1, Ordering::Relaxed);
        }
        
        /// Flush to Prometheus (call periodically)
        pub fn flush(&self) {
            let value = self.value.swap(0, Ordering::Relaxed);
            if value > 0 {
                counter!(self.name, &self.labels).increment(value);
            }
        }
    }
}

/// Cardinality Monitoring
pub mod monitoring {
    use super::*;
    
    /// Monitor cardinality growth
    pub struct CardinalityMonitor {
        limits: HashMap<&'static str, usize>,
        current: Arc<RwLock<HashMap<&'static str, HashSet<String>>>>,
    }
    
    impl CardinalityMonitor {
        pub fn new(limits: HashMap<&'static str, usize>) -> Self {
            Self {
                limits,
                current: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        
        /// Check if new label combination is within limits
        pub fn check_and_track(
            &self,
            metric_name: &'static str,
            label_combination: String,
        ) -> bool {
            let mut current = self.current.write().unwrap();
            let combinations = current.entry(metric_name)
                .or_insert_with(HashSet::new);
            
            if combinations.len() >= self.limits[metric_name] {
                if !combinations.contains(&label_combination) {
                    tracing::error!(
                        "Metric {} exceeds cardinality limit of {}",
                        metric_name, self.limits[metric_name]
                    );
                    
                    // Record cardinality violation
                    counter!(
                        "metrics_cardinality_limit_exceeded_total",
                        &[("metric", metric_name)]
                    )
                    .increment(1);
                    
                    return false;
                }
            }
            
            combinations.insert(label_combination);
            true
        }
        
        /// Get current cardinality stats
        pub fn stats(&self) -> HashMap<&'static str, usize> {
            let current = self.current.read().unwrap();
            let mut stats = HashMap::new();
            
            for (metric, combinations) in current.iter() {
                stats.insert(*metric, combinations.len());
            }
            
            stats
        }
        
        /// Start background monitoring task
        pub fn start_monitoring(self: Arc<Self>) {
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(300));
                
                loop {
                    interval.tick().await;
                    
                    let stats = self.stats();
                    for (metric, cardinality) in stats {
                        gauge!(
                            "metrics_cardinality_current",
                            &[("metric", metric)]
                        )
                        .set(cardinality as f64);
                        
                        // Warn if approaching limit
                        if let Some(limit) = self.limits.get(metric) {
                            if cardinality > limit * 8 / 10 {
                                tracing::warn!(
                                    "Metric {} at {}% of cardinality limit",
                                    metric,
                                    (cardinality * 100) / limit
                                );
                            }
                        }
                    }
                }
            });
        }
    }
}

/// Utility Functions
fn get_instance_id() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("INSTANCE_ID"))
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cardinality_limiter() {
        let limiter = CardinalityLimiter::new(3);
        
        // First 3 values accepted
        assert_eq!(limiter.get_label_value("op", "query1"), "query1");
        assert_eq!(limiter.get_label_value("op", "query2"), "query2");
        assert_eq!(limiter.get_label_value("op", "query3"), "query3");
        
        // 4th value returns "other"
        assert_eq!(limiter.get_label_value("op", "query4"), "other");
        
        // Original values still work
        assert_eq!(limiter.get_label_value("op", "query1"), "query1");
        
        // Check stats
        let stats = limiter.stats();
        assert_eq!(stats.labels["op"], 3);
        assert_eq!(stats.total_rejected, 1);
    }
    
    #[test]
    fn test_status_code_bucketing() {
        use http_metrics::bucket_status_code;
        
        assert_eq!(bucket_status_code(200), "2xx");
        assert_eq!(bucket_status_code(201), "2xx");
        assert_eq!(bucket_status_code(404), "4xx");
        assert_eq!(bucket_status_code(500), "5xx");
        assert_eq!(bucket_status_code(999), "other");
    }
    
    #[test]
    fn test_hash_bucketing() {
        use business_metrics::bucket_by_hash;
        
        // Same input always produces same bucket
        let bucket1 = bucket_by_hash("user123", 10);
        let bucket2 = bucket_by_hash("user123", 10);
        assert_eq!(bucket1, bucket2);
        
        // Bucket is within range
        assert!(bucket1.starts_with("bucket_"));
        let num: u32 = bucket1.strip_prefix("bucket_").unwrap().parse().unwrap();
        assert!(num < 10);
    }
    
    #[test]
    fn test_sampled_histogram() {
        use performance::SampledHistogram;
        
        let histogram = SampledHistogram::new(
            "test_histogram",
            vec![("label", "value".to_string())],
            0.1, // 10% sampling
        );
        
        // Record many values
        for _ in 0..1000 {
            histogram.record(1.0);
        }
        
        // Would see approximately 100 samples recorded
        // (actual test would need mock metrics backend)
    }
}
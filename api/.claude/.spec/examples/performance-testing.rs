/// Performance Testing Patterns - Phase 6 Implementation Examples
///
/// This file demonstrates performance testing patterns including load testing,
/// benchmarking, profiling helpers, and performance monitoring.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Load Test Configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Target requests per second
    pub target_rps: u32,
    /// Test duration
    pub duration: Duration,
    /// Ramp-up time to reach target RPS
    pub ramp_up: Duration,
    /// Number of concurrent connections
    pub connections: usize,
    /// Query distribution
    pub query_distribution: QueryDistribution,
    /// Think time between requests per connection
    pub think_time: Option<Duration>,
}

/// Query distribution for realistic load patterns
#[derive(Debug, Clone)]
pub struct QueryDistribution {
    pub queries: Vec<QueryPattern>,
}

#[derive(Debug, Clone)]
pub struct QueryPattern {
    pub name: String,
    pub query: String,
    pub variables: serde_json::Value,
    pub weight: f32, // Probability of selection
}

impl QueryDistribution {
    /// Create a realistic query distribution
    pub fn realistic() -> Self {
        Self {
            queries: vec![
                QueryPattern {
                    name: "user_list".to_string(),
                    query: "{ users(first: 20) { id name } }".to_string(),
                    variables: json!({}),
                    weight: 0.3,
                },
                QueryPattern {
                    name: "note_detail".to_string(),
                    query: "{ note(id: $id) { id title content author { name } tags { name } } }".to_string(),
                    variables: json!({"id": "random"}),
                    weight: 0.4,
                },
                QueryPattern {
                    name: "user_notes".to_string(),
                    query: "{ user(id: $id) { notes { id title author { name } } } }".to_string(),
                    variables: json!({"id": "random"}),
                    weight: 0.2,
                },
                QueryPattern {
                    name: "create_note".to_string(),
                    query: "mutation { createNote(input: $input) { id } }".to_string(),
                    variables: json!({"input": {"title": "Test", "content": "Content"}}),
                    weight: 0.1,
                },
            ],
        }
    }
    
    /// Select a query based on weights
    pub fn select(&self) -> &QueryPattern {
        let mut rng = rand::thread_rng();
        let roll: f32 = rand::Rng::gen(&mut rng);
        
        let mut cumulative = 0.0;
        for pattern in &self.queries {
            cumulative += pattern.weight;
            if roll <= cumulative {
                return pattern;
            }
        }
        
        &self.queries[0] // Fallback
    }
}

/// Load Test Results
#[derive(Debug, Clone, Serialize)]
pub struct LoadTestResults {
    pub config: LoadTestSummary,
    pub summary: PerformanceSummary,
    pub latency_percentiles: LatencyPercentiles,
    pub throughput: ThroughputStats,
    pub errors: ErrorStats,
    pub timeline: Vec<TimelinePoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadTestSummary {
    pub duration_secs: u64,
    pub target_rps: u32,
    pub connections: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceSummary {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub actual_rps: f64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatencyPercentiles {
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThroughputStats {
    pub avg_rps: f64,
    pub peak_rps: f64,
    pub min_rps: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub errors_by_type: HashMap<String, u64>,
    pub timeout_errors: u64,
    pub connection_errors: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelinePoint {
    pub timestamp_secs: u64,
    pub requests: u64,
    pub errors: u64,
    pub avg_latency_ms: f64,
    pub active_connections: usize,
}

/// Load Test Runner
pub struct LoadTestRunner {
    client: reqwest::Client,
    config: LoadTestConfig,
    metrics: Arc<LoadTestMetrics>,
}

struct LoadTestMetrics {
    requests: AtomicU64,
    successes: AtomicU64,
    failures: AtomicU64,
    latencies: RwLock<Vec<Duration>>,
    errors: RwLock<HashMap<String, AtomicU64>>,
    timeline: RwLock<Vec<TimelinePoint>>,
}

impl LoadTestRunner {
    pub fn new(endpoint: &str, config: LoadTestConfig) -> Self {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(config.connections)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
            
        Self {
            client,
            config,
            metrics: Arc::new(LoadTestMetrics {
                requests: AtomicU64::new(0),
                successes: AtomicU64::new(0),
                failures: AtomicU64::new(0),
                latencies: RwLock::new(Vec::new()),
                errors: RwLock::new(HashMap::new()),
                timeline: RwLock::new(Vec::new()),
            }),
        }
    }
    
    /// Run the load test
    pub async fn run(&self) -> LoadTestResults {
        let start = Instant::now();
        
        // Start timeline recorder
        let timeline_handle = self.start_timeline_recorder();
        
        // Start worker tasks
        let mut handles = Vec::new();
        for i in 0..self.config.connections {
            let runner = self.clone();
            let handle = tokio::spawn(async move {
                runner.worker_loop(i).await;
            });
            handles.push(handle);
        }
        
        // Wait for duration
        tokio::time::sleep(self.config.duration).await;
        
        // Stop workers (would use cancellation token in real impl)
        
        // Wait for workers to finish
        for handle in handles {
            let _ = handle.await;
        }
        
        // Stop timeline recorder
        timeline_handle.abort();
        
        // Calculate results
        self.calculate_results(start.elapsed()).await
    }
    
    /// Worker loop for a single connection
    async fn worker_loop(&self, worker_id: usize) {
        let start = Instant::now();
        let mut request_count = 0;
        
        loop {
            // Check if we should stop
            if start.elapsed() >= self.config.duration {
                break;
            }
            
            // Calculate current RPS based on ramp-up
            let elapsed = start.elapsed();
            let target_rps = if elapsed < self.config.ramp_up {
                // Linear ramp-up
                let progress = elapsed.as_secs_f64() / self.config.ramp_up.as_secs_f64();
                (self.config.target_rps as f64 * progress) as u32
            } else {
                self.config.target_rps
            };
            
            // Calculate delay between requests for this worker
            let rps_per_worker = target_rps as f64 / self.config.connections as f64;
            let delay = Duration::from_secs_f64(1.0 / rps_per_worker);
            
            // Select query
            let pattern = self.config.query_distribution.select();
            
            // Send request
            let request_start = Instant::now();
            let result = self.send_graphql_request(pattern).await;
            let latency = request_start.elapsed();
            
            // Record metrics
            self.record_request_result(result, latency).await;
            
            // Think time
            if let Some(think_time) = self.config.think_time {
                tokio::time::sleep(think_time).await;
            } else {
                tokio::time::sleep(delay).await;
            }
            
            request_count += 1;
        }
        
        tracing::debug!("Worker {} completed {} requests", worker_id, request_count);
    }
    
    /// Send a GraphQL request
    async fn send_graphql_request(&self, pattern: &QueryPattern) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .post("http://localhost:8080/graphql")
            .json(&json!({
                "query": pattern.query,
                "variables": pattern.variables,
            }))
            .send()
            .await
    }
    
    /// Record request result
    async fn record_request_result(&self, result: Result<reqwest::Response, reqwest::Error>, latency: Duration) {
        self.metrics.requests.fetch_add(1, Ordering::Relaxed);
        
        match result {
            Ok(response) => {
                if response.status().is_success() {
                    self.metrics.successes.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.metrics.failures.fetch_add(1, Ordering::Relaxed);
                    self.record_error(&format!("HTTP {}", response.status())).await;
                }
            }
            Err(e) => {
                self.metrics.failures.fetch_add(1, Ordering::Relaxed);
                self.record_error(&e.to_string()).await;
            }
        }
        
        // Record latency
        let mut latencies = self.metrics.latencies.write().await;
        latencies.push(latency);
    }
    
    /// Record error type
    async fn record_error(&self, error_type: &str) {
        let mut errors = self.metrics.errors.write().await;
        errors.entry(error_type.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }
    
    /// Start timeline recorder
    fn start_timeline_recorder(&self) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let start = Instant::now();
            
            loop {
                interval.tick().await;
                
                let timestamp_secs = start.elapsed().as_secs();
                let snapshot = TimelinePoint {
                    timestamp_secs,
                    requests: metrics.requests.load(Ordering::Relaxed),
                    errors: metrics.failures.load(Ordering::Relaxed),
                    avg_latency_ms: 0.0, // Would calculate from recent latencies
                    active_connections: 0, // Would track active connections
                };
                
                let mut timeline = metrics.timeline.write().await;
                timeline.push(snapshot);
            }
        })
    }
    
    /// Calculate final results
    async fn calculate_results(&self, duration: Duration) -> LoadTestResults {
        let total_requests = self.metrics.requests.load(Ordering::Relaxed);
        let successful_requests = self.metrics.successes.load(Ordering::Relaxed);
        let failed_requests = self.metrics.failures.load(Ordering::Relaxed);
        
        let latencies = self.metrics.latencies.read().await;
        let latency_percentiles = calculate_percentiles(&latencies);
        
        let avg_latency_ms = if !latencies.is_empty() {
            latencies.iter().map(|d| d.as_secs_f64() * 1000.0).sum::<f64>() / latencies.len() as f64
        } else {
            0.0
        };
        
        let actual_rps = total_requests as f64 / duration.as_secs_f64();
        
        LoadTestResults {
            config: LoadTestSummary {
                duration_secs: duration.as_secs(),
                target_rps: self.config.target_rps,
                connections: self.config.connections,
            },
            summary: PerformanceSummary {
                total_requests,
                successful_requests,
                failed_requests,
                success_rate: successful_requests as f64 / total_requests as f64,
                actual_rps,
                avg_latency_ms,
            },
            latency_percentiles,
            throughput: ThroughputStats {
                avg_rps: actual_rps,
                peak_rps: 0.0, // Would calculate from timeline
                min_rps: 0.0, // Would calculate from timeline
            },
            errors: self.calculate_error_stats().await,
            timeline: self.metrics.timeline.read().await.clone(),
        }
    }
    
    async fn calculate_error_stats(&self) -> ErrorStats {
        let errors = self.metrics.errors.read().await;
        let mut errors_by_type = HashMap::new();
        let mut timeout_errors = 0;
        let mut connection_errors = 0;
        
        for (error_type, count) in errors.iter() {
            let count_val = count.load(Ordering::Relaxed);
            errors_by_type.insert(error_type.clone(), count_val);
            
            if error_type.contains("timeout") {
                timeout_errors += count_val;
            }
            if error_type.contains("connection") {
                connection_errors += count_val;
            }
        }
        
        ErrorStats {
            total_errors: self.metrics.failures.load(Ordering::Relaxed),
            errors_by_type,
            timeout_errors,
            connection_errors,
        }
    }
}

impl Clone for LoadTestRunner {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// Calculate percentiles from latency data
fn calculate_percentiles(latencies: &[Duration]) -> LatencyPercentiles {
    if latencies.is_empty() {
        return LatencyPercentiles {
            p50: 0.0,
            p75: 0.0,
            p90: 0.0,
            p95: 0.0,
            p99: 0.0,
            p999: 0.0,
            max: 0.0,
        };
    }
    
    let mut sorted: Vec<f64> = latencies.iter()
        .map(|d| d.as_secs_f64() * 1000.0)
        .collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let len = sorted.len();
    
    LatencyPercentiles {
        p50: sorted[len * 50 / 100],
        p75: sorted[len * 75 / 100],
        p90: sorted[len * 90 / 100],
        p95: sorted[len * 95 / 100],
        p99: sorted[len * 99 / 100],
        p999: sorted[len.saturating_sub(1).min(len * 999 / 1000)],
        max: sorted[len - 1],
    }
}

/// Performance Benchmarking
pub mod benchmarks {
    use super::*;
    use criterion::{Criterion, BenchmarkId};
    
    /// Benchmark GraphQL query performance
    pub fn benchmark_graphql_queries(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = create_test_client();
        
        let mut group = c.benchmark_group("graphql_queries");
        
        // Simple query
        group.bench_function("simple_query", |b| {
            b.to_async(&rt).iter(|| async {
                client.graphql_query("{ users(first: 10) { id name } }").await
            })
        });
        
        // Complex nested query
        group.bench_function("nested_query", |b| {
            b.to_async(&rt).iter(|| async {
                client.graphql_query("{ users { notes { author { notes { tags } } } } }").await
            })
        });
        
        // Query with variables
        group.bench_function("parameterized_query", |b| {
            b.to_async(&rt).iter(|| async {
                client.graphql_query_with_vars(
                    "query ($id: ID!) { user(id: $id) { notes { title } } }",
                    json!({"id": "123"})
                ).await
            })
        });
        
        group.finish();
    }
    
    /// Benchmark DataLoader performance
    pub fn benchmark_dataloader(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let mut group = c.benchmark_group("dataloader");
        
        for batch_size in [1, 10, 50, 100, 500].iter() {
            group.bench_with_input(
                BenchmarkId::new("batch_size", batch_size),
                batch_size,
                |b, &size| {
                    b.to_async(&rt).iter(|| async move {
                        let loader = create_test_loader();
                        let keys: Vec<_> = (0..size).map(|i| i.to_string()).collect();
                        loader.load_many(&keys).await
                    })
                },
            );
        }
        
        group.finish();
    }
    
    fn create_test_client() -> TestClient {
        TestClient::new()
    }
    
    fn create_test_loader() -> DataLoader<TestLoader> {
        DataLoader::new(TestLoader)
    }
    
    struct TestClient;
    
    impl TestClient {
        fn new() -> Self { Self }
        
        async fn graphql_query(&self, _query: &str) -> Result<serde_json::Value, Error> {
            Ok(json!({}))
        }
        
        async fn graphql_query_with_vars(&self, _query: &str, _vars: serde_json::Value) -> Result<serde_json::Value, Error> {
            Ok(json!({}))
        }
    }
    
    struct TestLoader;
    
    type Error = Box<dyn std::error::Error>;
    type DataLoader<T> = super::DataLoader<T>;
}

/// Performance Profiling Helpers
pub mod profiling {
    use super::*;
    
    /// CPU profiler wrapper
    pub struct CpuProfiler {
        #[cfg(feature = "profiling")]
        guard: Option<pprof::ProfilerGuard<'static>>,
    }
    
    impl CpuProfiler {
        /// Start CPU profiling
        pub fn start() -> Self {
            #[cfg(feature = "profiling")]
            {
                let guard = pprof::ProfilerGuardBuilder::default()
                    .frequency(1000)
                    .blocklist(&["libc", "libgcc", "pthread"])
                    .build()
                    .unwrap();
                    
                Self { guard: Some(guard) }
            }
            
            #[cfg(not(feature = "profiling"))]
            Self {}
        }
        
        /// Stop profiling and save flamegraph
        pub fn save_flamegraph(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
            #[cfg(feature = "profiling")]
            if let Some(guard) = self.guard.take() {
                let report = guard.report().build()?;
                let file = std::fs::File::create(path)?;
                report.flamegraph(file)?;
            }
            
            Ok(())
        }
    }
    
    /// Memory profiler helper
    pub struct MemoryProfiler {
        baseline: Option<MemoryStats>,
    }
    
    #[derive(Debug, Clone)]
    pub struct MemoryStats {
        pub heap_size: usize,
        pub heap_used: usize,
        pub resident_set_size: usize,
    }
    
    impl MemoryProfiler {
        pub fn new() -> Self {
            Self { baseline: None }
        }
        
        /// Take baseline measurement
        pub fn baseline(&mut self) {
            self.baseline = Some(Self::current_stats());
        }
        
        /// Get memory growth since baseline
        pub fn growth(&self) -> Option<MemoryStats> {
            self.baseline.as_ref().map(|baseline| {
                let current = Self::current_stats();
                MemoryStats {
                    heap_size: current.heap_size.saturating_sub(baseline.heap_size),
                    heap_used: current.heap_used.saturating_sub(baseline.heap_used),
                    resident_set_size: current.resident_set_size.saturating_sub(baseline.resident_set_size),
                }
            })
        }
        
        fn current_stats() -> MemoryStats {
            // Would use jemalloc or system stats
            MemoryStats {
                heap_size: 0,
                heap_used: 0,
                resident_set_size: 0,
            }
        }
    }
}

/// Performance Monitoring
pub mod monitoring {
    use super::*;
    
    /// Real-time performance monitor
    pub struct PerformanceMonitor {
        latency_tracker: Arc<LatencyTracker>,
        throughput_counter: Arc<ThroughputCounter>,
        slo_checker: Arc<SloChecker>,
    }
    
    pub struct LatencyTracker {
        buckets: RwLock<LatencyBuckets>,
    }
    
    struct LatencyBuckets {
        current: Vec<Duration>,
        previous: Vec<Duration>,
        rotation_time: Instant,
    }
    
    impl LatencyTracker {
        pub async fn record(&self, latency: Duration) {
            let mut buckets = self.buckets.write().await;
            
            // Rotate buckets every minute
            if buckets.rotation_time.elapsed() > Duration::from_secs(60) {
                buckets.previous = std::mem::take(&mut buckets.current);
                buckets.rotation_time = Instant::now();
            }
            
            buckets.current.push(latency);
        }
        
        pub async fn p99(&self) -> Duration {
            let buckets = self.buckets.read().await;
            let mut all_latencies = buckets.current.clone();
            all_latencies.extend(&buckets.previous);
            
            if all_latencies.is_empty() {
                return Duration::from_secs(0);
            }
            
            all_latencies.sort();
            all_latencies[all_latencies.len() * 99 / 100]
        }
    }
    
    pub struct ThroughputCounter {
        counts: RwLock<HashMap<Instant, AtomicU64>>,
    }
    
    impl ThroughputCounter {
        pub async fn increment(&self) {
            let now = Instant::now();
            let second = Duration::from_secs(now.elapsed().as_secs());
            
            let mut counts = self.counts.write().await;
            counts.entry(now - second)
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
            
            // Clean old entries
            counts.retain(|k, _| now.duration_since(*k) < Duration::from_secs(300));
        }
        
        pub async fn current_rps(&self) -> f64 {
            let counts = self.counts.read().await;
            let now = Instant::now();
            
            let recent_count: u64 = counts.iter()
                .filter(|(k, _)| now.duration_since(**k) < Duration::from_secs(10))
                .map(|(_, v)| v.load(Ordering::Relaxed))
                .sum();
                
            recent_count as f64 / 10.0
        }
    }
    
    pub struct SloChecker {
        violations: AtomicU64,
    }
    
    impl SloChecker {
        pub async fn check_slos(&self, monitor: &PerformanceMonitor) {
            // Check P99 < 200ms
            if monitor.latency_tracker.p99().await > Duration::from_millis(200) {
                self.violations.fetch_add(1, Ordering::Relaxed);
                tracing::warn!("SLO violation: P99 latency exceeds 200ms");
            }
            
            // Check RPS
            let current_rps = monitor.throughput_counter.current_rps().await;
            if current_rps < 900.0 { // 90% of target
                tracing::warn!("Performance warning: RPS {} below target", current_rps);
            }
        }
    }
}

use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_query_distribution() {
        let dist = QueryDistribution::realistic();
        
        // Test selection works
        let mut counts = HashMap::new();
        for _ in 0..1000 {
            let pattern = dist.select();
            *counts.entry(pattern.name.clone()).or_insert(0) += 1;
        }
        
        // Verify roughly correct distribution
        assert!(counts["user_list"] > 200 && counts["user_list"] < 400);
        assert!(counts["note_detail"] > 300 && counts["note_detail"] < 500);
    }
    
    #[test]
    fn test_percentile_calculation() {
        let latencies = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
            Duration::from_millis(60),
            Duration::from_millis(70),
            Duration::from_millis(80),
            Duration::from_millis(90),
            Duration::from_millis(100),
        ];
        
        let percentiles = calculate_percentiles(&latencies);
        
        assert_eq!(percentiles.p50, 50.0);
        assert_eq!(percentiles.p90, 90.0);
        assert_eq!(percentiles.max, 100.0);
    }
}
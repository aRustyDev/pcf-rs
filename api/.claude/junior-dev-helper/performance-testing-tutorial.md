# Performance Testing Tutorial

## Why Performance Testing?

Performance testing ensures your API can handle real-world usage. Without it, you might discover problems only when users complain about slow responses or crashes.

### Key Metrics to Measure

1. **Latency**: How long requests take
   - P50 (median): 50% of requests are faster
   - P95: 95% of requests are faster
   - P99: 99% of requests are faster

2. **Throughput**: Requests per second (RPS)
   - Sustained load: Can maintain over time
   - Peak load: Maximum before degradation

3. **Error Rate**: Percentage of failed requests
   - Should be < 1% under normal load

4. **Resource Usage**: CPU, memory, connections
   - Linear scaling is good
   - Exponential scaling is bad

## Types of Performance Tests

### 1. Load Testing
Tests expected normal usage:
```rust
// Test: 1000 RPS for 5 minutes
LoadTestConfig {
    target_rps: 1000,
    duration: Duration::from_secs(300),
    ramp_up: Duration::from_secs(30),
    connections: 100,
}
```

### 2. Stress Testing
Finds the breaking point:
```rust
// Gradually increase load until failure
for rps in [100, 500, 1000, 2000, 5000] {
    if !run_load_test(rps).await.is_success() {
        println!("System breaks at {} RPS", rps);
        break;
    }
}
```

### 3. Spike Testing
Tests sudden traffic increases:
```rust
// Normal -> 10x spike -> Normal
LoadPattern::Spike {
    baseline_rps: 100,
    spike_rps: 1000,
    spike_duration: Duration::from_secs(60),
}
```

### 4. Soak Testing
Tests for memory leaks and degradation:
```rust
// Low load for extended time
LoadTestConfig {
    target_rps: 100,
    duration: Duration::from_hours(24),
    connections: 10,
}
```

## Building a Load Test

### Step 1: Define Realistic Query Mix

Real users don't all make the same query. Create a realistic distribution:

```rust
pub struct QueryDistribution {
    queries: Vec<WeightedQuery>,
}

pub struct WeightedQuery {
    name: String,
    query: String,
    variables: Value,
    weight: f32,  // Probability
}

impl QueryDistribution {
    pub fn realistic() -> Self {
        Self {
            queries: vec![
                WeightedQuery {
                    name: "user_dashboard".into(),
                    query: r#"
                        query Dashboard {
                            currentUser {
                                name
                                notifications { unreadCount }
                                recentActivity { ... }
                            }
                        }
                    "#.into(),
                    variables: json!({}),
                    weight: 0.3,  // 30% of requests
                },
                WeightedQuery {
                    name: "post_list".into(),
                    query: r#"
                        query Posts($cursor: String) {
                            posts(first: 20, after: $cursor) {
                                edges {
                                    node { id title author { name } }
                                }
                                pageInfo { hasNextPage endCursor }
                            }
                        }
                    "#.into(),
                    variables: json!({"cursor": null}),
                    weight: 0.4,  // 40% of requests
                },
                WeightedQuery {
                    name: "create_comment".into(),
                    query: r#"
                        mutation CreateComment($postId: ID!, $text: String!) {
                            createComment(postId: $postId, text: $text) {
                                id
                                createdAt
                            }
                        }
                    "#.into(),
                    variables: json!({"postId": "1", "text": "Test comment"}),
                    weight: 0.1,  // 10% of requests
                },
                // ... more queries to match real usage
            ],
        }
    }
}
```

### Step 2: Create Virtual Users

Each virtual user simulates a real user's behavior:

```rust
pub struct VirtualUser {
    id: usize,
    client: reqwest::Client,
    endpoint: String,
    auth_token: Option<String>,
}

impl VirtualUser {
    pub async fn run_session(&self, distribution: &QueryDistribution) {
        // Login if needed
        if self.auth_token.is_none() {
            self.login().await;
        }
        
        // Simulate user session
        for _ in 0..10 {  // 10 requests per session
            // Pick a query based on weights
            let query = distribution.select_weighted();
            
            // Add some randomness to variables
            let variables = self.randomize_variables(&query.variables);
            
            // Make request
            let start = Instant::now();
            let result = self.execute_query(&query.query, &variables).await;
            let duration = start.elapsed();
            
            // Record metrics
            self.record_result(result, duration, &query.name).await;
            
            // Think time (simulate user reading)
            tokio::time::sleep(Duration::from_millis(
                rand::thread_rng().gen_range(500..2000)
            )).await;
        }
    }
    
    async fn execute_query(&self, query: &str, variables: &Value) -> Result<Response> {
        self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.auth_token.as_ref().unwrap_or(&"".into())))
            .json(&json!({
                "query": query,
                "variables": variables,
            }))
            .timeout(Duration::from_secs(30))
            .send()
            .await
    }
}
```

### Step 3: Coordinate Load Generation

```rust
pub struct LoadGenerator {
    config: LoadTestConfig,
    users: Vec<VirtualUser>,
    metrics: Arc<Metrics>,
}

impl LoadGenerator {
    pub async fn run(&self) -> LoadTestResults {
        let start = Instant::now();
        let distribution = QueryDistribution::realistic();
        
        // Calculate requests per user
        let rps_per_user = self.config.target_rps as f64 / self.config.connections as f64;
        let delay_between_requests = Duration::from_secs_f64(1.0 / rps_per_user);
        
        // Spawn virtual users
        let mut handles = Vec::new();
        for (i, user) in self.users.iter().enumerate() {
            let user = user.clone();
            let dist = distribution.clone();
            let metrics = self.metrics.clone();
            let delay = delay_between_requests;
            let duration = self.config.duration;
            
            let handle = tokio::spawn(async move {
                // Stagger start times
                tokio::time::sleep(Duration::from_millis(i as u64 * 10)).await;
                
                let user_start = Instant::now();
                while user_start.elapsed() < duration {
                    user.run_session(&dist).await;
                    tokio::time::sleep(delay).await;
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all users to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Calculate results
        self.calculate_results(start.elapsed()).await
    }
}
```

### Step 4: Collect and Analyze Metrics

```rust
#[derive(Clone)]
pub struct Metrics {
    latencies: Arc<RwLock<Vec<(Instant, Duration)>>>,
    errors: Arc<RwLock<HashMap<String, AtomicU64>>>,
    requests: Arc<AtomicU64>,
    successes: Arc<AtomicU64>,
}

impl Metrics {
    pub async fn record(&self, duration: Duration, error: Option<String>) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        
        if let Some(error_type) = error {
            self.errors.write().await
                .entry(error_type)
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.successes.fetch_add(1, Ordering::Relaxed);
            
            let mut latencies = self.latencies.write().await;
            latencies.push((Instant::now(), duration));
            
            // Keep only recent data (sliding window)
            let cutoff = Instant::now() - Duration::from_secs(60);
            latencies.retain(|(time, _)| *time > cutoff);
        }
    }
    
    pub async fn calculate_percentiles(&self) -> LatencyPercentiles {
        let latencies = self.latencies.read().await;
        let mut durations: Vec<_> = latencies.iter()
            .map(|(_, d)| d.as_millis() as f64)
            .collect();
        
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let len = durations.len();
        if len == 0 {
            return LatencyPercentiles::default();
        }
        
        LatencyPercentiles {
            p50: durations[len * 50 / 100],
            p75: durations[len * 75 / 100],
            p90: durations[len * 90 / 100],
            p95: durations[len * 95 / 100],
            p99: durations[len * 99 / 100],
            p999: durations[len.saturating_sub(1).min(len * 999 / 1000)],
            max: durations[len - 1],
        }
    }
}
```

## Writing Performance Tests

### Basic Load Test

```rust
#[tokio::test]
async fn test_can_handle_100_rps() {
    let app = start_test_server().await;
    
    let config = LoadTestConfig {
        target_rps: 100,
        duration: Duration::from_secs(60),
        ramp_up: Duration::from_secs(10),
        connections: 10,
    };
    
    let results = LoadGenerator::new(config, &app.endpoint)
        .run()
        .await;
    
    // Assertions
    assert!(results.success_rate > 0.99, "Success rate too low: {}", results.success_rate);
    assert!(results.p99_latency_ms < 200.0, "P99 latency too high: {}ms", results.p99_latency_ms);
    assert_eq!(results.errors.timeout_count, 0, "Timeouts occurred");
}
```

### N+1 Query Detection

```rust
#[tokio::test]
async fn test_no_n_plus_one_under_load() {
    let app = start_test_server().await;
    
    // Track database query count
    let query_counter = app.instrument_database_queries();
    
    // Run load test
    let config = LoadTestConfig {
        target_rps: 50,
        duration: Duration::from_secs(30),
        connections: 5,
        query_distribution: QueryDistribution::single(
            // Query that could trigger N+1
            r#"
                query {
                    posts(first: 20) {
                        id
                        title
                        author { name email }
                        comments { text author { name } }
                    }
                }
            "#
        ),
    };
    
    let results = LoadGenerator::new(config, &app.endpoint).run().await;
    
    // Calculate queries per request
    let total_requests = results.total_requests;
    let total_queries = query_counter.load(Ordering::Relaxed);
    let queries_per_request = total_queries as f64 / total_requests as f64;
    
    // Should be ~3-4 queries (posts, authors, comments, comment authors)
    // Not 40+ (which would indicate N+1)
    assert!(
        queries_per_request < 10.0,
        "Possible N+1 detected: {} queries per request",
        queries_per_request
    );
}
```

### Cache Effectiveness Test

```rust
#[tokio::test]
async fn test_cache_reduces_latency() {
    let app = start_test_server().await;
    
    let query = r#"{ expensiveQuery { result } }"#;
    
    // First run - cold cache
    let cold_results = LoadGenerator::new(
        LoadTestConfig {
            target_rps: 10,
            duration: Duration::from_secs(30),
            connections: 1,
            query_distribution: QueryDistribution::single(query),
        },
        &app.endpoint
    ).run().await;
    
    // Second run - warm cache
    let warm_results = LoadGenerator::new(
        LoadTestConfig {
            target_rps: 10,
            duration: Duration::from_secs(30),
            connections: 1,
            query_distribution: QueryDistribution::single(query),
        },
        &app.endpoint
    ).run().await;
    
    // Cache should significantly reduce latency
    assert!(
        warm_results.p50_latency_ms < cold_results.p50_latency_ms * 0.5,
        "Cache not effective: cold={:.1}ms, warm={:.1}ms",
        cold_results.p50_latency_ms,
        warm_results.p50_latency_ms
    );
    
    // Check cache hit rate
    let metrics = app.get_metrics().await;
    assert!(
        metrics.cache_hit_rate > 0.8,
        "Cache hit rate too low: {:.1}%",
        metrics.cache_hit_rate * 100.0
    );
}
```

## Profiling During Load Tests

### CPU Profiling

```rust
pub async fn profile_under_load() {
    // Start CPU profiler
    let profiler = CpuProfiler::start();
    
    // Run load test
    let config = LoadTestConfig {
        target_rps: 500,
        duration: Duration::from_secs(60),
        connections: 50,
    };
    
    LoadGenerator::new(config, "http://localhost:8080/graphql")
        .run()
        .await;
    
    // Save flamegraph
    profiler.save_flamegraph("cpu_profile_500rps.svg").unwrap();
    
    // Analyze hot spots
    println!("Top CPU consumers:");
    println!("- 40% in GraphQL parsing");
    println!("- 30% in database queries");
    println!("- 20% in JSON serialization");
    println!("- 10% other");
}
```

### Memory Profiling

```rust
pub async fn check_memory_during_load() {
    let memory_monitor = MemoryMonitor::new();
    memory_monitor.baseline();
    
    // Run sustained load
    let config = LoadTestConfig {
        target_rps: 200,
        duration: Duration::from_hours(1),
        connections: 20,
    };
    
    // Monitor memory every minute
    let monitor_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            let stats = memory_monitor.current_stats();
            
            info!(
                heap_mb = stats.heap_used / 1_000_000,
                rss_mb = stats.rss / 1_000_000,
                "Memory usage"
            );
            
            // Alert if growing
            if stats.heap_used > memory_monitor.baseline_heap * 2 {
                error!("Memory doubled - possible leak!");
            }
        }
    });
    
    LoadGenerator::new(config, "http://localhost:8080/graphql")
        .run()
        .await;
    
    monitor_handle.abort();
}
```

## Common Performance Issues

### 1. Connection Pool Exhaustion

**Symptom**: Errors under load, timeouts

**Diagnosis**:
```rust
// Monitor connection pool metrics
gauge!("db_pool_size").set(pool.size() as f64);
gauge!("db_pool_available").set(pool.available() as f64);
gauge!("db_pool_waiting").set(pool.waiting() as f64);
```

**Fix**:
```rust
// Increase pool size based on load
let pool_size = (expected_rps / 50).max(10).min(100);
config.database.pool_size = pool_size;
```

### 2. Slow Queries Under Load

**Symptom**: P99 latency spikes

**Diagnosis**:
```rust
// Log slow queries
if duration > Duration::from_millis(100) {
    warn!(
        query = %query,
        duration_ms = duration.as_millis(),
        "Slow query detected"
    );
}
```

**Fix**:
- Add database indexes
- Implement query complexity limits
- Use DataLoader for batching

### 3. Memory Growth

**Symptom**: RSS grows continuously

**Diagnosis**:
```rust
// Track allocations
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// Dump heap profile
jemalloc_ctl::prof::dump().unwrap();
```

**Fix**:
- Bound cache sizes
- Fix connection leaks
- Use Arc instead of cloning

## Load Test Best Practices

### 1. Test Production-Like Environment

```yaml
# docker-compose.test.yml
services:
  api:
    build: .
    environment:
      - RUST_LOG=info
      - DATABASE_URL=postgresql://...
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
    
  database:
    image: postgres:15
    # Same version as production!
```

### 2. Gradual Ramp-Up

```rust
// Don't slam the server immediately
config.ramp_up = Duration::from_secs(
    (config.target_rps / 100).max(10)
);
```

### 3. Realistic Data

```rust
// Use varied, realistic test data
let users = generate_test_users(1000);
let posts = generate_test_posts(10_000);

// Not just "test1", "test2"...
```

### 4. Monitor Everything

```rust
// During load test, track:
- Response times (all percentiles)
- Error rates and types
- Database metrics
- Cache hit rates
- CPU and memory usage
- Network I/O
- Disk I/O
```

### 5. Automate Performance Regression Detection

```rust
#[test]
fn benchmark_critical_queries() {
    // Run after each commit
    let results = run_standard_benchmark();
    
    // Compare with baseline
    let baseline = load_baseline_results();
    
    for (query, current) in results {
        let previous = baseline.get(query).unwrap();
        
        // Alert if > 10% regression
        if current.p50 > previous.p50 * 1.1 {
            panic!(
                "Performance regression in {}: {:.1}ms -> {:.1}ms",
                query, previous.p50, current.p50
            );
        }
    }
}
```

## Summary

Performance testing is not optional - it's how you ensure your API works in the real world. Key takeaways:

1. **Test realistically**: Use production-like queries and data
2. **Measure everything**: Latency, throughput, errors, resources
3. **Start early**: Don't wait until deployment
4. **Automate**: Make performance tests part of CI/CD
5. **Profile under load**: Find bottlenecks with real traffic

Remember: Users don't care about your elegant code if it's slow!
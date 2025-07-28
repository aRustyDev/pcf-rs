# Benchmark Methodology

This document describes the methodology used for benchmarking the PCF API, ensuring reproducible and meaningful performance measurements.

## Test Environment

### Hardware Specifications

#### Development Environment
- **CPU**: 8-core Apple M1 Pro / Intel i7-9750H
- **Memory**: 16GB DDR4
- **Storage**: NVMe SSD
- **Network**: Localhost (no network latency)

#### Production-like Environment
- **CPU**: 16-core AMD EPYC / Intel Xeon
- **Memory**: 32GB DDR4 ECC
- **Storage**: Enterprise NVMe SSD
- **Network**: 10Gbps within datacenter

### Software Configuration

#### API Server
- **Runtime**: Rust 1.75+ with release optimizations
- **Async Runtime**: Tokio with multi-threaded scheduler
- **Connection Pool**: 100 connections to database
- **Cache**: Redis with 1GB memory limit

#### Database
- **SurrealDB**: Latest stable version
- **Configuration**: Production-optimized settings
- **Storage**: 100GB pre-allocated

#### Load Testing Tools
- **Primary**: k6 for API load testing
- **Secondary**: wrk2 for latency measurements
- **Monitoring**: Prometheus + Grafana

## Load Patterns

### 1. Baseline Performance
Single-threaded sequential requests to establish baseline metrics:
- 1 request at a time
- No concurrency
- Measure pure processing time

### 2. Normal Load
Simulates typical production traffic:
- 100 concurrent users
- 60-second ramp-up
- 5-minute sustained load
- 30-second ramp-down

### 3. Peak Load
Simulates high-traffic periods:
- 1000 concurrent users
- 2-minute ramp-up
- 10-minute sustained load
- 1-minute ramp-down

### 4. Stress Test
Finds the breaking point:
- Start with 100 users
- Add 100 users every minute
- Continue until error rate > 5%
- Identify maximum capacity

### 5. Endurance Test
Validates stability over time:
- 500 concurrent users
- 24-hour duration
- Monitor for memory leaks
- Check performance degradation

## Measurement Techniques

### Response Time Metrics
- **p50 (Median)**: 50th percentile response time
- **p90**: 90th percentile response time
- **p95**: 95th percentile response time
- **p99**: 99th percentile response time
- **p99.9**: 99.9th percentile response time
- **Max**: Maximum observed response time

### Throughput Metrics
- **RPS**: Requests per second
- **TPS**: Transactions per second
- **Successful Requests**: Total successful requests
- **Failed Requests**: Total failed requests
- **Error Rate**: Percentage of failed requests

### Resource Utilization
- **CPU Usage**: Average and peak CPU utilization
- **Memory Usage**: Heap size, RSS, and garbage collection
- **Network I/O**: Bytes sent/received per second
- **Disk I/O**: Read/write operations per second
- **Connection Pool**: Active connections and wait time

## Test Scenarios

### GraphQL Query Tests

#### Simple Query
```graphql
query HealthCheck {
  health {
    status
    timestamp
  }
}
```
- **Complexity**: 1
- **Expected p99**: <5ms

#### Medium Complexity Query
```graphql
query GetNote($id: ID!) {
  note(id: $id) {
    id
    title
    content
    author {
      id
      name
    }
    tags
  }
}
```
- **Complexity**: ~10
- **Expected p99**: <20ms

#### Complex Query
```graphql
query ListNotesWithDetails($first: Int!) {
  notes(first: $first) {
    edges {
      node {
        id
        title
        content
        author {
          id
          name
          email
        }
        tags
        comments {
          id
          content
          author {
            name
          }
        }
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
    totalCount
  }
}
```
- **Complexity**: ~100
- **Expected p99**: <100ms

### REST Endpoint Tests

#### GET /health
- **Expected p99**: <2ms
- **Expected RPS**: >10,000

#### GET /api/notes/:id
- **Expected p99**: <15ms
- **Expected RPS**: >5,000

#### GET /api/notes?limit=100
- **Expected p99**: <50ms
- **Expected RPS**: >1,000

## Statistical Analysis

### Data Collection
- Collect data points every second
- Discard first 10% (warm-up)
- Discard last 5% (cool-down)
- Minimum 1000 data points per test

### Statistical Methods
- **Standard Deviation**: Measure consistency
- **Coefficient of Variation**: Relative variability
- **Outlier Detection**: Remove points > 3 standard deviations
- **Confidence Intervals**: 95% confidence level

### Reporting Format
```
Metric: p99 Response Time
Value: 45.2ms
Std Dev: 3.1ms
CV: 6.86%
95% CI: [44.1ms, 46.3ms]
Samples: 5000
```

## Reproducibility Guidelines

### Environment Setup
1. Use containerized environment
2. Fix CPU governor to performance mode
3. Disable CPU frequency scaling
4. Ensure no background processes
5. Pre-warm all caches

### Test Execution
1. Run each test 5 times
2. Report median of 5 runs
3. Include variance information
4. Document any anomalies
5. Save raw data for analysis

### Configuration Files
Store all configuration in version control:
```yaml
# benchmark-config.yaml
environment:
  api_version: "1.0.0"
  rust_version: "1.75.0"
  surrealdb_version: "1.0.0"
  
test_parameters:
  warmup_duration: "30s"
  test_duration: "300s"
  cooldown_duration: "15s"
  
load_pattern:
  type: "constant"
  users: 100
  ramp_up: "60s"
```

## Benchmark Automation

### CI/CD Integration
```yaml
# .github/workflows/benchmark.yml
name: Performance Benchmark
on:
  push:
    branches: [main]
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM

jobs:
  benchmark:
    runs-on: benchmark-runner
    steps:
      - uses: actions/checkout@v3
      - name: Run Benchmarks
        run: ./scripts/run-benchmarks.sh
      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: results/
```

### Result Storage
- Store results in time-series database
- Track performance over time
- Alert on significant regressions
- Generate trend reports

## Common Pitfalls to Avoid

1. **Cold Start Effects**: Always warm up before measuring
2. **Network Variability**: Test on consistent network
3. **Resource Contention**: Isolate test environment
4. **Caching Effects**: Clear caches between test types
5. **Time-of-Day Effects**: Run tests at consistent times
6. **Version Mismatches**: Document all versions

## Interpreting Results

### Performance Goals
- **p50**: Typical user experience
- **p99**: Worst-case for most users
- **p99.9**: Edge cases and outliers
- **Error Rate**: Must be < 0.1% under normal load

### Red Flags
- High variance between runs
- Increasing latency over time
- Memory usage growing unbounded
- Error rate > 1%
- CPU constantly at 100%

## Further Reading
- [Performance Testing Best Practices](/developer/performance/testing.md)
- [Optimization Guide](/developer/performance/optimization.md)
- [Monitoring Setup](/admin/observability/metrics.md)

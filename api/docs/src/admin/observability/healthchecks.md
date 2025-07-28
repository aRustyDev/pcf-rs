# Health Checks

Comprehensive guide to implementing and managing health checks for the PCF API, including liveness probes, readiness probes, and dependency monitoring.

<!-- toc -->

## Overview

Health checks provide critical information about the operational status of the PCF API and its dependencies. They enable container orchestrators, load balancers, and monitoring systems to make informed decisions about traffic routing and service availability.

## Health Check Types

### 1. Liveness Probe

Indicates whether the application is running. If the liveness probe fails, the container should be restarted.

**Purpose**: Detect and recover from deadlocks or unresponsive states  
**Path**: `/health`  
**Expected Status**: 200 OK if alive, 503 if dead

### 2. Readiness Probe

Indicates whether the application is ready to serve requests. If the readiness probe fails, traffic should not be routed to this instance.

**Purpose**: Prevent routing traffic to instances that aren't ready  
**Path**: `/health/ready`  
**Expected Status**: 200 OK if ready, 503 if not ready

### 3. Startup Probe

Indicates whether the application has started successfully. Used for applications with long initialization times.

**Purpose**: Allow extra time for startup without failing liveness checks  
**Path**: `/health/startup`  
**Expected Status**: 200 OK once started, 503 during startup

## Implementation

### Basic Health Check Structure

```rust
use axum::{
    routing::get,
    Router,
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HealthDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDetails {
    pub checks: HashMap<String, ComponentHealth>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthState,
    pub message: Option<String>,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: Option<u64>,
}

pub struct HealthChecker {
    start_time: Instant,
    checks: Arc<RwLock<HashMap<String, Box<dyn HealthCheck>>>>,
    config: HealthConfig,
}

#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check(&self) -> ComponentHealth;
    fn name(&self) -> &str;
}
```

### Liveness Probe Implementation

```rust
pub async fn liveness_handler(
    State(health): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    // Simple liveness check - just verify the process is responsive
    let uptime = health.start_time.elapsed().as_secs();
    
    let status = HealthStatus {
        status: HealthState::Healthy,
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        details: None,
    };
    
    (StatusCode::OK, Json(status))
}
```

### Readiness Probe Implementation

```rust
pub async fn readiness_handler(
    State(health): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    let checks = health.run_all_checks().await;
    let overall_status = determine_overall_status(&checks);
    
    let status = HealthStatus {
        status: overall_status.clone(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: health.start_time.elapsed().as_secs(),
        details: Some(HealthDetails {
            checks,
            metadata: collect_metadata(),
        }),
    };
    
    let status_code = match overall_status {
        HealthState::Healthy => StatusCode::OK,
        HealthState::Degraded => StatusCode::OK, // Still accept traffic
        HealthState::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };
    
    (status_code, Json(status))
}

fn determine_overall_status(checks: &HashMap<String, ComponentHealth>) -> HealthState {
    let mut has_degraded = false;
    
    for (name, health) in checks {
        match health.status {
            HealthState::Unhealthy => {
                // Critical components make the whole service unhealthy
                if is_critical_component(name) {
                    return HealthState::Unhealthy;
                }
                has_degraded = true;
            }
            HealthState::Degraded => has_degraded = true,
            HealthState::Healthy => {}
        }
    }
    
    if has_degraded {
        HealthState::Degraded
    } else {
        HealthState::Healthy
    }
}

fn is_critical_component(name: &str) -> bool {
    matches!(name, "database" | "auth_service")
}
```

### Startup Probe Implementation

```rust
pub struct StartupState {
    initialized: AtomicBool,
    start_time: Instant,
    max_startup_time: Duration,
}

pub async fn startup_handler(
    State(startup): State<Arc<StartupState>>,
) -> impl IntoResponse {
    if startup.initialized.load(Ordering::Relaxed) {
        (
            StatusCode::OK,
            Json(json!({
                "status": "started",
                "startup_time_ms": startup.start_time.elapsed().as_millis(),
            }))
        )
    } else if startup.start_time.elapsed() > startup.max_startup_time {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "timeout",
                "message": "Startup exceeded maximum allowed time",
            }))
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "starting",
                "elapsed_ms": startup.start_time.elapsed().as_millis(),
            }))
        )
    }
}
```

## Component Health Checks

### Database Health Check

```rust
pub struct DatabaseHealthCheck {
    pool: Arc<DatabasePool>,
    timeout: Duration,
}

#[async_trait]
impl HealthCheck for DatabaseHealthCheck {
    async fn check(&self) -> ComponentHealth {
        let start = Instant::now();
        
        let result = tokio::time::timeout(
            self.timeout,
            self.check_database()
        ).await;
        
        let response_time_ms = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(Ok(_)) => ComponentHealth {
                status: HealthState::Healthy,
                message: None,
                last_check: Utc::now(),
                response_time_ms: Some(response_time_ms),
            },
            Ok(Err(e)) => ComponentHealth {
                status: HealthState::Unhealthy,
                message: Some(format!("Database error: {}", e)),
                last_check: Utc::now(),
                response_time_ms: Some(response_time_ms),
            },
            Err(_) => ComponentHealth {
                status: HealthState::Unhealthy,
                message: Some("Database check timed out".to_string()),
                last_check: Utc::now(),
                response_time_ms: None,
            },
        }
    }
    
    fn name(&self) -> &str {
        "database"
    }
}

impl DatabaseHealthCheck {
    async fn check_database(&self) -> Result<()> {
        // Run a simple query
        sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await?;
        
        // Check connection pool health
        let state = self.pool.state();
        if state.idle_connections == 0 && state.active_connections >= state.max_connections {
            return Err(anyhow!("Connection pool exhausted"));
        }
        
        Ok(())
    }
}
```

### Cache Health Check

```rust
pub struct CacheHealthCheck {
    client: Arc<RedisClient>,
}

#[async_trait]
impl HealthCheck for CacheHealthCheck {
    async fn check(&self) -> ComponentHealth {
        let start = Instant::now();
        
        match self.check_cache().await {
            Ok(_) => ComponentHealth {
                status: HealthState::Healthy,
                message: None,
                last_check: Utc::now(),
                response_time_ms: Some(start.elapsed().as_millis() as u64),
            },
            Err(e) => ComponentHealth {
                status: HealthState::Degraded, // Cache is not critical
                message: Some(format!("Cache error: {}", e)),
                last_check: Utc::now(),
                response_time_ms: None,
            },
        }
    }
    
    fn name(&self) -> &str {
        "cache"
    }
}

impl CacheHealthCheck {
    async fn check_cache(&self) -> Result<()> {
        // Ping Redis
        let _: String = self.client.ping().await?;
        
        // Test set/get
        let key = format!("health_check_{}", Uuid::new_v4());
        self.client.set(&key, "test", Some(10)).await?;
        let value: String = self.client.get(&key).await?;
        
        if value != "test" {
            return Err(anyhow!("Cache read/write test failed"));
        }
        
        Ok(())
    }
}
```

### External Service Health Check

```rust
pub struct ExternalServiceHealthCheck {
    name: String,
    url: String,
    client: reqwest::Client,
    timeout: Duration,
}

#[async_trait]
impl HealthCheck for ExternalServiceHealthCheck {
    async fn check(&self) -> ComponentHealth {
        let start = Instant::now();
        
        let result = self.client
            .get(&self.url)
            .timeout(self.timeout)
            .send()
            .await;
        
        match result {
            Ok(response) if response.status().is_success() => ComponentHealth {
                status: HealthState::Healthy,
                message: None,
                last_check: Utc::now(),
                response_time_ms: Some(start.elapsed().as_millis() as u64),
            },
            Ok(response) => ComponentHealth {
                status: HealthState::Unhealthy,
                message: Some(format!("Service returned status: {}", response.status())),
                last_check: Utc::now(),
                response_time_ms: Some(start.elapsed().as_millis() as u64),
            },
            Err(e) => ComponentHealth {
                status: HealthState::Unhealthy,
                message: Some(format!("Service unreachable: {}", e)),
                last_check: Utc::now(),
                response_time_ms: None,
            },
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}
```

### Disk Space Health Check

```rust
pub struct DiskSpaceHealthCheck {
    path: PathBuf,
    warning_threshold: f64,  // e.g., 0.8 for 80%
    critical_threshold: f64, // e.g., 0.9 for 90%
}

#[async_trait]
impl HealthCheck for DiskSpaceHealthCheck {
    async fn check(&self) -> ComponentHealth {
        match fs2::available_space(&self.path) {
            Ok(available) => {
                match fs2::total_space(&self.path) {
                    Ok(total) => {
                        let usage = 1.0 - (available as f64 / total as f64);
                        
                        let (status, message) = if usage >= self.critical_threshold {
                            (
                                HealthState::Unhealthy,
                                Some(format!("Disk usage critical: {:.1}%", usage * 100.0))
                            )
                        } else if usage >= self.warning_threshold {
                            (
                                HealthState::Degraded,
                                Some(format!("Disk usage warning: {:.1}%", usage * 100.0))
                            )
                        } else {
                            (
                                HealthState::Healthy,
                                Some(format!("Disk usage: {:.1}%", usage * 100.0))
                            )
                        };
                        
                        ComponentHealth {
                            status,
                            message,
                            last_check: Utc::now(),
                            response_time_ms: Some(0),
                        }
                    }
                    Err(e) => ComponentHealth {
                        status: HealthState::Unhealthy,
                        message: Some(format!("Failed to get total space: {}", e)),
                        last_check: Utc::now(),
                        response_time_ms: None,
                    }
                }
            }
            Err(e) => ComponentHealth {
                status: HealthState::Unhealthy,
                message: Some(format!("Failed to get available space: {}", e)),
                last_check: Utc::now(),
                response_time_ms: None,
            }
        }
    }
    
    fn name(&self) -> &str {
        "disk_space"
    }
}
```

## Configuration

### Health Check Configuration

```toml
[health]
liveness_path = "/health"
readiness_path = "/health/ready"
startup_path = "/health/startup"

# Include detailed component status in responses
include_details = true

# Cache health check results
cache_duration = 5  # seconds

# Component-specific settings
[health.database]
enabled = true
timeout = 5000  # milliseconds
critical = true

[health.cache]
enabled = true
timeout = 1000
critical = false

[health.disk_space]
enabled = true
path = "/data"
warning_threshold = 0.8  # 80%
critical_threshold = 0.9 # 90%

[health.external_services]
auth_service = { url = "http://auth-service/health", timeout = 2000 }
payment_service = { url = "http://payment-service/health", timeout = 3000 }
```

### Kubernetes Configuration

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pcf-api
spec:
  template:
    spec:
      containers:
      - name: api
        image: pcf-api:latest
        ports:
        - containerPort: 8080
        
        # Liveness probe
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          successThreshold: 1
          failureThreshold: 3
        
        # Readiness probe
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          successThreshold: 1
          failureThreshold: 3
        
        # Startup probe (K8s 1.16+)
        startupProbe:
          httpGet:
            path: /health/startup
            port: 8080
          initialDelaySeconds: 0
          periodSeconds: 10
          timeoutSeconds: 5
          successThreshold: 1
          failureThreshold: 30  # 5 minutes max startup
```

## Health Check Registry

```rust
impl HealthChecker {
    pub fn new(config: HealthConfig) -> Self {
        Self {
            start_time: Instant::now(),
            checks: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    pub async fn register(&self, check: Box<dyn HealthCheck>) {
        let mut checks = self.checks.write().await;
        checks.insert(check.name().to_string(), check);
    }
    
    pub async fn run_all_checks(&self) -> HashMap<String, ComponentHealth> {
        let checks = self.checks.read().await;
        let mut results = HashMap::new();
        
        // Run checks concurrently
        let futures: Vec<_> = checks
            .iter()
            .map(|(name, check)| async move {
                (name.clone(), check.check().await)
            })
            .collect();
        
        let completed = futures::future::join_all(futures).await;
        
        for (name, health) in completed {
            results.insert(name, health);
        }
        
        results
    }
    
    pub async fn run_check(&self, name: &str) -> Option<ComponentHealth> {
        let checks = self.checks.read().await;
        if let Some(check) = checks.get(name) {
            Some(check.check().await)
        } else {
            None
        }
    }
}
```

## Caching Health Results

```rust
pub struct CachedHealthChecker {
    checker: Arc<HealthChecker>,
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    cache_duration: Duration,
}

struct CachedResult {
    health: ComponentHealth,
    cached_at: Instant,
}

impl CachedHealthChecker {
    pub async fn check(&self, name: &str) -> Option<ComponentHealth> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(name) {
                if cached.cached_at.elapsed() < self.cache_duration {
                    return Some(cached.health.clone());
                }
            }
        }
        
        // Run fresh check
        if let Some(health) = self.checker.run_check(name).await {
            // Update cache
            let mut cache = self.cache.write().await;
            cache.insert(name.to_string(), CachedResult {
                health: health.clone(),
                cached_at: Instant::now(),
            });
            
            Some(health)
        } else {
            None
        }
    }
}
```

## Monitoring Health Checks

### Prometheus Metrics

```rust
lazy_static! {
    static ref HEALTH_CHECK_DURATION: HistogramVec = register_histogram_vec!(
        "health_check_duration_seconds",
        "Health check duration",
        &["component"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap();
    
    static ref HEALTH_CHECK_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "health_check_status",
        "Health check status (0=healthy, 1=degraded, 2=unhealthy)",
        &["component"]
    ).unwrap();
    
    static ref HEALTH_CHECK_FAILURES: CounterVec = register_counter_vec!(
        "health_check_failures_total",
        "Total health check failures",
        &["component", "reason"]
    ).unwrap();
}

impl HealthChecker {
    async fn check_with_metrics(&self, name: &str) -> Option<ComponentHealth> {
        let timer = HEALTH_CHECK_DURATION
            .with_label_values(&[name])
            .start_timer();
        
        let result = self.run_check(name).await;
        timer.observe_duration();
        
        if let Some(health) = &result {
            let status_value = match health.status {
                HealthState::Healthy => 0,
                HealthState::Degraded => 1,
                HealthState::Unhealthy => 2,
            };
            
            HEALTH_CHECK_STATUS
                .with_label_values(&[name])
                .set(status_value);
            
            if health.status == HealthState::Unhealthy {
                let reason = health.message.as_deref().unwrap_or("unknown");
                HEALTH_CHECK_FAILURES
                    .with_label_values(&[name, reason])
                    .inc();
            }
        }
        
        result
    }
}
```

### Alerting Rules

```yaml
groups:
  - name: health_checks
    rules:
      - alert: ServiceUnhealthy
        expr: health_check_status{component="database"} == 2
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Database health check failing"
          description: "Database has been unhealthy for 5 minutes"
      
      - alert: ServiceDegraded
        expr: health_check_status == 1
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "Service {{ $labels.component }} degraded"
          description: "{{ $labels.component }} has been degraded for 15 minutes"
      
      - alert: HealthCheckSlow
        expr: |
          histogram_quantile(0.95, 
            rate(health_check_duration_seconds_bucket[5m])
          ) > 1.0
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Health checks are slow"
          description: "P95 health check latency is above 1 second"
```

## Advanced Health Checks

### Dependency Graph Health

```rust
pub struct DependencyHealthChecker {
    dependencies: HashMap<String, Vec<String>>,
}

impl DependencyHealthChecker {
    pub async fn check_with_dependencies(
        &self,
        checker: &HealthChecker,
    ) -> HashMap<String, ComponentHealth> {
        let mut results = HashMap::new();
        let mut checked = HashSet::new();
        
        for root in self.find_roots() {
            self.check_recursive(&root, checker, &mut results, &mut checked).await;
        }
        
        results
    }
    
    async fn check_recursive(
        &self,
        component: &str,
        checker: &HealthChecker,
        results: &mut HashMap<String, ComponentHealth>,
        checked: &mut HashSet<String>,
    ) {
        if checked.contains(component) {
            return;
        }
        
        // Check dependencies first
        if let Some(deps) = self.dependencies.get(component) {
            for dep in deps {
                self.check_recursive(dep, checker, results, checked).await;
            }
        }
        
        // Check this component
        if let Some(health) = checker.run_check(component).await {
            // If dependencies are unhealthy, mark this as degraded
            let dep_health = self.get_dependency_health(component, results);
            let final_health = self.combine_health(health, dep_health);
            
            results.insert(component.to_string(), final_health);
        }
        
        checked.insert(component.to_string());
    }
}
```

### Circuit Breaker Integration

```rust
pub struct CircuitBreakerHealthCheck {
    circuit_breaker: Arc<CircuitBreaker>,
    inner_check: Box<dyn HealthCheck>,
}

#[async_trait]
impl HealthCheck for CircuitBreakerHealthCheck {
    async fn check(&self) -> ComponentHealth {
        match self.circuit_breaker.state() {
            CircuitState::Closed => {
                // Normal operation
                self.inner_check.check().await
            }
            CircuitState::Open => {
                // Circuit is open, report unhealthy
                ComponentHealth {
                    status: HealthState::Unhealthy,
                    message: Some("Circuit breaker is open".to_string()),
                    last_check: Utc::now(),
                    response_time_ms: None,
                }
            }
            CircuitState::HalfOpen => {
                // Test the service
                let health = self.inner_check.check().await;
                
                // Update circuit breaker based on result
                match health.status {
                    HealthState::Healthy => self.circuit_breaker.on_success(),
                    _ => self.circuit_breaker.on_failure(),
                }
                
                health
            }
        }
    }
    
    fn name(&self) -> &str {
        self.inner_check.name()
    }
}
```

## Best Practices

### 1. Keep Health Checks Fast

```rust
// Use timeouts to prevent hanging
let result = tokio::time::timeout(
    Duration::from_secs(5),
    expensive_check()
).await;
```

### 2. Don't Break on Non-Critical Dependencies

```rust
// Mark non-critical services appropriately
match external_service_check().await {
    Ok(_) => HealthState::Healthy,
    Err(_) => HealthState::Degraded, // Not Unhealthy
}
```

### 3. Include Useful Context

```rust
ComponentHealth {
    status: HealthState::Unhealthy,
    message: Some(format!(
        "Database connection pool exhausted: {}/{} connections in use",
        state.active_connections,
        state.max_connections
    )),
    last_check: Utc::now(),
    response_time_ms: Some(response_time),
}
```

### 4. Secure Health Endpoints

```rust
// Optionally require auth for detailed health info
pub async fn detailed_health_handler(
    auth: RequireAuth,
    State(health): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    // Only authenticated users can see detailed health
    let checks = health.run_all_checks().await;
    Json(checks)
}
```

### 5. Version Your Health Check API

```rust
#[derive(Serialize)]
struct HealthResponseV2 {
    api_version: &'static str,
    #[serde(flatten)]
    status: HealthStatus,
}

pub async fn health_v2_handler() -> impl IntoResponse {
    Json(HealthResponseV2 {
        api_version: "v2",
        status: get_health_status().await,
    })
}
```

## Troubleshooting

### Health Check Failures

```bash
# Check specific component
curl http://localhost:8080/health/ready | jq '.details.checks.database'

# Monitor health check metrics
curl http://localhost:9090/metrics | grep health_check

# View health check logs
kubectl logs -f deployment/pcf-api | grep -i health
```

### Performance Issues

```rust
// Add health check timing logs
debug!(
    component = %name,
    duration_ms = %duration.as_millis(),
    "Health check completed"
);
```

## Summary

Effective health checks require:
1. **Clear purpose** - Separate liveness, readiness, and startup concerns
2. **Fast execution** - Use timeouts and caching appropriately
3. **Meaningful status** - Distinguish between healthy, degraded, and unhealthy
4. **Rich context** - Include useful debugging information
5. **Proper integration** - Work well with orchestrators and monitoring
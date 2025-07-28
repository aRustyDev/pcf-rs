# Health Check Specification

## Core Principles

1. **Fast Response**: Health checks MUST complete within 5 seconds or return 503
2. **No Authentication**: Health endpoints MUST NOT require authentication
3. **Cached Results**: Results SHOULD be cached to prevent overload
4. **Graceful Degradation**: System SHOULD continue operating when non-critical services fail
5. **Clear Status**: MUST clearly indicate system state for operators and orchestrators

## Endpoints

### GET /health (Liveness)
Simple check to verify the server process is running and can handle HTTP requests.

**Response Requirements**:
- MUST return 200 OK with body "OK" if process is alive
- MUST NOT check external dependencies
- MUST respond within 1 second
- MUST use plain text, not JSON
- MUST NOT require authentication

**Implementation**:
```rust
async fn liveness_check() -> impl IntoResponse {
    // Simply return OK - proves the server can handle requests
    (StatusCode::OK, "OK")
}
```

**Used by**:
- Docker HEALTHCHECK directive
- Kubernetes liveness probe
- Load balancer health checks
- Simple uptime monitoring

### GET /health/ready (Readiness)
Comprehensive check of all service dependencies.

**Response Format**:
```json
{
  "status": "healthy|degraded|unhealthy|starting",
  "timestamp": "2024-01-01T00:00:00.000Z",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "services": {
    "database": {
      "status": "healthy",
      "latency_ms": 5,
      "last_check": "2024-01-01T00:00:00.000Z",
      "message": null
    },
    "graphql": {
      "status": "healthy", 
      "schema_loaded": true,
      "subscription_support": true
    },
    "spicedb": {
      "status": "unhealthy",
      "latency_ms": null,
      "last_check": "2024-01-01T00:00:00.000Z",
      "message": "Connection refused"
    }
  }
}
```

**Status Codes**:
- 200 OK: System is healthy or degraded (can handle requests)
- 503 Service Unavailable: System is unhealthy (cannot handle requests)

**Response Time Requirements**:
- MUST complete all checks within 5 seconds
- SHOULD return cached results if fresh check would exceed timeout
- MUST include `Cache-Control: no-cache` header

## Health Check Implementation

### Database Health Check
```rust
async fn check_database_health(db: &Database) -> ServiceHealth {
    let start = Instant::now();
    
    // Use a simple, fast query that exercises basic functionality
    match timeout(Duration::from_secs(2), db.execute("SELECT 1")).await {
        Ok(Ok(_)) => {
            let latency = start.elapsed().as_millis() as u64;
            ServiceHealth {
                status: match latency {
                    0..=100 => "healthy",
                    101..=1000 => "degraded", 
                    _ => "unhealthy"
                },
                latency_ms: Some(latency),
                last_check: Some(Utc::now()),
                message: None,
                consecutive_failures: 0,
            }
        }
        Ok(Err(e)) => {
            error!(error = ?e, "Database health check failed");
            ServiceHealth {
                status: "unhealthy",
                latency_ms: None,
                last_check: Some(Utc::now()),
                message: Some("Database query failed".to_string()), // Don't expose internal error
                consecutive_failures: self.increment_failure_count("database"),
            }
        }
        Err(_) => ServiceHealth {
            status: "unhealthy",
            latency_ms: None,
            last_check: Some(Utc::now()),
            message: Some("Health check timeout after 2s".to_string()),
            consecutive_failures: self.increment_failure_count("database"),
        },
    }
}
```

### SpiceDB Health Check
```rust
async fn check_spicedb_health(client: &SpiceDBClient) -> ServiceHealth {
    let start = Instant::now();
    
    // Use SpiceDB's gRPC health check or a simple permission check
    match timeout(Duration::from_secs(5), client.health_check()).await {
        Ok(Ok(_)) => {
            let latency = start.elapsed().as_millis() as u64;
            ServiceHealth {
                status: if latency < 500 { "healthy" } else { "degraded" },
                latency_ms: Some(latency),
                last_check: Some(Utc::now()),
                message: None,
            }
        }
        // ... error handling
    }
}
```

### GraphQL Health Check
```rust
async fn check_graphql_health(schema: &Schema) -> ServiceHealth {
    // Verify schema is loaded and can handle introspection
    let query = "{ __schema { queryType { name } } }";
    
    match schema.execute(query).await {
        Ok(response) if response.errors.is_empty() => ServiceHealth {
            status: "healthy",
            schema_loaded: true,
            subscription_support: true,
        },
        _ => ServiceHealth {
            status: "unhealthy",
            schema_loaded: false,
            subscription_support: false,
        },
    }
}
```

## Overall Status Determination

```rust
#[derive(Clone, Copy)]
enum ServiceCriticality {
    Critical,      // System cannot function without this
    Important,     // System degrades significantly without this  
    Optional,      // Nice to have but not required
}

struct ServiceDefinition {
    name: &'static str,
    criticality: ServiceCriticality,
    startup_grace_period: Duration,
}

const SERVICE_DEFINITIONS: &[ServiceDefinition] = &[
    ServiceDefinition {
        name: "database",
        criticality: ServiceCriticality::Critical,
        startup_grace_period: Duration::from_secs(30),
    },
    ServiceDefinition {
        name: "graphql",
        criticality: ServiceCriticality::Critical,
        startup_grace_period: Duration::from_secs(10),
    },
    ServiceDefinition {
        name: "spicedb",
        criticality: ServiceCriticality::Important,
        startup_grace_period: Duration::from_secs(60),
    },
    ServiceDefinition {
        name: "auth",
        criticality: ServiceCriticality::Important,
        startup_grace_period: Duration::from_secs(30),
    },
    ServiceDefinition {
        name: "cache",
        criticality: ServiceCriticality::Optional,
        startup_grace_period: Duration::from_secs(10),
    },
];

fn determine_overall_status(
    services: &HashMap<String, ServiceHealth>,
    start_time: Instant,
) -> OverallHealth {
    let uptime = start_time.elapsed();
    
    // Starting phase - be lenient
    if uptime < Duration::from_secs(30) {
        return OverallHealth {
            status: "starting",
            can_serve_traffic: false,
            description: "System is initializing".to_string(),
        };
    }
    
    let mut critical_failures = vec![];
    let mut important_failures = vec![];
    let mut optional_failures = vec![];
    
    for service_def in SERVICE_DEFINITIONS {
        if let Some(health) = services.get(service_def.name) {
            // Skip services still in grace period
            if uptime < service_def.startup_grace_period {
                continue;
            }
            
            if health.status == "unhealthy" {
                match service_def.criticality {
                    ServiceCriticality::Critical => {
                        critical_failures.push(service_def.name);
                    }
                    ServiceCriticality::Important => {
                        important_failures.push(service_def.name);
                    }
                    ServiceCriticality::Optional => {
                        optional_failures.push(service_def.name);
                    }
                }
            }
        }
    }
    
    // Determine overall status
    if !critical_failures.is_empty() {
        OverallHealth {
            status: "unhealthy",
            can_serve_traffic: false,
            description: format!(
                "Critical services failed: {}",
                critical_failures.join(", ")
            ),
        }
    } else if !important_failures.is_empty() {
        OverallHealth {
            status: "degraded",
            can_serve_traffic: true,
            description: format!(
                "Important services failed: {}. System operating with reduced functionality.",
                important_failures.join(", ")
            ),
        }
    } else if !optional_failures.is_empty() {
        OverallHealth {
            status: "healthy",
            can_serve_traffic: true,
            description: format!(
                "Optional services unavailable: {}",
                optional_failures.join(", ")
            ),
        }
    } else {
        OverallHealth {
            status: "healthy",
            can_serve_traffic: true,
            description: "All systems operational".to_string(),
        }
    }
}
```

## Caching and Rate Limiting

### Cache Strategy
Health check results MUST be cached to prevent overload and ensure fast responses:

```rust
struct HealthCache {
    last_full_check: Instant,
    cached_result: Option<HealthStatus>,
    check_in_progress: Arc<Mutex<bool>>,
    cache_ttl: Duration,
    stale_ttl: Duration,  // How long to serve stale data if check fails
}

impl HealthCache {
    fn new() -> Self {
        Self {
            last_full_check: Instant::now(),
            cached_result: None,
            check_in_progress: Arc::new(Mutex::new(false)),
            cache_ttl: Duration::from_secs(5),      // Fresh for 5 seconds
            stale_ttl: Duration::from_secs(30),     // Serve stale up to 30s
        }
    }
    
    async fn get_status(&self) -> Result<HealthStatus> {
        let now = Instant::now();
        let age = now.duration_since(self.last_full_check);
        
        // Return fresh cached result
        if age < self.cache_ttl {
            if let Some(cached) = &self.cached_result {
                return Ok(cached.clone());
            }
        }
        
        // Try to acquire lock for new check
        let mut checking = self.check_in_progress.try_lock();
        if let Ok(ref mut lock) = checking {
            if !**lock {
                **lock = true;
                drop(checking);
                
                // Perform check with timeout
                match timeout(Duration::from_secs(5), perform_health_checks()).await {
                    Ok(Ok(status)) => {
                        self.cached_result = Some(status.clone());
                        self.last_full_check = now;
                        *self.check_in_progress.lock().unwrap() = false;
                        return Ok(status);
                    }
                    _ => {
                        *self.check_in_progress.lock().unwrap() = false;
                        // Fall through to stale data
                    }
                }
            }
        }
        
        // Return stale data if available and not too old
        if age < self.stale_ttl {
            if let Some(cached) = &self.cached_result {
                let mut stale = cached.clone();
                stale.cache_status = "stale";
                return Ok(stale);
            }
        }
        
        // No data available
        Err(HealthCheckError::NoDataAvailable)
    }
}
```

### Rate Limiting
Health endpoints SHOULD implement rate limiting to prevent abuse:

```rust
// Per-IP rate limiting
const HEALTH_CHECK_RATE_LIMIT: u32 = 10;  // requests
const HEALTH_CHECK_WINDOW: Duration = Duration::from_secs(60);  // per minute

// But always allow orchestrator IPs
const ORCHESTRATOR_CIDRS: &[&str] = &[
    "10.0.0.0/8",      // Kubernetes pod network
    "172.16.0.0/12",   // Docker default
    "127.0.0.1/32",    // Localhost
];
```

## CLI Health Check

```rust
// pcf-api healthcheck
async fn run_healthcheck() -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    
    let response = client
        .get("http://localhost:4000/health/ready")
        .send()
        .await?;
    
    let status: HealthStatus = response.json().await?;
    
    // Pretty print status
    println!("Status: {}", status.status);
    println!("Version: {}", status.version);
    println!("Uptime: {}s", status.uptime_seconds);
    
    for (service, health) in &status.services {
        println!("  {}: {}", service, health.status);
        if let Some(latency) = health.latency_ms {
            println!("    Latency: {}ms", latency);
        }
        if let Some(msg) = &health.message {
            println!("    Message: {}", msg);
        }
    }
    
    match status.status.as_str() {
        "healthy" => Ok(()),
        "degraded" => Ok(()),
        _ => Err(anyhow!("System unhealthy")),
    }
}
```

## Container Orchestration Integration

### Docker HEALTHCHECK
```dockerfile
# Liveness check only - simple and fast
HEALTHCHECK --interval=30s --timeout=3s --start-period=60s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1
```

### Kubernetes Probes
```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: pcf-api
    # Liveness: Restart container if it's dead
    livenessProbe:
      httpGet:
        path: /health
        port: 8080
      initialDelaySeconds: 10
      periodSeconds: 30
      timeoutSeconds: 3
      failureThreshold: 3
    
    # Readiness: Remove from service if not ready
    readinessProbe:
      httpGet:
        path: /health/ready
        port: 8080
      initialDelaySeconds: 30  # Give time for DB connections
      periodSeconds: 10
      timeoutSeconds: 5
      successThreshold: 1
      failureThreshold: 3
    
    # Startup: Extended time for initial startup
    startupProbe:
      httpGet:
        path: /health/ready
        port: 8080
      initialDelaySeconds: 0
      periodSeconds: 10
      timeoutSeconds: 5
      failureThreshold: 30  # 5 minutes total
```

## Degraded Mode Operations

### Service-Specific Degradation

**Database Unavailable:**
- MUST return 503 for write operations
- MAY serve cached data for read operations
- MUST queue critical writes up to limit
- SHOULD attempt reconnection with backoff

**Authorization Service (SpiceDB) Unavailable:**
- MUST use cached authorization decisions if available (5 min TTL)
- MUST deny access if no cached decision exists
- MUST log all cache-based decisions
- SHOULD alert on cache miss rate > 10%

**Authentication Service Unavailable:**
- MUST honor existing valid sessions
- MUST reject new login attempts with 503
- MAY extend session timeout during outage
- MUST NOT create new sessions

**Cache Service Unavailable:**
- MUST continue without caching
- SHOULD log performance impact
- MAY increase database connection pool

### Graceful Degradation Example
```rust
async fn handle_request_with_degradation(
    services: &ServiceRegistry,
    request: Request,
) -> Result<Response> {
    // Check if we can handle this request
    let health = services.get_health_status();
    
    match (&request.operation, &health.status) {
        // Always allow health checks
        (Operation::HealthCheck, _) => handle_health_check(),
        
        // Block writes during critical failures
        (Operation::Write(_), HealthStatus::Unhealthy) => {
            Err(ServiceError::ServiceUnavailable(
                "System is not accepting writes at this time".to_string()
            ))
        }
        
        // Attempt reads with degradation
        (Operation::Read(query), HealthStatus::Degraded) => {
            // Try cache first
            if let Some(cached) = cache.get(&query.cache_key()).await {
                info!("Serving from cache due to degraded status");
                return Ok(Response::from_cache(cached));
            }
            
            // Try database with shorter timeout
            match timeout(Duration::from_secs(2), db.query(&query)).await {
                Ok(Ok(result)) => Ok(Response::from_db(result)),
                _ => Err(ServiceError::ServiceUnavailable(
                    "Unable to process request in degraded mode".to_string()
                ))
            }
        }
        
        // Normal operations when healthy
        _ => handle_normal_request(request).await,
    }
}
```
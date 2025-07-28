# Feature Flags

Comprehensive guide to managing feature flags and conditional functionality in the PCF API.

<!-- toc -->

## Overview

Feature flags allow you to control application behavior without code deployments. The PCF API supports multiple types of feature flags for different use cases including gradual rollouts, A/B testing, maintenance modes, and experimental features.

## Feature Flag Types

### System Feature Flags

Core system-level features controlled by configuration:

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `demo_mode` | Boolean | `false` | Enable demo mode with mock data |
| `maintenance_mode` | Boolean | `false` | Enable maintenance mode |
| `read_only` | Boolean | `false` | Restrict to read-only operations |
| `experimental` | Boolean | `false` | Enable experimental features |

### Environment Configuration

```toml
# config/production.toml
[features]
demo_mode = false         # Must be false in production
maintenance_mode = false  # Temporary maintenance
read_only = false        # Emergency read-only mode
experimental = false     # Beta features
```

Environment variables:
```bash
export PCF_API__FEATURES__DEMO_MODE=false
export PCF_API__FEATURES__MAINTENANCE_MODE=true
export PCF_API__FEATURES__READ_ONLY=false
export PCF_API__FEATURES__EXPERIMENTAL=false
```

## Demo Mode

### Overview

Demo mode provides a safe environment for testing and demonstrations without affecting real data.

### Configuration

```toml
[features]
demo_mode = true

[features.demo]
mock_users = 100         # Number of mock users
mock_data_seed = 42      # Seed for consistent data
reset_interval = 3600    # Reset data every hour
allow_mutations = true   # Allow data modifications
```

### Implementation

```rust
// Compile-time safety check
#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");

// Runtime check
pub fn is_demo_mode(config: &AppConfig) -> bool {
    config.features.demo_mode && cfg!(debug_assertions)
}

// Demo mode middleware
pub async fn demo_mode_check(
    State(config): State<AppConfig>,
    request: Request,
    next: Next,
) -> Response {
    if config.features.demo_mode {
        // Add demo mode header
        let mut response = next.run(request).await;
        response.headers_mut().insert(
            "X-Demo-Mode",
            HeaderValue::from_static("true")
        );
        response
    } else {
        next.run(request).await
    }
}
```

### Demo Mode Features

1. **Mock Data Generation**
   ```rust
   if is_demo_mode(&config) {
       // Generate consistent mock data
       let mock_service = MockDataService::new(config.features.demo.seed);
       return Ok(mock_service.generate_users(100));
   }
   ```

2. **Automatic Reset**
   ```rust
   // Scheduled task for demo reset
   if config.features.demo_mode {
       tokio::spawn(async move {
           loop {
               tokio::time::sleep(Duration::from_secs(3600)).await;
               reset_demo_data().await;
               info!("Demo data reset completed");
           }
       });
   }
   ```

3. **Safe Mutations**
   ```rust
   // Prevent real database changes
   if is_demo_mode(&config) {
       // Use in-memory database
       return Ok(InMemoryDatabase::new());
   }
   ```

## Maintenance Mode

### Overview

Maintenance mode allows you to temporarily disable the API for updates while providing informative responses.

### Configuration

```toml
[features]
maintenance_mode = true

[features.maintenance]
message = "System maintenance in progress"
estimated_time = "2024-01-01T12:00:00Z"
allowed_ips = ["10.0.0.0/8", "192.168.1.0/24"]
bypass_token = ""  # Set via environment variable
```

### Implementation

```rust
pub struct MaintenanceMiddleware {
    config: MaintenanceConfig,
}

impl MaintenanceMiddleware {
    pub async fn check(
        &self,
        headers: &HeaderMap,
        client_ip: IpAddr,
    ) -> Option<Response> {
        if !self.config.enabled {
            return None;
        }

        // Check bypass token
        if let Some(token) = headers.get("X-Maintenance-Bypass") {
            if token == self.config.bypass_token {
                return None;
            }
        }

        // Check allowed IPs
        if self.is_ip_allowed(client_ip) {
            return None;
        }

        // Return maintenance response
        Some(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("Retry-After", "3600")
            .body(json!({
                "error": "maintenance_mode",
                "message": self.config.message,
                "estimated_time": self.config.estimated_time,
            }))
            .unwrap())
    }
}
```

### Maintenance Mode Response

```json
{
  "error": "maintenance_mode",
  "message": "System maintenance in progress",
  "estimated_time": "2024-01-01T12:00:00Z",
  "retry_after": 3600
}
```

### Health Check Override

```rust
// Health checks remain available during maintenance
app.route("/health", get(health_check))
   .layer(middleware::from_fn(maintenance_exempt));
```

## Read-Only Mode

### Overview

Read-only mode restricts all write operations while keeping read operations available.

### Configuration

```toml
[features]
read_only = true

[features.read_only]
message = "System is in read-only mode"
allowed_operations = ["Query", "Subscription"]
blocked_operations = ["Mutation"]
whitelist_mutations = ["login", "logout"]
```

### Implementation

```rust
pub struct ReadOnlyGuard;

#[async_trait]
impl Guard for ReadOnlyGuard {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let config = ctx.data::<AppConfig>()?;
        
        if !config.features.read_only {
            return Ok(());
        }

        // Check operation type
        let operation = ctx.operation_name();
        
        // Allow whitelisted mutations
        if config.features.read_only.whitelist_mutations.contains(operation) {
            return Ok(());
        }

        // Block all other mutations
        if ctx.is_mutation() {
            return Err(Error::new("System is in read-only mode"));
        }

        Ok(())
    }
}
```

### GraphQL Integration

```rust
// Apply read-only guard to schema
let schema = Schema::build(Query, Mutation, Subscription)
    .extension(ReadOnlyExtension)
    .finish();

// Read-only response
{
  "errors": [{
    "message": "System is in read-only mode",
    "extensions": {
      "code": "READ_ONLY_MODE"
    }
  }]
}
```

## Experimental Features

### Overview

Experimental features allow testing new functionality with limited exposure.

### Configuration

```toml
[features]
experimental = true

[features.experimental]
enabled_features = [
    "new_search_algorithm",
    "advanced_caching",
    "beta_ui_endpoint"
]
require_header = "X-Enable-Experimental"
log_usage = true
```

### Feature Registry

```rust
pub struct ExperimentalFeatures {
    features: HashMap<String, FeatureConfig>,
}

pub struct FeatureConfig {
    pub name: String,
    pub enabled: bool,
    pub percentage: f32,  // Gradual rollout
    pub user_whitelist: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl ExperimentalFeatures {
    pub fn is_enabled(&self, feature: &str, user_id: Option<&str>) -> bool {
        let Some(config) = self.features.get(feature) else {
            return false;
        };

        // Check if globally enabled
        if !config.enabled {
            return false;
        }

        // Check user whitelist
        if let Some(uid) = user_id {
            if config.user_whitelist.contains(&uid.to_string()) {
                return true;
            }
        }

        // Check percentage rollout
        if let Some(uid) = user_id {
            let hash = calculate_hash(uid, feature);
            return (hash % 100) as f32 <= config.percentage;
        }

        false
    }
}
```

### Usage in Code

```rust
// Check experimental feature
if ctx.experimental_features.is_enabled("new_search_algorithm", user_id) {
    // Use new algorithm
    return new_search_algorithm(query).await;
} else {
    // Use stable algorithm
    return stable_search_algorithm(query).await;
}
```

## Dynamic Feature Flags

### Runtime Configuration

For features that need to change without restart:

```rust
pub struct DynamicFeatureStore {
    features: Arc<RwLock<HashMap<String, FeatureFlag>>>,
}

impl DynamicFeatureStore {
    pub async fn reload(&self) -> Result<()> {
        // Load from database or external service
        let features = load_features_from_db().await?;
        
        let mut store = self.features.write().await;
        *store = features;
        
        info!("Reloaded {} feature flags", store.len());
        Ok(())
    }

    pub async fn check(&self, feature: &str, context: &Context) -> bool {
        let store = self.features.read().await;
        
        if let Some(flag) = store.get(feature) {
            flag.evaluate(context).await
        } else {
            false
        }
    }
}
```

### Feature Flag Evaluation

```rust
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
    pub rules: Vec<Rule>,
}

pub enum Rule {
    Percentage(f32),
    UserAttribute { key: String, value: String },
    TimeWindow { start: DateTime<Utc>, end: DateTime<Utc> },
    Custom(Box<dyn Fn(&Context) -> bool + Send + Sync>),
}

impl FeatureFlag {
    pub async fn evaluate(&self, context: &Context) -> bool {
        if !self.enabled {
            return false;
        }

        for rule in &self.rules {
            if !rule.matches(context).await {
                return false;
            }
        }

        true
    }
}
```

## A/B Testing

### Configuration

```toml
[features.ab_tests]
enabled = true

[[features.ab_tests.experiments]]
name = "new_homepage"
variants = ["control", "variant_a", "variant_b"]
traffic_allocation = [50, 25, 25]  # Percentage for each variant
metrics = ["conversion_rate", "engagement_time"]
```

### Implementation

```rust
pub struct ABTestManager {
    experiments: HashMap<String, Experiment>,
}

impl ABTestManager {
    pub fn get_variant(&self, experiment: &str, user_id: &str) -> Option<String> {
        let exp = self.experiments.get(experiment)?;
        
        // Consistent hashing for user assignment
        let hash = calculate_hash(user_id, experiment);
        let bucket = hash % 100;
        
        let mut cumulative = 0;
        for (i, allocation) in exp.traffic_allocation.iter().enumerate() {
            cumulative += allocation;
            if bucket < cumulative {
                return Some(exp.variants[i].clone());
            }
        }
        
        None
    }
}

// Usage in GraphQL resolver
pub async fn homepage_data(ctx: &Context<'_>) -> Result<HomePageData> {
    let variant = ctx.ab_tests.get_variant("new_homepage", &ctx.user_id);
    
    match variant.as_deref() {
        Some("variant_a") => Ok(HomePageData::variant_a()),
        Some("variant_b") => Ok(HomePageData::variant_b()),
        _ => Ok(HomePageData::control()),
    }
}
```

## Monitoring Feature Flags

### Metrics Collection

```rust
pub struct FeatureFlagMetrics {
    evaluations: Counter,
    enabled_count: Gauge,
    evaluation_time: Histogram,
}

impl FeatureFlagMetrics {
    pub fn record_evaluation(&self, feature: &str, enabled: bool, duration: Duration) {
        self.evaluations
            .with_label_values(&[feature, &enabled.to_string()])
            .inc();
        
        self.evaluation_time
            .with_label_values(&[feature])
            .observe(duration.as_secs_f64());
    }
}
```

### Audit Logging

```rust
#[derive(Serialize)]
pub struct FeatureFlagAudit {
    timestamp: DateTime<Utc>,
    feature: String,
    user_id: Option<String>,
    enabled: bool,
    reason: String,
    context: HashMap<String, String>,
}

pub async fn audit_feature_check(
    feature: &str,
    user_id: Option<&str>,
    enabled: bool,
    reason: &str,
) {
    let audit = FeatureFlagAudit {
        timestamp: Utc::now(),
        feature: feature.to_string(),
        user_id: user_id.map(String::from),
        enabled,
        reason: reason.to_string(),
        context: collect_context(),
    };
    
    // Log to audit system
    info!(target: "audit", "{}", serde_json::to_string(&audit).unwrap());
}
```

## Best Practices

### 1. Safe Defaults

```toml
# Always default to safe values
[features]
demo_mode = false         # Safe: disabled
maintenance_mode = false  # Safe: normal operation
read_only = false        # Safe: full access
experimental = false     # Safe: stable features only
```

### 2. Environment Validation

```rust
pub fn validate_features(config: &AppConfig) -> Result<()> {
    // Production safety checks
    if config.environment == "production" {
        if config.features.demo_mode {
            return Err(anyhow!("Demo mode cannot be enabled in production"));
        }
        
        if config.features.experimental {
            warn!("Experimental features enabled in production");
        }
    }
    
    Ok(())
}
```

### 3. Feature Flag Hygiene

```rust
// Regular cleanup of old flags
pub async fn cleanup_old_flags(store: &FeatureFlagStore) {
    let cutoff = Utc::now() - Duration::days(90);
    
    store.remove_flags_older_than(cutoff).await;
    
    info!("Cleaned up expired feature flags");
}
```

### 4. Gradual Rollout

```toml
# Start with small percentage
[[features.experiments]]
name = "new_feature"
percentage = 1.0  # 1% of users

# Gradually increase
[[features.experiments]]
name = "new_feature"
percentage = 10.0  # 10% of users

# Full rollout
[[features.experiments]]
name = "new_feature"
percentage = 100.0  # All users
```

## Testing with Feature Flags

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_feature_flag_evaluation() {
        let mut config = AppConfig::default();
        config.features.experimental = true;
        
        let features = ExperimentalFeatures::new(&config);
        features.add_feature("test_feature", 50.0);  // 50% rollout
        
        // Test consistent assignment
        let user1_enabled = features.is_enabled("test_feature", Some("user1"));
        assert_eq!(
            user1_enabled,
            features.is_enabled("test_feature", Some("user1"))
        );
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_maintenance_mode() {
    let mut config = test_config();
    config.features.maintenance_mode = true;
    
    let app = create_app(config);
    
    let response = app
        .oneshot(Request::builder()
            .uri("/graphql")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

## Troubleshooting

### Common Issues

1. **Feature Not Working**
   ```bash
   # Check configuration
   ./pcf-api --print-config | grep features
   
   # Verify environment variables
   env | grep PCF_API__FEATURES
   ```

2. **Inconsistent Behavior**
   ```rust
   // Add debug logging
   debug!(
       "Feature '{}' evaluated to {} for user {}",
       feature_name,
       enabled,
       user_id
   );
   ```

3. **Performance Impact**
   ```rust
   // Cache feature evaluations
   let cached_result = cache.get(&(feature, user_id)).await;
   if let Some(result) = cached_result {
       return result;
   }
   ```

## Summary

Feature flags in the PCF API provide:
1. **Safe deployments** - Test features gradually
2. **Quick rollbacks** - Disable features without deployment
3. **A/B testing** - Measure feature impact
4. **Emergency controls** - Maintenance and read-only modes
5. **Innovation** - Experimental features for early adopters
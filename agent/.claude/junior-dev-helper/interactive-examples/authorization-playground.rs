/// Interactive Authorization Playground
/// 
/// This example demonstrates authorization patterns with SpiceDB integration,
/// caching, circuit breakers, and fallback rules.
/// 
/// Run with: cargo run --example authorization-playground --features demo

use async_graphql::{Context, Error, Object, Result, Schema, SimpleObject, ID};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
    Json,
};
use serde::{Deserialize, Serialize};

/// Authorization Context
#[derive(Debug, Clone)]
struct AuthContext {
    user_id: Option<String>,
    trace_id: String,
}

impl AuthContext {
    fn require_auth(&self) -> Result<&str, Error> {
        self.user_id.as_deref()
            .ok_or_else(|| Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHORIZED")))
    }
}

/// Simple Auth Cache
struct AuthCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: CacheStats,
}

#[derive(Clone)]
struct CacheEntry {
    allowed: bool,
    expires_at: Instant,
}

#[derive(Default)]
struct CacheStats {
    hits: Arc<RwLock<u64>>,
    misses: Arc<RwLock<u64>>,
}

impl AuthCache {
    fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            stats: CacheStats::default(),
        }
    }
    
    async fn get(&self, key: &str) -> Option<bool> {
        let entries = self.entries.read().await;
        
        if let Some(entry) = entries.get(key) {
            if entry.expires_at > Instant::now() {
                *self.stats.hits.write().await += 1;
                return Some(entry.allowed);
            }
        }
        
        *self.stats.misses.write().await += 1;
        None
    }
    
    async fn set(&self, key: String, allowed: bool, ttl: Duration) {
        // CRITICAL: Only cache positive results
        if !allowed {
            println!("🚨 Refusing to cache negative result for {}", key);
            return;
        }
        
        let mut entries = self.entries.write().await;
        entries.insert(key.clone(), CacheEntry {
            allowed,
            expires_at: Instant::now() + ttl,
        });
        println!("✅ Cached positive result for {}", key);
    }
    
    async fn hit_rate(&self) -> f64 {
        let hits = *self.stats.hits.read().await as f64;
        let misses = *self.stats.misses.read().await as f64;
        
        if hits + misses > 0.0 {
            hits / (hits + misses)
        } else {
            0.0
        }
    }
}

/// Circuit Breaker States
#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed,
    Open(Instant),
    HalfOpen,
}

/// Simple Circuit Breaker
struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    config: CircuitConfig,
}

struct CircuitConfig {
    failure_threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            config: CircuitConfig {
                failure_threshold: 3,
                timeout: Duration::from_secs(30),
            },
        }
    }
    
    async fn is_open(&self) -> bool {
        matches!(*self.state.read().await, CircuitState::Open(_))
    }
    
    async fn call<F, T>(&self, operation: F) -> Result<T, String>
    where
        F: Fn() -> futures::future::BoxFuture<'static, Result<T, String>>,
    {
        // Check state
        let mut state = self.state.write().await;
        match &*state {
            CircuitState::Open(opened_at) => {
                if opened_at.elapsed() > self.config.timeout {
                    *state = CircuitState::HalfOpen;
                    println!("🔄 Circuit breaker entering half-open state");
                } else {
                    println!("⛔ Circuit breaker is OPEN - failing fast");
                    return Err("Circuit breaker open".to_string());
                }
            }
            _ => {}
        }
        drop(state);
        
        // Execute operation
        match operation().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(e)
            }
        }
    }
    
    async fn record_success(&self) {
        *self.failure_count.write().await = 0;
        
        if matches!(*self.state.read().await, CircuitState::HalfOpen) {
            *self.state.write().await = CircuitState::Closed;
            println!("✅ Circuit breaker CLOSED - service recovered");
        }
    }
    
    async fn record_failure(&self) {
        let mut failures = self.failure_count.write().await;
        *failures += 1;
        
        if *failures >= self.config.failure_threshold {
            *self.state.write().await = CircuitState::Open(Instant::now());
            println!("🚨 Circuit breaker OPENED after {} failures", *failures);
        }
    }
}

/// Mock SpiceDB Client
struct MockSpiceDB {
    permissions: Arc<RwLock<HashMap<String, bool>>>,
    should_fail: Arc<RwLock<bool>>,
    delay: Duration,
}

impl MockSpiceDB {
    fn new() -> Self {
        Self {
            permissions: Arc::new(RwLock::new(HashMap::new())),
            should_fail: Arc::new(RwLock::new(false)),
            delay: Duration::from_millis(50), // Simulate network delay
        }
    }
    
    async fn set_permission(&self, subject: &str, resource: &str, permission: &str, allowed: bool) {
        let key = format!("{}:{}:{}", subject, resource, permission);
        self.permissions.write().await.insert(key, allowed);
    }
    
    async fn check_permission(&self, subject: &str, resource: &str, permission: &str) -> Result<bool, String> {
        // Simulate network delay
        tokio::time::sleep(self.delay).await;
        
        if *self.should_fail.read().await {
            return Err("SpiceDB is down".to_string());
        }
        
        let key = format!("{}:{}:{}", subject, resource, permission);
        Ok(self.permissions.read().await.get(&key).copied().unwrap_or(false))
    }
    
    async fn simulate_outage(&self, outage: bool) {
        *self.should_fail.write().await = outage;
    }
}

/// Fallback authorization rules
fn apply_fallback_rules(user_id: &str, resource: &str, action: &str) -> bool {
    println!("⚠️  Using fallback rules for {}:{}:{}", user_id, resource, action);
    
    let (resource_type, resource_id) = resource.split_once(':').unwrap_or(("", ""));
    
    match (resource_type, action) {
        // Always allow health checks
        ("health", "read") => true,
        
        // Users can read their own profile
        ("user", "read") if resource_id == user_id => true,
        
        // Demo mode - allow playground access
        ("playground", "read") => true,
        
        // Deny everything else (conservative)
        _ => false,
    }
}

/// The main authorization function
async fn is_authorized(
    ctx: &Context<'_>,
    resource: &str,
    action: &str,
) -> Result<(), Error> {
    println!("\n🔐 Checking authorization for resource: {}, action: {}", resource, action);
    
    // 1. Check authentication
    let auth = ctx.data::<AuthContext>()?;
    let user_id = auth.require_auth()?;
    println!("👤 User authenticated: {}", user_id);
    
    // 2. Check cache
    let cache = ctx.data::<Arc<AuthCache>>()?;
    let cache_key = format!("{}:{}:{}", user_id, resource, action);
    
    if let Some(allowed) = cache.get(&cache_key).await {
        println!("💾 Cache HIT! Result: {}", allowed);
        return if allowed {
            Ok(())
        } else {
            Err(Error::new("Permission denied").extend_with(|_, e| e.set("code", "FORBIDDEN")))
        };
    }
    
    println!("💾 Cache MISS - checking with SpiceDB");
    
    // 3. Check with SpiceDB through circuit breaker
    let spicedb = ctx.data::<Arc<MockSpiceDB>>()?;
    let circuit_breaker = ctx.data::<Arc<CircuitBreaker>>()?;
    
    let allowed = match circuit_breaker.call(|| {
        let spicedb = spicedb.clone();
        let subject = format!("user:{}", user_id);
        let resource = resource.to_string();
        let permission = action.to_string();
        
        Box::pin(async move {
            spicedb.check_permission(&subject, &resource, &permission).await
        })
    }).await {
        Ok(result) => {
            println!("✅ SpiceDB returned: {}", result);
            result
        }
        Err(_) => {
            println!("❌ SpiceDB failed - using fallback rules");
            apply_fallback_rules(user_id, resource, action)
        }
    };
    
    // 4. Cache positive results
    if allowed {
        let ttl = if circuit_breaker.is_open().await {
            Duration::from_secs(1800) // 30 minutes during outage
        } else {
            Duration::from_secs(300)  // 5 minutes normally
        };
        
        cache.set(cache_key, allowed, ttl).await;
    }
    
    // 5. Return result
    if allowed {
        println!("✅ Authorization GRANTED");
        Ok(())
    } else {
        println!("❌ Authorization DENIED");
        Err(Error::new("Permission denied").extend_with(|_, e| e.set("code", "FORBIDDEN")))
    }
}

/// GraphQL Schema
#[derive(SimpleObject)]
struct AuthStatus {
    authenticated: bool,
    user_id: Option<String>,
    cache_hit_rate: f64,
    circuit_breaker_open: bool,
}

#[derive(SimpleObject)]
struct PermissionCheck {
    resource: String,
    action: String,
    allowed: bool,
    source: String,
}

struct Query;

#[Object]
impl Query {
    /// Check current auth status
    async fn auth_status(&self, ctx: &Context<'_>) -> Result<AuthStatus> {
        let auth = ctx.data::<AuthContext>()?;
        let cache = ctx.data::<Arc<AuthCache>>()?;
        let circuit = ctx.data::<Arc<CircuitBreaker>>()?;
        
        Ok(AuthStatus {
            authenticated: auth.user_id.is_some(),
            user_id: auth.user_id.clone(),
            cache_hit_rate: cache.hit_rate().await,
            circuit_breaker_open: circuit.is_open().await,
        })
    }
    
    /// Test authorization for a resource
    async fn check_permission(
        &self,
        ctx: &Context<'_>,
        resource: String,
        action: String,
    ) -> Result<PermissionCheck> {
        let allowed = is_authorized(ctx, &resource, &action).await.is_ok();
        
        // Determine source
        let cache = ctx.data::<Arc<AuthCache>>()?;
        let cache_key = format!("{}:{}:{}", 
            ctx.data::<AuthContext>()?.user_id.as_ref().unwrap_or(&"anon".to_string()),
            resource,
            action
        );
        
        let source = if cache.get(&cache_key).await.is_some() {
            "cache"
        } else if ctx.data::<Arc<CircuitBreaker>>()?.is_open().await {
            "fallback"
        } else {
            "spicedb"
        };
        
        Ok(PermissionCheck {
            resource,
            action,
            allowed,
            source: source.to_string(),
        })
    }
    
    /// Protected query - requires authorization
    async fn protected_data(&self, ctx: &Context<'_>, id: ID) -> Result<String> {
        is_authorized(ctx, &format!("data:{}", id), "read").await?;
        Ok(format!("Secret data for {}", id))
    }
}

struct Mutation;

#[Object]
impl Mutation {
    /// Simulate SpiceDB outage
    async fn simulate_outage(&self, ctx: &Context<'_>, enable: bool) -> Result<String> {
        let spicedb = ctx.data::<Arc<MockSpiceDB>>()?;
        spicedb.simulate_outage(enable).await;
        
        Ok(if enable {
            "SpiceDB outage simulated - circuit breaker will activate after failures"
        } else {
            "SpiceDB restored - circuit breaker will recover after successes"
        }.to_string())
    }
    
    /// Set a permission in mock SpiceDB
    async fn set_permission(
        &self,
        ctx: &Context<'_>,
        subject: String,
        resource: String,
        permission: String,
        allowed: bool,
    ) -> Result<String> {
        let spicedb = ctx.data::<Arc<MockSpiceDB>>()?;
        spicedb.set_permission(&subject, &resource, &permission, allowed).await;
        
        Ok(format!("Permission set: {} {} {} = {}", subject, resource, permission, allowed))
    }
}

/// API Endpoints
async fn graphql_handler(
    Extension(schema): Extension<Schema<Query, Mutation, async_graphql::EmptySubscription>>,
    req: async_graphql_axum::GraphQLRequest,
) -> async_graphql_axum::GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// Status endpoint
#[derive(Serialize)]
struct Status {
    cache_stats: CacheStatsResponse,
    circuit_breaker: CircuitBreakerStatus,
}

#[derive(Serialize)]
struct CacheStatsResponse {
    hit_rate: f64,
    hits: u64,
    misses: u64,
}

#[derive(Serialize)]
struct CircuitBreakerStatus {
    state: String,
    failure_count: u32,
}

async fn status_handler(
    Extension(cache): Extension<Arc<AuthCache>>,
    Extension(circuit): Extension<Arc<CircuitBreaker>>,
) -> Json<Status> {
    let state = match &*circuit.state.read().await {
        CircuitState::Closed => "closed",
        CircuitState::Open(_) => "open",
        CircuitState::HalfOpen => "half-open",
    };
    
    Json(Status {
        cache_stats: CacheStatsResponse {
            hit_rate: cache.hit_rate().await,
            hits: *cache.stats.hits.read().await,
            misses: *cache.stats.misses.read().await,
        },
        circuit_breaker: CircuitBreakerStatus {
            state: state.to_string(),
            failure_count: *circuit.failure_count.read().await,
        },
    })
}

/// Interactive UI
async fn playground() -> impl IntoResponse {
    Html(include_str!("authorization-playground.html"))
}

/// Initialize demo data
async fn setup_demo_permissions(spicedb: &MockSpiceDB) {
    // Alice owns her notes
    spicedb.set_permission("user:alice", "note:1", "read", true).await;
    spicedb.set_permission("user:alice", "note:1", "write", true).await;
    spicedb.set_permission("user:alice", "note:1", "delete", true).await;
    
    // Bob can read note 1
    spicedb.set_permission("user:bob", "note:1", "read", true).await;
    
    // Alice owns her profile
    spicedb.set_permission("user:alice", "user:alice", "read", true).await;
    spicedb.set_permission("user:alice", "user:alice", "write", true).await;
    
    // Everyone can read health
    spicedb.set_permission("user:alice", "health:status", "read", true).await;
    spicedb.set_permission("user:bob", "health:status", "read", true).await;
}

#[tokio::main]
async fn main() {
    // Initialize
    let cache = Arc::new(AuthCache::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new());
    let spicedb = Arc::new(MockSpiceDB::new());
    
    setup_demo_permissions(&spicedb).await;
    
    // Create GraphQL schema
    let schema = Schema::build(Query, Mutation, async_graphql::EmptySubscription)
        .data(AuthContext {
            user_id: Some("alice".to_string()), // Demo user
            trace_id: "demo-trace".to_string(),
        })
        .data(cache.clone())
        .data(circuit_breaker.clone())
        .data(spicedb)
        .finish();
    
    // Create router
    let app = Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/playground", get(playground))
        .route("/status", get(status_handler))
        .route("/", get(|| async { "Authorization Playground - Navigate to /playground" }))
        .layer(Extension(schema))
        .layer(Extension(cache))
        .layer(Extension(circuit_breaker));
    
    println!("🚀 Authorization Playground running at http://localhost:8080/playground");
    println!("📊 Status dashboard at http://localhost:8080/status");
    println!();
    println!("Features to try:");
    println!("- Permission checks with caching");
    println!("- Circuit breaker activation");
    println!("- Fallback rules during outage");
    println!("- Cache hit rate monitoring");
    
    // Start server
    axum::Server::bind(&"127.0.0.1:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
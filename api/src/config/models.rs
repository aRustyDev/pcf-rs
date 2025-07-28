use garde::Validate;
use serde::{Deserialize, Serialize};

/// Application configuration with 4-tier hierarchy support
/// 
/// Configuration is loaded in this order (lowest to highest priority):
/// 1. Embedded defaults
/// 2. Default config file (config/default.toml)
/// 3. Environment-specific config file (config/{env}.toml)
/// 4. Environment variables (APP_ prefix)
/// 5. CLI arguments
#[derive(Debug, Deserialize, Serialize, Validate, Default)]
pub struct AppConfig {
    /// Server configuration including port and bind address
    #[garde(dive)]
    #[serde(default)]
    pub server: ServerConfig,
    
    /// Logging configuration for format and level settings
    #[garde(dive)]
    #[serde(default)]
    pub logging: LoggingConfig,
    
    /// Health check endpoint configuration
    #[garde(dive)]
    #[serde(default)]
    pub health: HealthConfig,
    
    /// Deployment environment (Development, Staging, Production)
    #[garde(skip)]
    #[serde(default)]
    pub environment: Environment,
    
    /// Demo mode configuration (WARNING: Never enable in production!)
    #[garde(skip)]
    #[serde(default)]
    pub demo: super::demo::DemoConfig,
    
    /// Authorization system configuration
    #[garde(dive)]
    #[serde(default)]
    pub authorization: AuthorizationConfig,
}

/// Server configuration for network binding and timeouts
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct ServerConfig {
    /// Port to bind the HTTP server to (must be between 1024-65535)
    #[garde(range(min = 1024, max = 65535))]
    pub port: u16,
    
    /// IP address to bind to (e.g., "0.0.0.0" for all interfaces)
    #[garde(length(min = 1), custom(validate_bind_address))]
    #[serde(default = "default_bind")]
    pub bind: String,
    
    /// Graceful shutdown timeout in seconds (1-300 seconds)
    #[garde(range(min = 1, max = 300))]
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u64,
}

fn default_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_shutdown_timeout() -> u64 {
    30
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            bind: "0.0.0.0".to_string(),
            shutdown_timeout: 30,
        }
    }
}

/// Logging configuration for output format and level
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    #[garde(length(min = 1))]
    #[serde(default = "default_log_level")]
    pub level: String,
    
    /// Log format: "json" for production, "pretty" for development
    #[garde(pattern(r"^(json|pretty)$"))]
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
        }
    }
}

/// Health check endpoint configuration
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct HealthConfig {
    /// Path for liveness probe (e.g., "/health")
    #[garde(length(min = 1))]
    #[serde(default = "default_liveness_path")]
    pub liveness_path: String,
    
    /// Path for readiness probe (e.g., "/health/ready")
    #[garde(length(min = 1))]
    #[serde(default = "default_readiness_path")]
    pub readiness_path: String,
    
    /// Startup timeout in seconds before health checks start (1-3600 seconds)
    #[garde(range(min = 1, max = 3600))]
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_seconds: u32,
}

fn default_liveness_path() -> String {
    "/health".to_string()
}

fn default_readiness_path() -> String {
    "/health/ready".to_string()
}

fn default_startup_timeout() -> u32 {
    300
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            liveness_path: "/health".to_string(),
            readiness_path: "/health/ready".to_string(),
            startup_timeout_seconds: 300,
        }
    }
}

/// Deployment environment affecting configuration defaults
#[derive(Debug, Deserialize, Serialize, Default)]
pub enum Environment {
    /// Development environment with relaxed security and verbose logging
    Development,
    /// Staging environment mirroring production with additional debugging
    Staging,
    /// Production environment with security hardening and minimal logging
    #[default]
    Production,
}

/// Authorization system configuration
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct AuthorizationConfig {
    /// SpiceDB endpoint URL
    #[garde(length(min = 1))]
    #[serde(default = "default_spicedb_endpoint")]
    pub spicedb_endpoint: String,
    
    /// SpiceDB preshared key for authentication
    #[garde(length(min = 1))]
    #[serde(default = "default_spicedb_preshared_key")]
    pub spicedb_preshared_key: String,
    
    /// Maximum entries in authorization cache
    #[garde(range(min = 100, max = 100000))]
    #[serde(default = "default_cache_max_entries")]
    pub cache_max_entries: usize,
    
    /// Cache TTL in seconds
    #[garde(range(min = 60, max = 3600))]
    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,
    
    /// Circuit breaker failure threshold
    #[garde(range(min = 1, max = 20))]
    #[serde(default = "default_circuit_breaker_failure_threshold")]
    pub circuit_breaker_failure_threshold: u32,
    
    /// Circuit breaker timeout in milliseconds
    #[garde(range(min = 100, max = 10000))]
    #[serde(default = "default_circuit_breaker_timeout_ms")]
    pub circuit_breaker_timeout_ms: u64,
    
    /// Circuit breaker retry timeout in seconds
    #[garde(range(min = 10, max = 300))]
    #[serde(default = "default_circuit_breaker_retry_timeout_seconds")]
    pub circuit_breaker_retry_timeout_seconds: u64,
}

impl Default for AuthorizationConfig {
    fn default() -> Self {
        Self {
            spicedb_endpoint: default_spicedb_endpoint(),
            spicedb_preshared_key: default_spicedb_preshared_key(),
            cache_max_entries: default_cache_max_entries(),
            cache_ttl_seconds: default_cache_ttl_seconds(),
            circuit_breaker_failure_threshold: default_circuit_breaker_failure_threshold(),
            circuit_breaker_timeout_ms: default_circuit_breaker_timeout_ms(),
            circuit_breaker_retry_timeout_seconds: default_circuit_breaker_retry_timeout_seconds(),
        }
    }
}

/// Custom validator for IP addresses (both IPv4 and IPv6)
fn validate_bind_address(value: &str, _: &()) -> garde::Result {
    value.parse::<std::net::IpAddr>()
        .map(|_| ())
        .map_err(|_| garde::Error::new("Invalid IP address"))
}

// Authorization configuration defaults
fn default_spicedb_endpoint() -> String {
    std::env::var("SPICEDB_ENDPOINT").unwrap_or_else(|_| "http://localhost:50051".to_string())
}

fn default_spicedb_preshared_key() -> String {
    std::env::var("SPICEDB_PRESHARED_KEY").unwrap_or_else(|_| "dev_key_12345".to_string())
}

fn default_cache_max_entries() -> usize {
    std::env::var("AUTH_CACHE_MAX_ENTRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
}

fn default_cache_ttl_seconds() -> u64 {
    std::env::var("AUTH_CACHE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300) // 5 minutes
}

fn default_circuit_breaker_failure_threshold() -> u32 {
    std::env::var("AUTH_CIRCUIT_BREAKER_FAILURE_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
}

fn default_circuit_breaker_timeout_ms() -> u64 {
    std::env::var("AUTH_CIRCUIT_BREAKER_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000) // 1 second
}

fn default_circuit_breaker_retry_timeout_seconds() -> u64 {
    std::env::var("AUTH_CIRCUIT_BREAKER_RETRY_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60) // 1 minute
}
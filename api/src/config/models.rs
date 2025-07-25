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

/// Custom validator for IP addresses (both IPv4 and IPv6)
fn validate_bind_address(value: &str, _: &()) -> garde::Result {
    value.parse::<std::net::IpAddr>()
        .map(|_| ())
        .map_err(|_| garde::Error::new("Invalid IP address"))
}
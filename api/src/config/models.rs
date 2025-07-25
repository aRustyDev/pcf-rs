use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Validate, Default)]
pub struct AppConfig {
    #[garde(dive)]
    #[serde(default)]
    pub server: ServerConfig,
    
    #[garde(dive)]
    #[serde(default)]
    pub logging: LoggingConfig,
    
    #[garde(dive)]
    #[serde(default)]
    pub health: HealthConfig,
    
    #[garde(skip)]
    #[serde(default)]
    pub environment: Environment,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct ServerConfig {
    #[garde(range(min = 1024, max = 65535))]
    pub port: u16,
    
    #[garde(length(min = 1), custom(validate_bind_address))]
    #[serde(default = "default_bind")]
    pub bind: String,
    
    #[garde(range(min = 1, max = 300))]
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u64, // seconds
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

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct LoggingConfig {
    #[garde(length(min = 1))]
    #[serde(default = "default_log_level")]
    pub level: String,  // trace, debug, info, warn, error
    
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

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct HealthConfig {
    #[garde(length(min = 1))]
    #[serde(default = "default_liveness_path")]
    pub liveness_path: String,
    
    #[garde(length(min = 1))]
    #[serde(default = "default_readiness_path")]
    pub readiness_path: String,
    
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

#[derive(Debug, Deserialize, Serialize, Default)]
pub enum Environment {
    Development,
    Staging,
    #[default]
    Production,
}

// Custom validator example from configuration.md
fn validate_bind_address(value: &str, _: &()) -> garde::Result {
    value.parse::<std::net::IpAddr>()
        .map(|_| ())
        .map_err(|_| garde::Error::new("Invalid IP address"))
}
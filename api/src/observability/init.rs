//! Observability initialization
//!
//! This module provides initialization functions for setting up observability
//! components including metrics, logging, and tracing at server startup.

use anyhow::Result;
use std::env;
use tracing;

use super::recorder::{init_metrics, MetricsConfig};
use super::logging::{init_logging, LoggingConfig};
use super::tracing::{init_tracing, TracingConfig};

/// Initialize observability components based on environment configuration
pub fn init_observability() -> Result<()> {
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    let is_production = environment == "production";
    
    // Initialize structured logging first (before any tracing calls)
    let logging_config = LoggingConfig {
        level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        json_format: is_production,
        enable_sanitization: env::var("LOG_SANITIZATION")
            .map(|v| v.to_lowercase() != "false")
            .unwrap_or(true), // Enable by default
        sanitization_rules: super::logging::default_sanitization_rules(),
    };
    
    init_logging(&logging_config)
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;
    
    // Initialize distributed tracing (after logging)
    let tracing_config = TracingConfig {
        otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:4317".to_string()),
        sample_rate: env::var("OTEL_SAMPLE_RATE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.1), // Default to 10% sampling
        service_name: "pcf-api".to_string(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        environment: environment.clone(),
        enabled: env::var("OTEL_TRACES_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true),
        export_timeout: std::time::Duration::from_secs(10),
    };
    
    init_tracing(&tracing_config)
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
    
    // Initialize metrics with configuration from environment
    let metrics_config = MetricsConfig {
        port: env::var("METRICS_PORT")
            .unwrap_or_else(|_| "9090".to_string())
            .parse()
            .unwrap_or(9090),
        environment: environment.clone(),
        max_operation_labels: env::var("METRICS_MAX_OPERATIONS")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .unwrap_or(50),
        ip_allowlist: parse_ip_allowlist(),
        detailed_metrics: env::var("METRICS_DETAILED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false),
    };

    // Initialize metrics manager
    init_metrics(metrics_config)?;
    
    tracing::info!(
        environment = %environment,
        json_logging = %is_production,
        sanitization = %logging_config.enable_sanitization,
        tracing_enabled = %tracing_config.enabled,
        otlp_endpoint = %tracing_config.otlp_endpoint,
        sample_rate = %tracing_config.sample_rate,
        "Observability components initialized successfully"
    );
    Ok(())
}

/// Parse IP allowlist from environment variable
fn parse_ip_allowlist() -> Option<Vec<String>> {
    parse_ip_allowlist_from_var("METRICS_IP_ALLOWLIST")
}

/// Parse IP allowlist from a specific environment variable (for testing)
fn parse_ip_allowlist_from_var(var_name: &str) -> Option<Vec<String>> {
    env::var(var_name)
        .ok()
        .and_then(|allowlist_str| {
            if allowlist_str.trim().is_empty() {
                None
            } else {
                Some(
                    allowlist_str
                        .split(',')
                        .map(|ip| ip.trim().to_string())
                        .filter(|ip| !ip.is_empty())
                        .collect()
                )
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_parse_ip_allowlist_empty() {
        unsafe {
            env::remove_var("TEST_ALLOWLIST_EMPTY");
        }
        assert_eq!(parse_ip_allowlist_from_var("TEST_ALLOWLIST_EMPTY"), None);
    }

    #[test]
    fn test_parse_ip_allowlist_single() {
        unsafe {
            env::set_var("TEST_ALLOWLIST_SINGLE", "127.0.0.1");
        }
        assert_eq!(
            parse_ip_allowlist_from_var("TEST_ALLOWLIST_SINGLE"), 
            Some(vec!["127.0.0.1".to_string()])
        );
    }

    #[test]
    fn test_parse_ip_allowlist_multiple() {
        unsafe {
            env::set_var("TEST_ALLOWLIST_MULTIPLE", "127.0.0.1,::1,10.0.0.1");
        }
        assert_eq!(
            parse_ip_allowlist_from_var("TEST_ALLOWLIST_MULTIPLE"), 
            Some(vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "10.0.0.1".to_string()
            ])
        );
    }

    #[test] 
    fn test_parse_ip_allowlist_with_spaces() {
        unsafe {
            env::set_var("TEST_ALLOWLIST_SPACES", " 127.0.0.1 ,  ::1  , 10.0.0.1 ");
        }
        assert_eq!(
            parse_ip_allowlist_from_var("TEST_ALLOWLIST_SPACES"),
            Some(vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "10.0.0.1".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_ip_allowlist_empty_string() {
        unsafe {
            env::set_var("TEST_ALLOWLIST_EMPTY_STR", "");
        }
        assert_eq!(parse_ip_allowlist_from_var("TEST_ALLOWLIST_EMPTY_STR"), None);
    }

    #[test]
    fn test_parse_ip_allowlist_whitespace_only() {
        unsafe {
            env::set_var("TEST_ALLOWLIST_WHITESPACE", "   ");
        }
        assert_eq!(parse_ip_allowlist_from_var("TEST_ALLOWLIST_WHITESPACE"), None);
    }
}
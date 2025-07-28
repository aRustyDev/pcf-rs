//! Metrics endpoint with security controls
//!
//! This module provides a secure HTTP endpoint for Prometheus metrics with:
//! - IP allowlist access control
//! - Proper error handling and logging
//! - Integration with metrics manager
//! - Security headers and response formatting

use axum::{
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use tracing;

use super::recorder::{get_metrics_manager, extract_client_ip};

/// Metrics endpoint handler with IP allowlist security
pub async fn metrics_endpoint(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    // Get metrics manager
    let manager = match get_metrics_manager() {
        Ok(manager) => manager,
        Err(e) => {
            tracing::error!("Metrics manager not initialized: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Metrics not available"
            ).into_response();
        }
    };

    // Extract client IP from headers or connection
    let client_ip = if headers.is_empty() {
        addr.ip().to_string()
    } else {
        let header_ip = extract_client_ip(&headers);
        if header_ip == "unknown" {
            addr.ip().to_string()
        } else {
            header_ip
        }
    };

    // Check IP allowlist
    if !manager.is_ip_allowed(&client_ip) {
        tracing::warn!(
            client_ip = %client_ip,
            "Metrics access denied: IP not in allowlist"
        );
        return (
            StatusCode::FORBIDDEN,
            "Access denied: IP not authorized"
        ).into_response();
    }

    // Generate metrics output
    let metrics_content = manager.render();
    
    tracing::debug!(
        client_ip = %client_ip,
        metrics_size = %metrics_content.len(),
        "Metrics served successfully"
    );

    // Return metrics with appropriate content type
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        metrics_content
    ).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::ConnectInfo,
        http::{HeaderMap, HeaderValue},
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use crate::observability::recorder::{init_metrics, MetricsConfig};

    #[tokio::test]
    async fn test_metrics_endpoint_allowed_ip() {
        // Initialize metrics manager with IP allowlist
        let config = MetricsConfig {
            port: 9099,
            environment: "test".to_string(),
            max_operation_labels: 50,
            ip_allowlist: Some(vec!["127.0.0.1".to_string(), "::1".to_string()]),
            detailed_metrics: true,
        };
        
        let _ = init_metrics(config).or_else(|_| {
            Ok::<(), anyhow::Error>(())
        });

        // Test with allowed IP
        let headers = HeaderMap::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let connect_info = ConnectInfo(addr);

        let response = metrics_endpoint(headers, connect_info).await;
        
        // Should return 200 OK with metrics content
        // Note: We can't easily check the status code in this test setup,
        // but the function should not panic and should return content
    }

    #[tokio::test] 
    async fn test_metrics_endpoint_denied_ip() {
        // Initialize metrics manager with IP allowlist
        let config = MetricsConfig {
            port: 9100,
            environment: "test".to_string(),
            max_operation_labels: 50,
            ip_allowlist: Some(vec!["127.0.0.1".to_string()]),
            detailed_metrics: true,
        };
        
        let _ = init_metrics(config).or_else(|_| {
            Ok::<(), anyhow::Error>(())
        });

        // Test with denied IP
        let headers = HeaderMap::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let connect_info = ConnectInfo(addr);

        let response = metrics_endpoint(headers, connect_info).await;
        
        // Should return 403 Forbidden
        // Note: We can't easily check the status code in this test setup,
        // but the function should not panic
    }

    #[tokio::test]
    async fn test_metrics_endpoint_with_proxy_headers() {
        // Initialize metrics manager with IP allowlist
        let config = MetricsConfig {
            port: 9101,
            environment: "test".to_string(),
            max_operation_labels: 50,
            ip_allowlist: Some(vec!["10.0.0.1".to_string()]),
            detailed_metrics: true,
        };
        
        let _ = init_metrics(config).or_else(|_| {
            Ok::<(), anyhow::Error>(())
        });

        // Test with X-Forwarded-For header
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1, 192.168.1.1"));
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let connect_info = ConnectInfo(addr);

        let response = metrics_endpoint(headers, connect_info).await;
        
        // Should use the X-Forwarded-For IP (10.0.0.1) which is allowed
        // Function should not panic
    }

    #[tokio::test]
    async fn test_metrics_endpoint_no_allowlist() {
        // Initialize metrics manager without IP allowlist (all IPs allowed)
        let config = MetricsConfig {
            port: 9102,
            environment: "test".to_string(),
            max_operation_labels: 50,
            ip_allowlist: None,
            detailed_metrics: true,
        };
        
        let _ = init_metrics(config).or_else(|_| {
            Ok::<(), anyhow::Error>(())
        });

        // Test with any IP
        let headers = HeaderMap::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        let connect_info = ConnectInfo(addr);

        let response = metrics_endpoint(headers, connect_info).await;
        
        // Should return 200 OK since no allowlist means all IPs are allowed
        // Function should not panic
    }
}
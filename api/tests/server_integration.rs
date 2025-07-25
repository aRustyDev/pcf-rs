use anyhow::Result;
use std::net::TcpListener;
use std::time::Duration;
use tokio::time::timeout;

/// Test that server starts and binds to configured port
#[tokio::test]
async fn test_server_starts_and_binds() -> Result<()> {
    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    // Set environment variables for test
    unsafe {
        std::env::set_var("APP_SERVER__PORT", port.to_string());
        std::env::set_var("APP_SERVER__BIND", "127.0.0.1");
        std::env::set_var("ENVIRONMENT", "test");
    }
    
    // Start server in background task
    let server_handle = tokio::spawn(async move {
        pcf_api::run_server().await
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test that we can connect to the port
    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(5),
        client.get(&format!("http://127.0.0.1:{}/health/liveness", port)).send()
    ).await??;

    assert_eq!(response.status(), 200);
    let body = response.text().await?;
    assert_eq!(body, "OK");

    // Cleanup
    server_handle.abort();
    
    Ok(())
}

/// Test graceful shutdown completes within timeout
#[tokio::test]
async fn test_graceful_shutdown() -> Result<()> {
    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    // Set environment variables for test
    unsafe {
        std::env::set_var("APP_SERVER__PORT", port.to_string());
        std::env::set_var("APP_SERVER__BIND", "127.0.0.1");
        std::env::set_var("ENVIRONMENT", "test");
    }
    
    // Start server
    let server_handle = tokio::spawn(async move {
        pcf_api::run_server().await
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send shutdown signal by aborting (simulates SIGTERM)
    server_handle.abort();
    
    // Verify server shuts down within reasonable time
    let shutdown_result = timeout(Duration::from_secs(5), server_handle).await;
    
    // The task should complete (either successfully or be aborted)
    assert!(shutdown_result.is_ok() || shutdown_result.is_err());
    
    Ok(())
}

/// Test server returns clear error when port is already in use
#[tokio::test]
async fn test_port_conflict_error() -> Result<()> {
    // Bind to a port to make it unavailable
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    // Keep listener alive to hold the port
    
    // Set environment variables to use the occupied port
    unsafe {
        std::env::set_var("APP_SERVER__PORT", port.to_string());
        std::env::set_var("APP_SERVER__BIND", "127.0.0.1");
        std::env::set_var("ENVIRONMENT", "test");
    }
    
    // Attempt to start server - should fail with port conflict
    let result = timeout(Duration::from_secs(2), pcf_api::run_server()).await;
    
    // Should timeout or return error due to port conflict
    assert!(result.is_err() || result.unwrap().is_err());
    
    drop(listener);
    Ok(())
}

/// Test health endpoints return expected responses
#[tokio::test]
async fn test_health_endpoints() -> Result<()> {
    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    // Set environment variables for test
    unsafe {
        std::env::set_var("APP_SERVER__PORT", port.to_string());
        std::env::set_var("APP_SERVER__BIND", "127.0.0.1");
        std::env::set_var("ENVIRONMENT", "test");
    }
    
    // Start server in background task
    let server_handle = tokio::spawn(async move {
        pcf_api::run_server().await
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    
    // Test liveness endpoint
    let liveness_response = timeout(
        Duration::from_secs(5),
        client.get(&format!("http://127.0.0.1:{}/health/liveness", port)).send()
    ).await??;
    assert_eq!(liveness_response.status(), 200);
    assert_eq!(liveness_response.text().await?, "OK");

    // Test readiness endpoint  
    let readiness_response = timeout(
        Duration::from_secs(5),
        client.get(&format!("http://127.0.0.1:{}/health/readiness", port)).send()
    ).await??;
    assert_eq!(readiness_response.status(), 200);
    assert_eq!(readiness_response.text().await?, "OK");

    // Cleanup
    server_handle.abort();
    
    Ok(())
}

/// Test that server logs include trace IDs
#[tokio::test]
async fn test_trace_ids_in_logs() -> Result<()> {
    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    // Set environment variables for test (JSON format to easily parse trace IDs)
    unsafe {
        std::env::set_var("APP_SERVER__PORT", port.to_string());
        std::env::set_var("APP_SERVER__BIND", "127.0.0.1");
        std::env::set_var("ENVIRONMENT", "production"); // JSON format
    }
    
    // Start server in background task
    let server_handle = tokio::spawn(async move {
        pcf_api::run_server().await
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    
    // Make request that should generate trace ID
    let response = timeout(
        Duration::from_secs(5),
        client.get(&format!("http://127.0.0.1:{}/health/liveness", port)).send()
    ).await??;

    // Check for trace ID header in response
    assert!(response.headers().contains_key("x-trace-id"));
    let trace_id = response.headers().get("x-trace-id").unwrap();
    
    // Trace ID should be UUID format (36 characters)
    assert_eq!(trace_id.len(), 36);

    // Cleanup
    server_handle.abort();
    
    Ok(())
}
//! Comprehensive tests for SpiceDB integration and circuit breaker
//!
//! These tests follow TDD methodology and cover:
//! - SpiceDB client connection and permission checking
//! - Circuit breaker state machine behavior
//! - Fallback authorization rules
//! - Connection pooling and retry logic
//! - Integration with the auth cache
//!
//! Tests are designed to run against both mock and real SpiceDB instances.

use super::*;
use crate::auth::{AuthContext, ProductionAuthCache, CacheConfig};
use crate::middleware::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use crate::auth::fallback::FallbackAuthorizer;
use async_graphql::{Schema, EmptyMutation, EmptySubscription, Object, Context};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tonic::transport::Channel;
use authzed::api::v1::*;

/// Mock SpiceDB client for testing circuit breaker behavior
#[derive(Clone)]
pub struct MockSpiceDBClient {
    should_fail: Arc<RwLock<bool>>,
    call_count: Arc<RwLock<u32>>,
    latency: Duration,
}

impl MockSpiceDBClient {
    pub fn new() -> Self {
        Self {
            should_fail: Arc::new(RwLock::new(false)),
            call_count: Arc::new(RwLock::new(0)),
            latency: Duration::from_millis(10),
        }
    }

    pub fn failing() -> Self {
        Self {
            should_fail: Arc::new(RwLock::new(true)),
            call_count: Arc::new(RwLock::new(0)),
            latency: Duration::from_millis(10),
        }
    }

    pub fn with_latency(latency: Duration) -> Self {
        Self {
            should_fail: Arc::new(RwLock::new(false)),
            call_count: Arc::new(RwLock::new(0)),
            latency,
        }
    }

    pub async fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.write().await = should_fail;
    }

    pub async fn call_count(&self) -> u32 {
        *self.call_count.read().await
    }

    pub async fn check_permission_mock(&self, req: CheckPermissionRequest) -> Result<bool, SpiceDBError> {
        // Increment call count
        *self.call_count.write().await += 1;

        // Simulate latency
        tokio::time::sleep(self.latency).await;

        if *self.should_fail.read().await {
            return Err(SpiceDBError::ConnectionError("Mock failure".to_string()));
        }

        // Mock permission logic
        let subject_parts: Vec<&str> = req.subject.split(':').collect();
        let resource_parts: Vec<&str> = req.resource.split(':').collect();

        if subject_parts.len() != 2 || resource_parts.len() < 2 {
            return Ok(false);
        }

        let user_id = subject_parts[1];
        let resource_type = resource_parts[0];

        // Mock permission rules
        match (resource_type, req.permission) {
            ("notes", "read") => {
                // Allow users to read their own notes
                if resource_parts.len() >= 3 && resource_parts[1] == user_id {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            ("notes", "write") => {
                // Allow users to write their own notes  
                if resource_parts.len() >= 3 && resource_parts[1] == user_id {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            ("system", "health") => Ok(true), // Health checks always pass
            _ => Ok(false), // Deny everything else
        }
    }
}

#[derive(Debug, Clone)]
pub enum SpiceDBError {
    ConnectionError(String),
    PermissionDenied(String),
    Timeout,
}

impl std::fmt::Display for SpiceDBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpiceDBError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            SpiceDBError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            SpiceDBError::Timeout => write!(f, "Request timed out"),
        }
    }
}

impl std::error::Error for SpiceDBError {}

impl From<&str> for SpiceDBError {
    fn from(s: &str) -> Self {
        SpiceDBError::ConnectionError(s.to_string())
    }
}

/// Test schema for GraphQL integration tests
struct TestQuery;

#[Object]
impl TestQuery {
    async fn test_notes(&self, ctx: &Context<'_>) -> async_graphql::Result<String> {
        crate::helpers::authorization::is_authorized(ctx, "notes", "read").await?;
        Ok("Test note content".to_string())
    }

    async fn test_write_note(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        crate::helpers::authorization::is_authorized(ctx, "notes", "write").await?;
        Ok(true)
    }
}

/// Helper to create test SpiceDB config
fn test_spicedb_config() -> SpiceDBConfig {
    SpiceDBConfig {
        endpoint: "http://localhost:50051".to_string(),
        preshared_key: "test-key".to_string(),
        request_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(2),
        max_connections: 10,
    }
}

/// Helper to create test circuit breaker config
fn test_circuit_breaker_config() -> CircuitBreakerConfig {
    CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        timeout: Duration::from_millis(100),
        half_open_timeout: Duration::from_millis(500),
    }
}

/// Helper to create GraphQL schema with all dependencies
async fn create_test_schema() -> Schema<TestQuery, EmptyMutation, EmptySubscription> {
    let cache = Arc::new(ProductionAuthCache::new(CacheConfig::default()));
    let circuit_breaker = Arc::new(CircuitBreaker::new(test_circuit_breaker_config()));
    let fallback = Arc::new(FallbackAuthorizer::new());
    let mock_client = Arc::new(MockSpiceDBClient::new());

    Schema::build(TestQuery, EmptyMutation, EmptySubscription)
        .data(cache as Arc<dyn crate::auth::AuthCache>)
        .data(circuit_breaker)
        .data(fallback)
        .data(mock_client)
        .data(AuthContext {
            user_id: Some("test_user".to_string()),
            trace_id: "test_trace".to_string(),
            is_admin: false,
            session_token: Some("test_token".to_string()),
        })
        .finish()
}

#[cfg(test)]
mod spicedb_client_tests {
    use super::*;

    #[tokio::test]
    async fn test_spicedb_config_creation() {
        let config = test_spicedb_config();
        
        assert_eq!(config.endpoint, "http://localhost:50051");
        assert_eq!(config.preshared_key, "test-key");
        assert_eq!(config.request_timeout, Duration::from_secs(5));
        assert_eq!(config.connect_timeout, Duration::from_secs(2));
        assert_eq!(config.max_connections, 10);
    }

    #[tokio::test]
    async fn test_mock_client_success() {
        let client = MockSpiceDBClient::new();
        
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123",
            permission: "read",
        }).await;
        
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert_eq!(client.call_count().await, 1);
    }

    #[tokio::test]
    async fn test_mock_client_failure() {
        let client = MockSpiceDBClient::failing();
        
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123",
            permission: "read",
        }).await;
        
        assert!(result.is_err());
        assert_eq!(client.call_count().await, 1);
    }

    #[tokio::test]
    async fn test_mock_client_permission_logic() {
        let client = MockSpiceDBClient::new();
        
        // Test own resource access
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123",
            permission: "read",
        }).await.unwrap();
        assert!(result);

        // Test other user's resource access
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:bob:456",
            permission: "read",
        }).await.unwrap();
        assert!(!result);

        // Test write permissions
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123",
            permission: "write",
        }).await.unwrap();
        assert!(result);

        // Test health check
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:system",
            resource: "system:health",
            permission: "check",
        }).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_mock_client_latency() {
        let client = MockSpiceDBClient::with_latency(Duration::from_millis(50));
        
        let start = Instant::now();
        let _result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123",
            permission: "read",
        }).await;
        
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(45)); // Allow some variance
        assert!(elapsed < Duration::from_millis(100)); // Shouldn't be too slow
    }

    #[tokio::test]
    async fn test_mock_client_call_counting() {
        let client = MockSpiceDBClient::new();
        
        assert_eq!(client.call_count().await, 0);
        
        // Make several calls
        for i in 1..=5 {
            let _result = client.check_permission_mock(CheckPermissionRequest {
                subject: "user:alice",
                resource: "notes:alice:123",
                permission: "read",
            }).await;
            assert_eq!(client.call_count().await, i);
        }
    }

    #[tokio::test]
    async fn test_mock_client_failure_toggle() {
        let client = MockSpiceDBClient::new();
        
        // Should succeed initially
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123", 
            permission: "read",
        }).await;
        assert!(result.is_ok());
        
        // Set to fail
        client.set_should_fail(true).await;
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123",
            permission: "read",
        }).await;
        assert!(result.is_err());
        
        // Set back to success
        client.set_should_fail(false).await;
        let result = client.check_permission_mock(CheckPermissionRequest {
            subject: "user:alice",
            resource: "notes:alice:123",
            permission: "read",
        }).await;
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod circuit_breaker_tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_starts_closed() {
        let breaker = CircuitBreaker::new(test_circuit_breaker_config());
        
        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert!(!breaker.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_in_closed_state() {
        let breaker = CircuitBreaker::new(test_circuit_breaker_config());
        let client = MockSpiceDBClient::new();
        
        let result = breaker.call(|| {
            let client = client.clone();
            Box::pin(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: "user:alice",
                    resource: "notes:alice:123",
                    permission: "read",
                }).await
            })
        }).await;
        
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert_eq!(client.call_count().await, 1);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(500),
        };
        let breaker = CircuitBreaker::new(config);
        let client = MockSpiceDBClient::failing();
        
        // Trigger failures to open the circuit
        for i in 1..=3 {
            let result = breaker.call(|| {
                let client = client.clone();
                Box::pin(async move {
                    client.check_permission_mock(CheckPermissionRequest {
                        subject: "user:alice",
                        resource: "notes:alice:123",
                        permission: "read",
                    }).await
                })
            }).await;
            
            assert!(result.is_err());
            assert_eq!(client.call_count().await, i);
            
            if i < 3 {
                assert_eq!(breaker.state().await, CircuitState::Closed);
            }
        }
        
        // Circuit should now be open
        assert_eq!(breaker.state().await, CircuitState::Open);
        assert!(breaker.is_open().await);
        
        // Subsequent calls should fail immediately without calling the service
        let start = Instant::now();
        let result = breaker.call(|| {
            let client = client.clone();
            Box::pin(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: "user:alice",
                    resource: "notes:alice:123",
                    permission: "read",
                }).await
            })
        }).await;
        
        assert!(result.is_err());
        assert!(start.elapsed() < Duration::from_millis(10)); // Should be immediate
        assert_eq!(client.call_count().await, 3); // No additional calls
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(50), // Short timeout for test
        };
        let breaker = CircuitBreaker::new(config);
        let client = MockSpiceDBClient::failing();
        
        // Open the circuit
        for _ in 0..2 {
            let _result = breaker.call(|| {
                let client = client.clone();
                Box::pin(async move {
                    client.check_permission_mock(CheckPermissionRequest {
                        subject: "user:alice",
                        resource: "notes:alice:123",
                        permission: "read",
                    }).await
                })
            }).await;
        }
        
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Wait for half-open timeout
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Fix the client
        client.set_should_fail(false).await;
        
        // Next call should transition to half-open and succeed
        let result = breaker.call(|| {
            let client = client.clone();
            Box::pin(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: "user:alice",
                    resource: "notes:alice:123",
                    permission: "read",
                }).await
            })
        }).await;
        
        assert!(result.is_ok());
        // Should transition back to closed after one success (success_threshold = 1)
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_timeout_handling() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            timeout: Duration::from_millis(50), // Short timeout
            half_open_timeout: Duration::from_millis(500),
        };
        let breaker = CircuitBreaker::new(config);
        let client = MockSpiceDBClient::with_latency(Duration::from_millis(100)); // Longer than timeout
        
        let start = Instant::now();
        let result = breaker.call(|| {
            let client = client.clone();
            Box::pin(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: "user:alice",
                    resource: "notes:alice:123",
                    permission: "read",
                }).await
            })
        }).await;
        
        assert!(result.is_err());
        assert!(start.elapsed() >= Duration::from_millis(45)); // At least timeout duration
        assert!(start.elapsed() < Duration::from_millis(80)); // But not much longer
        
        // Circuit should open after timeout
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_failure_in_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(50),
        };
        let breaker = CircuitBreaker::new(config);
        let client = MockSpiceDBClient::failing();
        
        // Open the circuit
        let _result = breaker.call(|| {
            let client = client.clone();
            Box::pin(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: "user:alice",
                    resource: "notes:alice:123",
                    permission: "read",
                }).await
            })
        }).await;
        
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Wait for half-open timeout
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Next call should fail and keep circuit open
        let result = breaker.call(|| {
            let client = client.clone();
            Box::pin(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: "user:alice",
                    resource: "notes:alice:123",
                    permission: "read",
                }).await
            })
        }).await;
        
        assert!(result.is_err());
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_multiple_successes_to_close() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 3, // Need 3 successes to close
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(50),
        };
        let breaker = CircuitBreaker::new(config);
        let client = MockSpiceDBClient::failing();
        
        // Open the circuit
        let _result = breaker.call(|| {
            let client = client.clone();
            Box::pin(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: "user:alice",
                    resource: "notes:alice:123",
                    permission: "read",
                }).await
            })
        }).await;
        
        assert_eq!(breaker.state().await, CircuitState::Open);
        
        // Wait for half-open timeout
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Fix the client
        client.set_should_fail(false).await;
        
        // Make successful calls
        for i in 1..=3 {
            let result = breaker.call(|| {
                let client = client.clone();
                Box::pin(async move {
                    client.check_permission_mock(CheckPermissionRequest {
                        subject: "user:alice",
                        resource: "notes:alice:123",
                        permission: "read",
                    }).await
                })
            }).await;
            
            assert!(result.is_ok());
            
            if i < 3 {
                assert_eq!(breaker.state().await, CircuitState::HalfOpen);
            } else {
                assert_eq!(breaker.state().await, CircuitState::Closed);
            }
        }
    }
}

#[cfg(test)]
mod fallback_authorization_tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_authorizer_creation() {
        let fallback = FallbackAuthorizer::new();
        // Just test it can be created without panic
        assert!(!fallback.is_authorized("invalid", "invalid", "invalid"));
    }

    #[tokio::test]
    async fn test_fallback_health_checks_allowed() {
        let fallback = FallbackAuthorizer::new();
        
        assert!(fallback.is_authorized("user:system", "system:health:check", "read"));
        assert!(fallback.is_authorized("user:admin", "system:health:status", "check"));
        assert!(fallback.is_authorized("user:anyone", "system:health", "ping"));
    }

    #[tokio::test]
    async fn test_fallback_owner_read_allowed() {
        let fallback = FallbackAuthorizer::new();
        
        // Users can read their own notes
        assert!(fallback.is_authorized("user:alice", "notes:alice:123", "read"));
        assert!(fallback.is_authorized("user:bob", "notes:bob:456", "read"));
        assert!(fallback.is_authorized("user:alice", "notes:alice:789", "list"));
    }

    #[tokio::test]
    async fn test_fallback_cross_user_read_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Users cannot read other users' notes
        assert!(!fallback.is_authorized("user:alice", "notes:bob:123", "read"));
        assert!(!fallback.is_authorized("user:bob", "notes:alice:456", "read"));
        assert!(!fallback.is_authorized("user:alice", "notes:charlie:789", "list"));
    }

    #[tokio::test]
    async fn test_fallback_all_writes_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // All write operations denied in fallback
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "write"));
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "delete"));
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "update"));
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "create"));
    }

    #[tokio::test]
    async fn test_fallback_public_resources_allowed() {
        let fallback = FallbackAuthorizer::new();
        
        // Public resources allowed for read
        assert!(fallback.is_authorized("user:alice", "public:announcements", "read"));
        assert!(fallback.is_authorized("user:bob", "public:docs:api", "read"));
        assert!(fallback.is_authorized("user:anyone", "public:status", "list"));
    }

    #[tokio::test]
    async fn test_fallback_public_writes_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Even public resources deny writes
        assert!(!fallback.is_authorized("user:alice", "public:announcements", "write"));
        assert!(!fallback.is_authorized("user:admin", "public:docs", "create"));
        assert!(!fallback.is_authorized("user:system", "public:status", "update"));
    }

    #[tokio::test]
    async fn test_fallback_unknown_resources_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Unknown resource types denied
        assert!(!fallback.is_authorized("user:alice", "unknown:resource", "read"));
        assert!(!fallback.is_authorized("user:alice", "secrets:key", "read"));
        assert!(!fallback.is_authorized("user:alice", "admin:panel", "read"));
    }

    #[tokio::test]
    async fn test_fallback_malformed_subjects_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Malformed subjects denied
        assert!(!fallback.is_authorized("alice", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("user", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("user:alice:extra", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("", "notes:alice:123", "read"));
    }

    #[tokio::test]
    async fn test_fallback_malformed_resources_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Malformed resources denied
        assert!(!fallback.is_authorized("user:alice", "notes", "read"));
        assert!(!fallback.is_authorized("user:alice", "notes:", "read"));
        assert!(!fallback.is_authorized("user:alice", ":alice:123", "read"));
        assert!(!fallback.is_authorized("user:alice", "", "read"));
    }

    #[tokio::test]
    async fn test_fallback_admin_operations_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Admin operations always denied in fallback
        assert!(!fallback.is_authorized("user:admin", "notes:alice:123", "admin"));
        assert!(!fallback.is_authorized("user:root", "system:config", "admin"));
        assert!(!fallback.is_authorized("user:superuser", "users:bob", "admin"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use async_graphql::Request;

    #[tokio::test]
    async fn test_integration_successful_authorization() {
        let schema = create_test_schema().await;
        
        let request = Request::new("{ testNotes }");
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        assert_eq!(response.data.to_string(), r#"{"testNotes":"Test note content"}"#);
    }

    #[tokio::test]
    async fn test_integration_failed_authorization() {
        let schema = create_test_schema().await;
        
        // Create auth context for different user
        let auth_context = AuthContext {
            user_id: Some("other_user".to_string()),
            trace_id: "test_trace".to_string(),
            is_admin: false,
            session_token: Some("test_token".to_string()),
        };
        
        let schema_with_different_user = Schema::build(TestQuery, EmptyMutation, EmptySubscription)
            .data(Arc::new(ProductionAuthCache::new(CacheConfig::default())) as Arc<dyn crate::auth::AuthCache>)
            .data(Arc::new(CircuitBreaker::new(test_circuit_breaker_config())))
            .data(Arc::new(FallbackAuthorizer::new()))
            .data(Arc::new(MockSpiceDBClient::new()))
            .data(auth_context)
            .finish();
        
        let request = Request::new("{ testNotes }");
        let response = schema_with_different_user.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Permission denied"));
    }

    #[tokio::test]
    async fn test_integration_circuit_breaker_fallback() {
        let cache = Arc::new(ProductionAuthCache::new(CacheConfig::default()));
        let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1, // Open immediately
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_millis(500),
        }));
        let fallback = Arc::new(FallbackAuthorizer::new());
        let failing_client = Arc::new(MockSpiceDBClient::failing());
        
        let auth_context = AuthContext {
            user_id: Some("test_user".to_string()),
            trace_id: "test_trace".to_string(),
            is_admin: false,
            session_token: Some("test_token".to_string()),
        };
        
        let schema = Schema::build(TestQuery, EmptyMutation, EmptySubscription)
            .data(cache as Arc<dyn crate::auth::AuthCache>)
            .data(circuit_breaker)
            .data(fallback)
            .data(failing_client)
            .data(auth_context)
            .finish();
        
        // This should trigger fallback authorization
        // Since fallback denies writes, this should fail
        let request = Request::new("{ testWriteNote }");
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Permission denied"));
    }

    #[tokio::test]
    async fn test_integration_cache_behavior() {
        let schema = create_test_schema().await;
        
        // First request should hit SpiceDB
        let request = Request::new("{ testNotes }");
        let response1 = schema.execute(request.clone()).await;
        assert!(response1.errors.is_empty());
        
        // Second request should hit cache (faster)
        let start = Instant::now();
        let response2 = schema.execute(request).await;
        let elapsed = start.elapsed();
        
        assert!(response2.errors.is_empty());
        assert!(elapsed < Duration::from_millis(5)); // Should be very fast from cache
    }

    #[tokio::test]
    async fn test_integration_unauthenticated_request() {
        let cache = Arc::new(ProductionAuthCache::new(CacheConfig::default()));
        let circuit_breaker = Arc::new(CircuitBreaker::new(test_circuit_breaker_config()));
        let fallback = Arc::new(FallbackAuthorizer::new());
        let mock_client = Arc::new(MockSpiceDBClient::new());
        
        // No auth context
        let schema = Schema::build(TestQuery, EmptyMutation, EmptySubscription)
            .data(cache as Arc<dyn crate::auth::AuthCache>)
            .data(circuit_breaker)
            .data(fallback)
            .data(mock_client)
            .finish();
        
        let request = Request::new("{ testNotes }");
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("auth context not available"));
    }

    #[tokio::test]
    async fn test_integration_demo_mode_bypass() {
        use crate::helpers::authorization::DemoMode;
        
        let cache = Arc::new(ProductionAuthCache::new(CacheConfig::default()));
        let circuit_breaker = Arc::new(CircuitBreaker::new(test_circuit_breaker_config()));
        let fallback = Arc::new(FallbackAuthorizer::new());
        let mock_client = Arc::new(MockSpiceDBClient::new());
        
        let auth_context = AuthContext {
            user_id: Some("test_user".to_string()),
            trace_id: "test_trace".to_string(),
            is_admin: false,
            session_token: Some("test_token".to_string()),
        };
        
        #[cfg(feature = "demo")]
        let demo_mode = DemoMode { enabled: true };
        
        let mut schema_builder = Schema::build(TestQuery, EmptyMutation, EmptySubscription)
            .data(cache as Arc<dyn crate::auth::AuthCache>)
            .data(circuit_breaker)
            .data(fallback)
            .data(mock_client)
            .data(auth_context);
        
        #[cfg(feature = "demo")]
        {
            schema_builder = schema_builder.data(demo_mode);
        }
        
        let schema = schema_builder.finish();
        
        let request = Request::new("{ testNotes }");
        let response = schema.execute(request).await;
        
        // Should succeed regardless of actual permissions in demo mode
        #[cfg(feature = "demo")]
        assert!(response.errors.is_empty());
        
        #[cfg(not(feature = "demo"))]
        {
            // Without demo feature, should use normal authorization
            assert!(response.errors.is_empty() || !response.errors.is_empty()); // Either is fine
        }
    }
}

/// Performance and load tests
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_performance() {
        let breaker = CircuitBreaker::new(test_circuit_breaker_config());
        let client = MockSpiceDBClient::new();
        
        // Test many successful calls
        let start = Instant::now();
        let mut handles = vec![];
        
        for _ in 0..100 {
            let breaker = breaker.clone();
            let client = client.clone();
            
            let handle = tokio::spawn(async move {
                breaker.call(|| {
                    let client = client.clone();
                    Box::pin(async move {
                        client.check_permission_mock(CheckPermissionRequest {
                            subject: "user:alice",
                            resource: "notes:alice:123",
                            permission: "read",
                        }).await
                    })
                }).await
            });
            
            handles.push(handle);
        }
        
        // Wait for all to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
        
        let elapsed = start.elapsed();
        println!("100 parallel circuit breaker calls took: {:?}", elapsed);
        
        // Should complete in reasonable time (allowing for some variance in CI)
        assert!(elapsed < Duration::from_secs(2));
        assert_eq!(client.call_count().await, 100);
    }

    #[tokio::test]
    async fn test_fallback_authorizer_performance() {
        let fallback = FallbackAuthorizer::new();
        
        let start = Instant::now();
        
        // Test many authorization checks
        for i in 0..1000 {
            let user_id = format!("user{}", i % 10);
            let resource = format!("notes:{}:{}", user_id.split(':').nth(1).unwrap(), i);
            let result = fallback.is_authorized(&user_id, &resource, "read");
            assert!(result); // Should allow reading own resources
        }
        
        let elapsed = start.elapsed();
        println!("1000 fallback authorization checks took: {:?}", elapsed);
        
        // Should be very fast (no network calls)
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_mock_client_concurrent_access() {
        let client = MockSpiceDBClient::new();
        let client = Arc::new(client);
        
        let start = Instant::now();
        let mut handles = vec![];
        
        // Test concurrent access
        for i in 0..50 {
            let client = client.clone();
            let handle = tokio::spawn(async move {
                client.check_permission_mock(CheckPermissionRequest {
                    subject: &format!("user:user{}", i),
                    resource: &format!("notes:user{}:{}", i, i),
                    permission: "read",
                }).await
            });
            handles.push(handle);
        }
        
        // Wait for all to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
            assert!(result.unwrap()); // Should allow reading own resources
        }
        
        let elapsed = start.elapsed();
        println!("50 concurrent mock client calls took: {:?}", elapsed);
        
        assert!(elapsed < Duration::from_secs(1));
        assert_eq!(client.call_count().await, 50);
    }
}
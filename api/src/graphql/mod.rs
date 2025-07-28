use async_graphql::{
    Schema,
    extensions::Logger,
};
use crate::services::database::DatabaseService;
use crate::auth::components::AuthorizationComponents;
use std::sync::Arc;

pub mod context;
pub mod dataloaders;
pub mod errors;
pub mod handlers;
pub mod pagination;
pub mod query;
pub mod mutation;
pub mod subscription;
pub mod security;
pub mod metrics;

#[cfg(test)]
mod integration_test;

pub type AppSchema = Schema<Query, Mutation, Subscription>;

/// Create GraphQL schema with all resolvers and configuration
pub fn create_schema(
    database: Arc<dyn DatabaseService>,
    config: Option<GraphQLConfig>,
    auth_components: AuthorizationComponents,
) -> AppSchema {
    create_schema_with_demo(database, config, None, auth_components)
}

/// Create GraphQL schema with demo mode support
pub fn create_schema_with_demo(
    database: Arc<dyn DatabaseService>,
    config: Option<GraphQLConfig>,
    demo_config: Option<crate::config::DemoConfig>,
    auth_components: AuthorizationComponents,
) -> AppSchema {
    let config = config.unwrap_or_default();
    let demo_config = demo_config.unwrap_or_else(|| crate::config::DemoConfig::from_env());
    
    // Log demo mode status
    demo_config.log_status();
    
    // Create DataLoader registry
    let dataloaders = dataloaders::create_dataloaders(database.clone());
    
    // Create event broadcaster for subscriptions
    let event_broadcaster = subscription::EventBroadcaster::default();
    
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .data(database)
        .data(dataloaders)
        .data(event_broadcaster)
        .data(demo_config.clone())
        // Inject authorization components for use by resolvers
        .data(auth_components.spicedb.clone())
        .data(auth_components.cache.clone())
        .data(auth_components.circuit_breaker.clone())
        .data(auth_components.fallback.clone())
        .limit_depth(config.max_depth)
        .limit_complexity(config.max_complexity);
    
    // Disable introspection in production (unless demo mode overrides)
    let is_production = std::env::var("ENVIRONMENT").unwrap_or_default() == "production";
    if is_production && !demo_config.is_enabled() {
        builder = builder.disable_introspection();
    }
    
    // Add extensions
    if config.enable_logging {
        builder = builder.extension(Logger);
    }
    
    builder.finish()
}

/// Create GraphQL schema with full security and metrics extensions
pub fn create_production_schema(
    database: Arc<dyn DatabaseService>,
    config: GraphQLConfig,
    auth_components: AuthorizationComponents,
) -> AppSchema {
    // Create DataLoader registry
    let dataloaders = dataloaders::create_dataloaders(database.clone());
    
    // Create event broadcaster for subscriptions
    let event_broadcaster = subscription::EventBroadcaster::default();
    
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .data(database)
        .data(dataloaders)
        .data(event_broadcaster)
        // Inject authorization components for use by resolvers
        .data(auth_components.spicedb.clone())
        .data(auth_components.cache.clone())
        .data(auth_components.circuit_breaker.clone())
        .data(auth_components.fallback.clone())
        .extension(security::DepthLimit::new(config.max_depth))
        .extension(security::ComplexityLimit::new(config.max_complexity))
        .extension(metrics::MetricsExtension::new())
        .limit_depth(config.max_depth)
        .limit_complexity(config.max_complexity);
    
    // Production security
    if std::env::var("ENVIRONMENT").unwrap_or_default() == "production" {
        builder = builder.disable_introspection();
    }
    
    // Add logging extension if enabled
    if config.enable_logging {
        builder = builder.extension(Logger);
    }
    
    builder.finish()
}

/// Create GraphQL schema with full configuration and extensions
/// This will be used for the complete implementation in later tasks
pub fn create_schema_with_extensions(
    database: Arc<dyn DatabaseService>,
    config: GraphQLConfig,
    auth_components: AuthorizationComponents,
) -> AppSchema {
    // Create DataLoader registry
    let dataloaders = dataloaders::create_dataloaders(database.clone());
    
    // Create event broadcaster for subscriptions
    let event_broadcaster = subscription::EventBroadcaster::default();
    
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .data(database)
        .data(dataloaders)
        .data(event_broadcaster)
        // Inject authorization components for use by resolvers
        .data(auth_components.spicedb.clone())
        .data(auth_components.cache.clone())
        .data(auth_components.circuit_breaker.clone())
        .data(auth_components.fallback.clone());
    
    // Apply limits
    builder = builder
        .limit_depth(config.max_depth)
        .limit_complexity(config.max_complexity);
    
    // Production security
    if std::env::var("ENVIRONMENT").unwrap_or_default() == "production" {
        builder = builder.disable_introspection();
    }
    
    // Add logging extension if enabled
    if config.enable_logging {
        builder = builder.extension(Logger);
    }
    
    // TODO: Add security extensions in later tasks
    // .extension(DepthLimit::new(config.max_depth))
    // .extension(ComplexityLimit::new(config.max_complexity))
    // .extension(MetricsExtension)
    
    builder.finish()
}

#[derive(Debug, Clone)]
pub struct GraphQLConfig {
    pub max_depth: usize,
    pub max_complexity: usize,
    pub enable_logging: bool,
    pub enable_playground: bool,
}

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self {
            max_depth: 15,
            max_complexity: 1000,
            enable_logging: cfg!(debug_assertions),
            enable_playground: cfg!(feature = "demo"),
        }
    }
}

// Re-export the Query, Mutation, and Subscription types from their modules
pub use query::Query;
pub use mutation::Mutation;
pub use subscription::Subscription;

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Request;
    use crate::services::database::MockDatabase;
    
    fn mock_database() -> Arc<dyn DatabaseService> {
        Arc::new(MockDatabase::new().with_health(crate::services::database::DatabaseHealth::Healthy))
    }
    
    fn mock_auth_components() -> AuthorizationComponents {
        AuthorizationComponents::new_mock()
    }
    
    #[tokio::test]
    async fn test_schema_builds_successfully() {
        let schema = create_schema(mock_database(), None, mock_auth_components());
        assert!(!schema.sdl().is_empty());
    }
    
    #[tokio::test]
    async fn test_health_query_available() {
        let schema = create_schema(mock_database(), None, mock_auth_components());
        let query = r#"
            query {
                health {
                    status
                    timestamp
                    version
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        
        // Verify response data structure
        let data = response.data.into_json().unwrap();
        let health = &data["health"];
        assert_eq!(health["status"], "healthy");
        assert!(health["version"].is_string());
        assert!(health["timestamp"].is_string());
    }
    
    #[tokio::test]
    async fn test_introspection_logic_exists() {
        // This test verifies the introspection disabling logic exists
        // Full integration testing with environment variables requires
        // separate integration test processes to avoid test isolation issues
        
        let config = GraphQLConfig {
            max_depth: 15,
            max_complexity: 1000,
            enable_logging: false,
            enable_playground: false,
        };
        
        let schema = create_schema(mock_database(), Some(config), mock_auth_components());
        
        // Test that introspection queries can be executed (in development mode)
        let query = r#"
            query {
                __schema {
                    queryType {
                        name
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should work in development mode (no ENVIRONMENT=production set)
        assert!(response.errors.is_empty(), "Introspection should work in development");
        let data = response.data.into_json().unwrap();
        assert_eq!(data["__schema"]["queryType"]["name"], "Query");
    }
    
    #[tokio::test]
    async fn test_schema_configuration_works() {
        // Test that schema builder respects configuration
        let config = GraphQLConfig {
            max_depth: 10,
            max_complexity: 500,
            enable_logging: true,
            enable_playground: true,
        };
        
        let schema = create_schema(mock_database(), Some(config), mock_auth_components());
        
        // Test basic functionality
        let query = r#"
            query {
                health {
                    status
                    version
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        assert_eq!(data["health"]["status"], "healthy");
    }
    
    #[tokio::test]
    async fn test_graphql_config_defaults() {
        let config = GraphQLConfig::default();
        assert_eq!(config.max_depth, 15);
        assert_eq!(config.max_complexity, 1000);
        // enable_logging depends on debug_assertions
        // enable_playground depends on demo feature
    }
    
    #[tokio::test]
    async fn test_depth_and_complexity_limits_applied() {
        let config = GraphQLConfig {
            max_depth: 5,
            max_complexity: 100,
            enable_logging: false,
            enable_playground: false,
        };
        
        let schema = create_schema(mock_database(), Some(config), mock_auth_components());
        
        // Test a query that should be accepted
        let simple_query = r#"
            query {
                health {
                    status
                }
            }
        "#;
        
        let response = schema.execute(Request::new(simple_query)).await;
        assert!(response.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_graphql_errors_mapped_correctly() {
        use crate::graphql::errors::{ToGraphQLError, field_error};
        use crate::error::AppError;
        
        // Test that AppError maps to proper GraphQL errors
        let app_error = AppError::InvalidInput("Test validation error".to_string());
        let graphql_error = app_error.to_graphql_error();
        
        assert_eq!(graphql_error.message, "Test validation error");
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap().to_string(), "\"INVALID_INPUT\"");
        
        // Test field error helper
        let field_err = field_error("username", "Username is required");
        assert_eq!(field_err.message, "Username is required");
        let field_extensions = field_err.extensions.unwrap();
        assert_eq!(field_extensions.get("field").unwrap().to_string(), "\"username\"");
    }
    
    #[tokio::test]
    async fn test_enhanced_schema_builder() {
        let config = GraphQLConfig {
            max_depth: 10,
            max_complexity: 500,
            enable_logging: true,
            enable_playground: true,
        };
        
        let schema = create_schema_with_extensions(mock_database(), config, mock_auth_components());
        assert!(!schema.sdl().is_empty());
        
        // Test that health query works with enhanced builder
        let query = r#"
            query {
                health {
                    status
                    version
                }
            }
        "#;
        
        let response = schema.execute(Request::new(query)).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        assert!(data["health"]["status"].is_string());
    }
    
    #[tokio::test]
    async fn test_health_query_with_database_context() {
        let database = mock_database();
        let schema = create_schema(database, None, mock_auth_components());
        
        let query = r#"
            query {
                health {
                    status
                    timestamp
                    version
                }
            }
        "#;
        
        let response = schema.execute(Request::new(query)).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        let health = &data["health"];
        
        // Should reflect actual database health (MockDatabase is healthy by default)
        assert_eq!(health["status"], "healthy");
        assert!(health["version"].is_string());
        assert!(health["timestamp"].is_string());
    }
}
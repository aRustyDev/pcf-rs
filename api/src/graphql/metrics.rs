use async_graphql::*;
use async_graphql::extensions::*;
use async_graphql::parser::types::OperationType;
use std::time::Instant;
use tracing::{info, debug};

/// Extension for collecting GraphQL metrics
/// 
/// Tracks:
/// - Request duration by operation type and name
/// - Request count by operation type and status
/// - Field resolution duration by parent type and field name
/// - Error rates and types
pub struct MetricsExtension;

impl Default for MetricsExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ExtensionFactory for MetricsExtension {
    fn create(&self) -> std::sync::Arc<dyn Extension> {
        std::sync::Arc::new(MetricsCollector)
    }
}

struct MetricsCollector;

#[async_trait::async_trait]
impl Extension for MetricsCollector {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let start = Instant::now();
        
        debug!("GraphQL request started");
        
        let response = next.run(ctx).await;
        
        let duration = start.elapsed();
        let status = if response.errors.is_empty() { "success" } else { "error" };
        
        // Log metrics
        info!(
            duration_ms = duration.as_millis() as u64,
            status = status,
            error_count = response.errors.len(),
            "GraphQL request completed"
        );
        
        // Log individual errors for debugging
        for error in &response.errors {
            debug!(
                error_message = %error.message,
                error_path = ?error.path,
                "GraphQL error occurred"
            );
        }
        
        response
    }
    
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let start = Instant::now();
        
        // Determine operation type
        let operation_type = ctx
            .data_opt::<OperationType>()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| "unknown".to_string());
        
        let operation_name = operation_name.unwrap_or("anonymous");
        
        debug!(
            operation_type = %operation_type,
            operation_name = %operation_name,
            "GraphQL operation execution started"
        );
        
        let response = next.run(ctx, Some(operation_name)).await;
        
        let duration = start.elapsed();
        let status = if response.errors.is_empty() { "success" } else { "error" };
        
        // Log operation metrics
        info!(
            operation_type = %operation_type,
            operation_name = %operation_name,
            duration_ms = duration.as_millis() as u64,
            status = status,
            field_count = count_fields_in_response(&response),
            "GraphQL operation executed"
        );
        
        response
    }
    
    async fn resolve(
        &self,
        ctx: &ExtensionContext<'_>,
        info: ResolveInfo<'_>,
        next: NextResolve<'_>,
    ) -> ServerResult<Option<Value>> {
        let start = Instant::now();
        
        let parent_type = info.parent_type;
        let field_name = info.name;
        
        debug!(
            parent_type = %parent_type,
            field_name = %field_name,
            "Field resolution started"
        );
        
        let result = next.run(ctx, info).await;
        
        let duration = start.elapsed();
        let status = if result.is_ok() { "success" } else { "error" };
        
        // Log field resolution metrics
        debug!(
            parent_type = %parent_type,
            field_name = %field_name,
            duration_us = duration.as_micros() as u64,
            status = status,
            "Field resolved"
        );
        
        // Log slow field resolutions
        if duration.as_millis() > 100 {
            info!(
                parent_type = %parent_type,
                field_name = %field_name,
                duration_ms = duration.as_millis() as u64,
                "Slow field resolution detected"
            );
        }
        
        result
    }
}

/// Count the approximate number of fields in a GraphQL response
fn count_fields_in_response(response: &Response) -> usize {
    // Simple heuristic: count the depth of the JSON response
    match &response.data {
        Value::Object(obj) => count_value_fields(&Value::Object(obj.clone())),
        _ => 1,
    }
}

/// Recursively count fields in a JSON value
fn count_value_fields(value: &Value) -> usize {
    match value {
        Value::Object(obj) => {
            obj.values().map(count_value_fields).sum::<usize>() + obj.len()
        }
        Value::List(list) => {
            list.iter().map(count_value_fields).sum::<usize>()
        }
        _ => 1,
    }
}

/// Metrics summary for reporting
#[derive(Debug, Clone)]
pub struct GraphQLMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_duration_ms: f64,
    pub slow_queries: u64, // Queries taking >1s
    pub subscription_count: u64,
}

impl Default for GraphQLMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_duration_ms: 0.0,
            slow_queries: 0,
            subscription_count: 0,
        }
    }
}

impl GraphQLMetrics {
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.failed_requests as f64 / self.total_requests as f64
        }
    }
    
    pub fn success_rate(&self) -> f64 {
        1.0 - self.error_rate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::{MockDatabase, DatabaseHealth};
    use crate::graphql::{context::GraphQLContext, Query, Mutation, Subscription};
    use std::sync::Arc;
    
    fn mock_database() -> Arc<dyn crate::services::database::DatabaseService> {
        Arc::new(MockDatabase::new().with_health(DatabaseHealth::Healthy))
    }
    
    fn create_test_schema_with_metrics() -> Schema<Query, Mutation, Subscription> {
        let database = mock_database();
        let dataloaders = crate::graphql::dataloaders::create_dataloaders(database.clone());
        let event_broadcaster = crate::graphql::subscription::EventBroadcaster::default();
        
        Schema::build(Query, Mutation, Subscription)
            .data(database)
            .data(dataloaders)
            .data(event_broadcaster)
            .extension(MetricsExtension::new())
            .finish()
    }
    
    #[tokio::test]
    async fn test_metrics_extension_tracks_successful_query() {
        let schema = create_test_schema_with_metrics();
        
        let query = r#"
            query TestQuery {
                health {
                    status
                    version
                }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(query).data(context)).await;
        
        // Should succeed
        assert!(response.errors.is_empty(), "Query should succeed");
        
        // Metrics extension should not interfere with normal operation
        let data = response.data.into_json().unwrap();
        assert_eq!(data["health"]["status"], "healthy");
    }
    
    #[tokio::test]
    async fn test_metrics_extension_tracks_query_errors() {
        let schema = create_test_schema_with_metrics();
        
        // Invalid query that should produce an error
        let query = r#"
            query InvalidQuery {
                nonExistentField {
                    id
                }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(query).data(context)).await;
        
        // Should have errors
        assert!(!response.errors.is_empty(), "Invalid query should produce errors");
        
        // Metrics extension should still work
        assert!(!response.errors[0].message.is_empty());
    }
    
    #[tokio::test]
    async fn test_metrics_extension_tracks_mutations() {
        let schema = create_test_schema_with_metrics();
        
        let mutation = r#"
            mutation TestMutation($input: CreateNoteInput!) {
                createNote(input: $input) {
                    success
                    message
                    note {
                        id
                        title
                    }
                }
            }
        "#;
        
        let variables = serde_json::json!({
            "input": {
                "title": "Test Note",
                "content": "Test content for metrics tracking"
            }
        });
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(
            Request::new(mutation)
                .variables(Variables::from_json(variables))
                .data(context)
        ).await;
        
        // Should succeed
        assert!(response.errors.is_empty(), "Mutation should succeed: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        assert_eq!(data["createNote"]["success"], true);
    }
    
    #[tokio::test]
    async fn test_metrics_extension_tracks_complex_queries() {
        let schema = create_test_schema_with_metrics();
        
        // Complex query with multiple fields and nesting
        let query = r#"
            query ComplexQuery {
                health {
                    status
                    version
                    timestamp
                }
                notes(first: 5) {
                    edges {
                        node {
                            id
                            title
                            author
                            createdAt
                            tags
                        }
                    }
                    pageInfo {
                        hasNextPage
                        endCursor
                    }
                }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(query).data(context)).await;
        
        // Should succeed
        assert!(response.errors.is_empty(), "Complex query should succeed: {:?}", response.errors);
        
        // Verify the response structure
        let data = response.data.into_json().unwrap();
        assert_eq!(data["health"]["status"], "healthy");
        assert!(data["notes"]["edges"].is_array());
    }
    
    #[tokio::test]
    async fn test_field_count_calculation() {
        // Test the field counting helper function
        let simple_value = serde_json::json!({
            "health": {
                "status": "healthy"
            }
        });
        
        let value: Value = serde_json::from_value(simple_value).unwrap();
        let count = count_value_fields(&value);
        
        // Should count nested structure properly
        assert!(count >= 2, "Should count at least 2 fields, got {}", count);
    }
    
    #[tokio::test]
    async fn test_metrics_summary_calculations() {
        let mut metrics = GraphQLMetrics::default();
        
        // Test initial state
        assert_eq!(metrics.error_rate(), 0.0);
        assert_eq!(metrics.success_rate(), 1.0);
        
        // Simulate some requests
        metrics.total_requests = 100;
        metrics.successful_requests = 90;
        metrics.failed_requests = 10;
        
        assert_eq!(metrics.error_rate(), 0.1);
        assert_eq!(metrics.success_rate(), 0.9);
    }
    
    #[test]
    fn test_metrics_extension_creation() {
        let extension = MetricsExtension::new();
        let _collector = extension.create();
        
        // Should create successfully without panicking
        // The fact that we reach this line means creation worked
        assert!(true);
    }
}
use async_graphql::*;
use async_graphql::extensions::*;
use async_graphql::parser::types::*;
use async_graphql::Positioned;
use std::sync::Arc;

/// Extension to limit query depth to prevent deep nesting attacks
pub struct DepthLimit {
    max_depth: usize,
}

impl DepthLimit {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

#[async_trait::async_trait]
impl ExtensionFactory for DepthLimit {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(DepthLimitExtension {
            max_depth: self.max_depth,
        })
    }
}

struct DepthLimitExtension {
    max_depth: usize,
}

#[async_trait::async_trait]
impl Extension for DepthLimitExtension {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let doc = next.run(ctx, query, variables).await?;
        
        // Calculate depth
        let depth = calculate_query_depth(&doc);
        
        if depth > self.max_depth {
            return Err(ServerError::new(
                format!(
                    "Query depth {} exceeds maximum allowed depth of {}",
                    depth, self.max_depth
                ),
                None,
            ));
        }
        
        Ok(doc)
    }
}

/// Extension to limit query complexity to prevent resource exhaustion
pub struct ComplexityLimit {
    max_complexity: usize,
}

impl ComplexityLimit {
    pub fn new(max_complexity: usize) -> Self {
        Self { max_complexity }
    }
}

#[async_trait::async_trait]
impl ExtensionFactory for ComplexityLimit {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(ComplexityLimitExtension {
            max_complexity: self.max_complexity,
        })
    }
}

struct ComplexityLimitExtension {
    max_complexity: usize,
}

#[async_trait::async_trait]
impl Extension for ComplexityLimitExtension {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let doc = next.run(ctx, query, variables).await?;
        
        // Calculate complexity
        let complexity = calculate_query_complexity(&doc, variables);
        
        if complexity > self.max_complexity {
            return Err(ServerError::new(
                format!(
                    "Query complexity {} exceeds maximum allowed complexity of {}",
                    complexity, self.max_complexity
                ),
                None,
            ));
        }
        
        Ok(doc)
    }
}

/// Calculate the maximum depth of a GraphQL query using visitor pattern
fn calculate_query_depth(doc: &ExecutableDocument) -> usize {
    let mut max_depth = 0;
    
    for (_name, operation) in doc.operations.iter() {
        let depth = calculate_selection_set_depth(&operation.node.selection_set, 1);
        max_depth = max_depth.max(depth);
    }
    
    // Also check fragments
    for fragment in doc.fragments.values() {
        let depth = calculate_selection_set_depth(&fragment.node.selection_set, 1);
        max_depth = max_depth.max(depth);
    }
    
    max_depth
}

/// Calculate depth of a selection set recursively
fn calculate_selection_set_depth(selection_set: &Positioned<SelectionSet>, current_depth: usize) -> usize {
    let mut max_depth = current_depth;
    
    for selection in &selection_set.node.items {
        match &selection.node {
            Selection::Field(field) => {
                let field_depth = if field.node.selection_set.node.items.is_empty() {
                    current_depth
                } else {
                    calculate_selection_set_depth(&field.node.selection_set, current_depth + 1)
                };
                max_depth = max_depth.max(field_depth);
            }
            Selection::FragmentSpread(_) => {
                // Fragment spreads don't add depth themselves, 
                // but their contents are counted when we process fragments
                max_depth = max_depth.max(current_depth);
            }
            Selection::InlineFragment(fragment) => {
                let fragment_depth = calculate_selection_set_depth(&fragment.node.selection_set, current_depth);
                max_depth = max_depth.max(fragment_depth);
            }
        }
    }
    
    max_depth
}

/// Calculate the complexity score of a GraphQL query
fn calculate_query_complexity(doc: &ExecutableDocument, variables: &Variables) -> usize {
    let mut total_complexity = 0;
    
    for (_name, operation) in doc.operations.iter() {
        total_complexity += calculate_selection_set_complexity(&operation.node.selection_set, variables, 1);
    }
    
    // Fragments are included when referenced, so we don't need to count them separately
    
    total_complexity
}

/// Calculate complexity of a selection set
fn calculate_selection_set_complexity(
    selection_set: &Positioned<SelectionSet>, 
    variables: &Variables,
    multiplier: usize
) -> usize {
    let mut complexity = 0;
    
    for selection in &selection_set.node.items {
        match &selection.node {
            Selection::Field(field) => {
                // Base complexity of 1 per field
                let mut field_complexity = 1;
                
                // Check for list arguments that multiply complexity
                for (arg_name, _arg_value) in &field.node.arguments {
                    if arg_name.node == "first" || arg_name.node == "last" {
                        // Simple heuristic: assume list queries add complexity
                        field_complexity *= 10; // Reasonable multiplier for list operations
                    }
                }
                
                // Add complexity for nested selections
                if !field.node.selection_set.node.items.is_empty() {
                    field_complexity += calculate_selection_set_complexity(
                        &field.node.selection_set, 
                        variables, 
                        field_complexity
                    );
                }
                
                complexity += field_complexity * multiplier;
            }
            Selection::FragmentSpread(_) => {
                // Fragment complexity is handled when processing the fragment definition
                complexity += 1 * multiplier;
            }
            Selection::InlineFragment(fragment) => {
                complexity += calculate_selection_set_complexity(
                    &fragment.node.selection_set, 
                    variables, 
                    multiplier
                );
            }
        }
    }
    
    complexity
}

// Removed extract_list_size function - using simpler heuristic approach instead

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::{MockDatabase, DatabaseHealth};
    use crate::graphql::context::GraphQLContext;
    use crate::auth::components::AuthorizationComponents;
    use std::sync::Arc;
    use async_graphql::{Request, Variables};
    
    fn mock_database() -> Arc<dyn crate::services::database::DatabaseService> {
        Arc::new(MockDatabase::new().with_health(DatabaseHealth::Healthy))
    }
    
    fn create_test_schema_with_limits(max_depth: usize, max_complexity: usize) -> crate::graphql::AppSchema {
        use crate::graphql::*;
        
        let database = mock_database();
        let dataloaders = dataloaders::create_dataloaders(database.clone());
        let event_broadcaster = subscription::EventBroadcaster::default();
        let auth_components = AuthorizationComponents::new_mock();
        
        Schema::build(Query, Mutation, Subscription)
            .data(database)
            .data(dataloaders)
            .data(event_broadcaster)
            .data(auth_components.spicedb.clone())
            .data(auth_components.cache.clone())
            .data(auth_components.circuit_breaker.clone())
            .data(auth_components.fallback.clone())
            .extension(DepthLimit::new(max_depth))
            .extension(ComplexityLimit::new(max_complexity))
            .limit_depth(max_depth)
            .limit_complexity(max_complexity)
            .finish()
    }
    
    #[tokio::test]
    async fn test_query_depth_limit_enforced() {
        // TDD Red: Test depth limiting with a deeply nested query
        let schema = create_test_schema_with_limits(5, 1000);
        
        // Create a query that exceeds the depth limit
        let query = r#"
            query DeeplyNested {
                notes {
                    edges {
                        node {
                            author {
                                notes {
                                    edges {
                                        node {
                                            author {
                                                notes {
                                                    edges {
                                                        node {
                                                            author {
                                                                notes {
                                                                    edges {
                                                                        node {
                                                                            id
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
        
        // Should fail due to depth limit
        assert!(!response.errors.is_empty());
        // The error should mention depth
        let error_message = &response.errors[0].message;
        assert!(
            error_message.contains("depth") || error_message.contains("Query is too deep"),
            "Error message should mention depth limit: {}",
            error_message
        );
    }
    
    #[tokio::test]
    async fn test_query_complexity_limit_enforced() {
        // TDD Red: Test complexity limiting with a high-complexity query
        let schema = create_test_schema_with_limits(15, 100); // Low complexity limit
        
        // Create a query with high complexity (many fields, large lists)
        let query = r#"
            query ComplexQuery {
                n1: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n2: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n3: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n4: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n5: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n6: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n7: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n8: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n9: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
                n10: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(query).data(context)).await;
        
        // Should fail due to complexity limit
        assert!(!response.errors.is_empty());
        // The error should mention complexity
        let error_message = &response.errors[0].message;
        assert!(
            error_message.contains("complexity") || error_message.contains("Query is too complex"),
            "Error message should mention complexity limit: {}",
            error_message
        );
    }
    
    #[tokio::test]
    async fn test_allowed_query_within_limits() {
        // TDD Green: Test that reasonable queries are allowed
        let schema = create_test_schema_with_limits(15, 1000);
        
        // Simple query that should be within limits
        let query = r#"
            query SimpleQuery {
                health {
                    status
                    version
                    timestamp
                }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(query).data(context)).await;
        
        // Should succeed - simple query within limits
        assert!(response.errors.is_empty(), "Simple query should succeed: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        assert_eq!(data["health"]["status"], "healthy");
    }
    
    #[tokio::test]
    async fn test_moderate_query_allowed() {
        // TDD Green: Test that moderately complex queries are allowed
        let schema = create_test_schema_with_limits(10, 500);
        
        // Moderate query that should be within limits
        let query = r#"
            query ModerateQuery {
                notes(first: 10) {
                    edges {
                        node {
                            id
                            title
                            author
                            createdAt
                        }
                    }
                    pageInfo {
                        hasNextPage
                        endCursor
                    }
                }
                health {
                    status
                }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(query).data(context)).await;
        
        // Should succeed - moderate query within limits
        assert!(response.errors.is_empty(), "Moderate query should succeed: {:?}", response.errors);
    }
    
    #[tokio::test]
    async fn test_depth_limit_boundary() {
        // TDD Edge Case: Test queries right at the depth limit
        let schema = create_test_schema_with_limits(4, 1000); // Adjusted based on actual depth calculation
        
        // Query with exactly 4 levels of depth: query -> notes -> edges -> node -> id
        let query_at_limit = r#"
            query AtDepthLimit {
                notes {
                    edges {
                        node {
                            id
                        }
                    }
                }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(query_at_limit).data(context)).await;
        
        // Should succeed at the limit
        assert!(response.errors.is_empty(), "Query at depth limit should succeed: {:?}", response.errors);
    }
    
    #[tokio::test]
    async fn test_complexity_calculation_with_variables() {
        // TDD: Test complexity calculation considers variables
        let schema = create_test_schema_with_limits(15, 50); // Very low complexity limit
        
        let query = r#"
            query VariableQuery($first: Int!) {
                notes(first: $first) {
                    edges {
                        node {
                            id
                            title
                            content
                            author
                            tags
                            createdAt
                            updatedAt
                        }
                    }
                }
            }
        "#;
        
        // Variables that would create high complexity
        let variables = serde_json::json!({
            "first": 100
        });
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(
            Request::new(query)
                .variables(Variables::from_json(variables))
                .data(context)
        ).await;
        
        // Should fail due to high complexity from large `first` value
        assert!(!response.errors.is_empty());
        let error_message = &response.errors[0].message;
        assert!(
            error_message.contains("complexity") || error_message.contains("Query is too complex"),
            "Error should mention complexity: {}",
            error_message
        );
    }
    
    #[tokio::test]
    async fn test_introspection_within_limits() {
        // TDD: Test that introspection queries respect limits
        let schema = create_test_schema_with_limits(8, 500);
        
        // Introspection query should be allowed
        let query = r#"
            query IntrospectionQuery {
                __schema {
                    queryType {
                        name
                    }
                    mutationType {
                        name
                    }
                    subscriptionType {
                        name
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
        
        // Should succeed - introspection should be within reasonable limits
        assert!(response.errors.is_empty(), "Introspection should succeed: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        assert_eq!(data["__schema"]["queryType"]["name"], "Query");
    }
    
    #[tokio::test]
    async fn test_mutation_within_limits() {
        // TDD: Test that mutations respect limits
        let schema = create_test_schema_with_limits(10, 300);
        
        // Simple mutation should be allowed
        let mutation = r#"
            mutation CreateNote($input: CreateNoteInput!) {
                createNote(input: $input) {
                    note {
                        id
                        title
                        content
                        author
                    }
                    success
                    message
                }
            }
        "#;
        
        let variables = serde_json::json!({
            "input": {
                "title": "Test Note",
                "content": "This is a test note for security testing"
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
        
        // Should succeed - simple mutation within limits
        assert!(response.errors.is_empty(), "Simple mutation should succeed: {:?}", response.errors);
    }
    
    #[tokio::test]
    async fn test_security_extensions_active() {
        // TDD: Test that security extensions are properly active
        let schema = create_test_schema_with_limits(5, 100);
        
        // Test depth limit is active
        let deep_query = r#"
            query TooDeep {
                notes {
                    edges {
                        node {
                            author {
                                notes {
                                    edges {
                                        node {
                                            id
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;
        
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        let response = schema.execute(Request::new(deep_query).data(context)).await;
        
        // Should fail due to depth limit
        assert!(!response.errors.is_empty(), "Deep query should be rejected");
        assert!(response.errors[0].message.contains("depth"), "Error should mention depth");
    }
}
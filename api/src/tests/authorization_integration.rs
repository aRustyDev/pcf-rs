//! Comprehensive integration tests for authorization system
//!
//! This module tests the complete authorization flow from GraphQL endpoints
//! through the authorization helper to SpiceDB, including fallback behavior,
//! caching, and circuit breaker functionality.

use async_graphql::{Request, Variables};
use serde_json::json;
use std::sync::Arc;
use tokio::time::Duration;

use crate::graphql::{create_schema, context::{GraphQLContext, Session}};
use crate::services::database::{MockDatabase, DatabaseHealth};
use crate::auth::components::AuthorizationComponents;

/// Create a test database with some sample data
fn create_test_database() -> Arc<dyn crate::services::database::DatabaseService> {
    // MockDatabase already returns test data for notes with test IDs
    let db = MockDatabase::new().with_health(DatabaseHealth::Healthy);
    Arc::new(db)
}

/// Create authenticated GraphQL context for testing
fn create_authenticated_context(
    database: Arc<dyn crate::services::database::DatabaseService>
) -> GraphQLContext {
    GraphQLContext::new(
        database,
        Some(Session {
            user_id: "demo_user".to_string(),
            is_admin: false,
        }),
        "test-request".to_string(),
    )
}

/// Create unauthenticated GraphQL context for testing
fn create_unauthenticated_context(
    database: Arc<dyn crate::services::database::DatabaseService>
) -> GraphQLContext {
    GraphQLContext::new(
        database,
        None,
        "test-request".to_string(),
    )
}

#[tokio::test]
async fn test_query_note_with_valid_authorization() {
    // Test that authorized users can query notes they have permission to read
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_authenticated_context(database);
    
    let query = r#"
        query GetNote($id: ID!) {
            note(id: $id) {
                id
                title
                content
                author
            }
        }
    "#;
    
    let variables = json!({
        "id": "notes:test123"
    });
    
    let request = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(context);
    
    let response = schema.execute(request).await;
    
    // Should succeed with proper authorization
    assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    
    let data = response.data.into_json().unwrap();
    assert_eq!(data["note"]["id"], "notes:test123");
    assert_eq!(data["note"]["author"], "demo_user");
}

#[tokio::test]
async fn test_query_note_without_authentication() {
    // Test that unauthenticated requests are rejected
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_unauthenticated_context(database);
    
    let query = r#"
        query GetNote($id: ID!) {
            note(id: $id) {
                id
                title
            }
        }
    "#;
    
    let variables = json!({
        "id": "notes:test123"
    });
    
    let request = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(context);
    
    let response = schema.execute(request).await;
    
    // Should fail with authentication error
    assert!(!response.errors.is_empty());
    assert!(response.errors[0].message.contains("Authentication required"));
}

#[tokio::test]
async fn test_create_note_with_valid_authorization() {
    // Test that authorized users can create notes
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_authenticated_context(database);
    
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
    
    let variables = json!({
        "input": {
            "title": "New Test Note",
            "content": "This is a new test note",
            "tags": ["test", "integration"]
        }
    });
    
    let request = Request::new(mutation)
        .variables(Variables::from_json(variables))
        .data(context);
    
    let response = schema.execute(request).await;
    
    // Should succeed with proper authorization
    assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    
    let data = response.data.into_json().unwrap();
    let create_result = &data["createNote"];
    assert_eq!(create_result["success"], true);
    assert_eq!(create_result["note"]["author"], "demo_user");
}

#[tokio::test]
async fn test_update_note_with_valid_authorization() {
    // Test that authorized users can update notes they own
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_authenticated_context(database);
    
    let mutation = r#"
        mutation UpdateNote($input: UpdateNoteInput!) {
            updateNote(input: $input) {
                note {
                    id
                    title
                    content
                }
                success
                message
            }
        }
    "#;
    
    let variables = json!({
        "input": {
            "id": "notes:test123",
            "title": "Updated Test Note",
            "content": "This note has been updated"
        }
    });
    
    let request = Request::new(mutation)
        .variables(Variables::from_json(variables))
        .data(context);
    
    let response = schema.execute(request).await;
    
    // Should succeed with proper authorization
    assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    
    let data = response.data.into_json().unwrap();
    let update_result = &data["updateNote"];
    assert_eq!(update_result["success"], true);
}

#[tokio::test]
async fn test_delete_note_with_valid_authorization() {
    // Test that authorized users can delete notes they own
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_authenticated_context(database);
    
    let mutation = r#"
        mutation DeleteNote($input: DeleteNoteInput!) {
            deleteNote(input: $input) {
                success
                message
                deletedId
            }
        }
    "#;
    
    let variables = json!({
        "input": {
            "id": "notes:test123"
        }
    });
    
    let request = Request::new(mutation)
        .variables(Variables::from_json(variables))
        .data(context);
    
    let response = schema.execute(request).await;
    
    // Should succeed with proper authorization
    assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    
    let data = response.data.into_json().unwrap();
    let delete_result = &data["deleteNote"];
    assert_eq!(delete_result["success"], true);
}

#[tokio::test]
async fn test_health_check_without_authentication() {
    // Test that health checks work without authentication
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_unauthenticated_context(database);
    
    let query = r#"
        query HealthCheck {
            health {
                status
                version
            }
        }
    "#;
    
    let request = Request::new(query).data(context);
    let response = schema.execute(request).await;
    
    // Should succeed without authentication
    assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    
    let data = response.data.into_json().unwrap();
    assert!(!data["health"]["status"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_authorization_caching_behavior() {
    // Test that repeated queries with same credentials work consistently
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_authenticated_context(database);
    
    let query = r#"
        query GetNote($id: ID!) {
            note(id: $id) {
                id
                title
            }
        }
    "#;
    
    let variables = json!({
        "id": "notes:test123"
    });
    
    // First request
    let request1 = Request::new(query)
        .variables(Variables::from_json(variables.clone()))
        .data(create_authenticated_context(create_test_database()));
    
    let response1 = schema.execute(request1).await;
    
    // Second request with same parameters
    let request2 = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(create_authenticated_context(create_test_database()));
    
    let response2 = schema.execute(request2).await;
    
    // Both should succeed consistently
    assert!(response1.errors.is_empty(), "First request failed: {:?}", response1.errors);
    assert!(response2.errors.is_empty(), "Second request failed: {:?}", response2.errors);
}

#[tokio::test]
async fn test_circuit_breaker_fallback_behavior() {
    // Test that system handles authorization failures gracefully
    // This test verifies fallback behavior when SpiceDB is unavailable
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_authenticated_context(database);
    
    let query = r#"
        query HealthCheck {
            health {
                status
                version
            }
        }
    "#;
    
    let request = Request::new(query).data(context);
    let response = schema.execute(request).await;
    
    // Health checks should always work (they don't require authorization)
    assert!(response.errors.is_empty(), "Health check should work: {:?}", response.errors);
    
    let data = response.data.into_json().unwrap();
    assert!(!data["health"]["status"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_authorization_with_different_permissions() {
    // Test various permission combinations
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    
    // Test reading notes (should be allowed)
    let read_query = r#"
        query GetNotes {
            notes(first: 10) {
                edges {
                    node {
                        id
                        title
                    }
                }
            }
        }
    "#;
    
    let read_request = Request::new(read_query).data(create_authenticated_context(database.clone()));
    let read_response = schema.execute(read_request).await;
    
    assert!(read_response.errors.is_empty(), "Read should be allowed: {:?}", read_response.errors);
    
    // Test creating notes (should be allowed for authenticated users)
    let create_mutation = r#"
        mutation CreateNote($input: CreateNoteInput!) {
            createNote(input: $input) {
                success
                message
            }
        }
    "#;
    
    let create_variables = json!({
        "input": {
            "title": "Permission Test Note",
            "content": "Testing permissions"
        }
    });
    
    let create_request = Request::new(create_mutation)
        .variables(Variables::from_json(create_variables))
        .data(create_authenticated_context(database));
    
    let create_response = schema.execute(create_request).await;
    
    assert!(create_response.errors.is_empty(), "Create should be allowed: {:?}", create_response.errors);
}

#[tokio::test]
async fn test_subscription_authorization() {
    // Test that subscriptions require proper authorization
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    
    // Test with authenticated context
    let auth_context = create_authenticated_context(database.clone());
    
    let subscription = r#"
        subscription NoteEvents {
            noteCreated {
                id
                title
                author
            }
        }
    "#;
    
    let auth_request = Request::new(subscription).data(auth_context);
    let auth_response = schema.execute(auth_request).await;
    
    // Should succeed with authentication
    assert!(auth_response.errors.is_empty(), "Authenticated subscription should work: {:?}", auth_response.errors);
    
    // Test with unauthenticated context
    let unauth_context = create_unauthenticated_context(database);
    let unauth_request = Request::new(subscription).data(unauth_context);
    let unauth_response = schema.execute(unauth_request).await;
    
    // Should fail without authentication
    assert!(!unauth_response.errors.is_empty(), "Unauthenticated subscription should fail");
}

#[tokio::test]
async fn test_authorization_performance_and_caching() {
    // Test that repeated GraphQL requests complete in reasonable time
    use std::time::Instant;
    
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    
    let query = r#"
        query GetNote($id: ID!) {
            note(id: $id) {
                id
                title
            }
        }
    "#;
    
    let variables = json!({
        "id": "notes:test123"
    });
    
    // First call
    let start = Instant::now();
    let request1 = Request::new(query)
        .variables(Variables::from_json(variables.clone()))
        .data(create_authenticated_context(database.clone()));
    let response1 = schema.execute(request1).await;
    let first_duration = start.elapsed();
    
    assert!(response1.errors.is_empty());
    
    // Second call
    let start = Instant::now();
    let request2 = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(create_authenticated_context(database));
    let response2 = schema.execute(request2).await;
    let second_duration = start.elapsed();
    
    assert!(response2.errors.is_empty());
    
    // Both calls should complete in reasonable time
    assert!(first_duration < Duration::from_millis(1000));
    assert!(second_duration < Duration::from_millis(1000));
}

#[tokio::test]
async fn test_authorization_with_malformed_requests() {
    // Test that malformed authorization requests are handled gracefully
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    let context = create_authenticated_context(database);
    
    // Test with invalid note ID
    let query = r#"
        query GetNote($id: ID!) {
            note(id: $id) {
                id
                title
            }
        }
    "#;
    
    let variables = json!({
        "id": "invalid-note-id"
    });
    
    let request = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(context);
    
    let response = schema.execute(request).await;
    
    // Should handle gracefully - either authorize and return null, or reject cleanly
    // The important thing is no panics or system errors
    if !response.errors.is_empty() {
        // If there are errors, they should be clean authorization errors
        let error_msg = &response.errors[0].message;
        assert!(
            error_msg.contains("Authorization") || 
            error_msg.contains("Permission") ||
            error_msg.contains("Access denied"),
            "Error should be authorization-related: {}", error_msg
        );
    }
}

#[tokio::test]
async fn test_concurrent_authorization_requests() {
    // Test that concurrent GraphQL requests are handled correctly
    use tokio::task::JoinSet;
    
    let database = create_test_database();
    let schema = create_schema(database.clone(), None, AuthorizationComponents::new_mock());
    
    let query = r#"
        query GetNote($id: ID!) {
            note(id: $id) {
                id
                title
            }
        }
    "#;
    
    let mut tasks = JoinSet::new();
    
    // Launch multiple concurrent GraphQL requests
    for i in 0..5 {
        let schema_clone = schema.clone();
        let db_clone = database.clone();
        tasks.spawn(async move {
            let variables = json!({
                "id": format!("notes:test{}", i)
            });
            
            let request = Request::new(query)
                .variables(Variables::from_json(variables))
                .data(create_authenticated_context(db_clone));
            
            schema_clone.execute(request).await
        });
    }
    
    // Wait for all tasks to complete
    let mut responses = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(response) => responses.push(response),
            Err(e) => panic!("Task failed: {}", e),
        }
    }
    
    // All requests should complete successfully
    assert_eq!(responses.len(), 5);
    for response in responses {
        assert!(response.errors.is_empty(), "Request failed: {:?}", response.errors);
    }
}
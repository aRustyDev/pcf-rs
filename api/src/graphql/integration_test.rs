use async_graphql::Request;
use crate::services::database::{MockDatabase, DatabaseHealth};
use crate::graphql::{create_production_schema, GraphQLConfig, context::GraphQLContext};
use crate::auth::components::AuthorizationComponents;
use std::sync::Arc;

#[tokio::test]
async fn test_production_schema_with_all_extensions() {
    let database = Arc::new(MockDatabase::new().with_health(DatabaseHealth::Healthy));
    let config = GraphQLConfig {
        max_depth: 5,
        max_complexity: 100,
        enable_logging: true,
        enable_playground: false,
    };
    
    let schema = create_production_schema(database.clone(), config, AuthorizationComponents::new_mock());
    
    // Test that simple queries work
    let query = r#"
        query {
            health {
                status
                version
            }
        }
    "#;
    
    let context = GraphQLContext::new(database, None, "test-request".to_string());
    let response = schema.execute(Request::new(query).data(context)).await;
    
    assert!(response.errors.is_empty(), "Simple query should work: {:?}", response.errors);
    
    let data = response.data.into_json().unwrap();
    assert_eq!(data["health"]["status"], "healthy");
}

#[tokio::test]
async fn test_production_schema_depth_limit_active() {
    let database = Arc::new(MockDatabase::new().with_health(DatabaseHealth::Healthy));
    let config = GraphQLConfig {
        max_depth: 3,
        max_complexity: 1000,
        enable_logging: false,
        enable_playground: false,
    };
    
    let schema = create_production_schema(database.clone(), config, AuthorizationComponents::new_mock());
    
    // Test that depth limit is enforced
    let deep_query = r#"
        query {
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
    
    let context = GraphQLContext::new(database, None, "test-request".to_string());
    let response = schema.execute(Request::new(deep_query).data(context)).await;
    
    assert!(!response.errors.is_empty(), "Deep query should be rejected");
    assert!(response.errors[0].message.contains("depth"), "Error should mention depth");
}

#[tokio::test]
async fn test_production_schema_complexity_limit_active() {
    let database = Arc::new(MockDatabase::new().with_health(DatabaseHealth::Healthy));
    let config = GraphQLConfig {
        max_depth: 15,
        max_complexity: 50,
        enable_logging: false,
        enable_playground: false,
    };
    
    let schema = create_production_schema(database.clone(), config, AuthorizationComponents::new_mock());
    
    // Test that complexity limit is enforced
    let complex_query = r#"
        query {
            n1: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
            n2: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
            n3: notes(first: 50) { edges { node { id title content author tags createdAt updatedAt } } }
        }
    "#;
    
    let context = GraphQLContext::new(database, None, "test-request".to_string());
    let response = schema.execute(Request::new(complex_query).data(context)).await;
    
    assert!(!response.errors.is_empty(), "Complex query should be rejected");
    assert!(response.errors[0].message.contains("complexity"), "Error should mention complexity");
}
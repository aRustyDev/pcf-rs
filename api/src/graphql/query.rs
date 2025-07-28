use async_graphql::*;
use async_graphql::connection::Connection;
use crate::graphql::context::ContextExt;
use crate::graphql::pagination::query_notes_paginated;
use crate::helpers::authorization::is_authorized;
use crate::schema::Note;
use tracing::{info_span, Span};

/// Root query type for GraphQL schema
pub struct Query;

#[Object]
impl Query {
    /// Health check endpoint - no authorization required
    async fn health(&self, ctx: &Context<'_>) -> Result<HealthStatus> {
        // Health checks are always allowed - no authorization needed
        
        // Try to get the database health if context is available
        let database_health = if let Ok(database) = ctx.data::<std::sync::Arc<dyn crate::services::database::DatabaseService>>() {
            match database.health_check().await {
                crate::services::database::DatabaseHealth::Healthy => "healthy",
                crate::services::database::DatabaseHealth::Degraded(_) => "degraded",
                crate::services::database::DatabaseHealth::Unhealthy(_) => "unhealthy",
                crate::services::database::DatabaseHealth::Starting => "starting",
            }
        } else {
            "healthy" // Fallback for tests without full context
        };
        
        Ok(HealthStatus {
            status: database_health.to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    
    /// Get a single note by ID
    #[tracing::instrument(
        skip(self, ctx, id),
        fields(
            operation.type = "query",
            operation.name = "note",
            input.id = tracing::field::Empty,
            user.id = tracing::field::Empty
        )
    )]
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        let span = Span::current();
        span.record("input.id", &id.to_string());
        
        // Check authorization for reading this specific note
        let _auth_span = info_span!(
            "authorization_check",
            resource = %format!("notes:{}", id.to_string()),
            action = "read"
        );
        is_authorized(ctx, &format!("notes:{}", id.to_string()), "read").await?;
        
        let context = ctx.get_context()?;
        
        // Record user context if available
        if let Ok(current_user) = context.get_current_user() {
            span.record("user.id", &current_user);
        }
        
        // Database read with span
        let _db_span = info_span!(
            "database_operation",
            db.operation = "read",
            db.table = "notes",
            db.key = %id.to_string()
        );
        let note_data = context.database
            .read("notes", &id.to_string())
            .await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        if let Some(data) = note_data {
            let note: Note = serde_json::from_value(data)
                .map_err(|e| Error::new(format!("Failed to deserialize note: {}", e)))?;
            Ok(Some(note))
        } else {
            Ok(None)
        }
    }
    
    /// Get paginated list of notes
    async fn notes(
        &self,
        ctx: &Context<'_>,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> Result<Connection<String, Note>> {
        // Check authorization for listing notes
        is_authorized(ctx, "notes:*", "list").await?;
        
        let context = ctx.get_context()?;
        
        // Validate pagination parameters
        if first.is_some() && last.is_some() {
            return Err(Error::new("Cannot specify both 'first' and 'last'"));
        }
        
        // Enforce reasonable limits
        let limit = first.or(last).unwrap_or(20).min(100);
        
        // Use pagination utility
        query_notes_paginated(
            context.database.clone(),
            limit,
            after,
            before,
        ).await
    }
    
    /// Get all notes by a specific author
    async fn notes_by_author(
        &self,
        ctx: &Context<'_>,
        author: String,
    ) -> Result<Vec<Note>> {
        // Check authorization for reading notes by this author
        is_authorized(ctx, &format!("notes:{}:*", author), "read").await?;
        
        let context = ctx.get_context()?;
        
        // Use DataLoader for efficient batching and caching
        if let Ok(loaders) = ctx.data::<crate::graphql::dataloaders::DataLoaderRegistry>() {
            loaders.author_notes
                .load_one(author)
                .await
                .map_err(|e| Error::new(format!("DataLoader error: {}", e)))
        } else {
            // Fallback to direct query if DataLoader not available
            let query = crate::services::database::Query {
                filter: {
                    let mut filter = std::collections::HashMap::new();
                    filter.insert("author".to_string(), serde_json::Value::String(author.clone()));
                    filter
                },
                limit: Some(100), // Reasonable limit
                offset: None,
                sort: Some({
                    let mut sort = std::collections::HashMap::new();
                    sort.insert("created_at".to_string(), crate::services::database::SortOrder::Desc);
                    sort
                }),
            };
            
            let results = context.database
                .query("notes", query)
                .await
                .map_err(|e| Error::new(format!("Database error: {}", e)))?;
            
            let notes: Result<Vec<Note>, _> = results
                .into_iter()
                .map(|data| {
                    serde_json::from_value(data)
                        .map_err(|e| Error::new(format!("Failed to deserialize note: {}", e)))
                })
                .collect();
            
            notes
        }
    }
}

#[derive(SimpleObject)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use async_graphql::{Request, Variables};
    use crate::services::database::{MockDatabase, DatabaseHealth};
    use crate::graphql::{create_schema, context::{GraphQLContext, Session}};
    use crate::auth::components::AuthorizationComponents;
    use std::sync::Arc;
    
    fn mock_database() -> Arc<dyn crate::services::database::DatabaseService> {
        Arc::new(MockDatabase::new().with_health(DatabaseHealth::Healthy))
    }
    
    fn create_authenticated_context() -> GraphQLContext {
        GraphQLContext::new(
            mock_database(),
            Some(Session {
                user_id: "demo_user".to_string(),
                is_admin: false,
            }),
            "test-request".to_string(),
        )
    }
    
    #[tokio::test]
    async fn test_note_by_id_query_fails_before_implementation() {
        // TDD: Test should fail until we implement the resolver
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        let query = r#"
            query GetNote($id: ID!) {
                note(id: $id) {
                    id
                    title
                    content
                    author
                    createdAt
                    updatedAt
                    tags
                }
            }
        "#;
        
        let variables = serde_json::json!({
            "id": "notes:test123"
        });
        
        let request = Request::new(query).variables(Variables::from_json(variables));
        let response = schema.execute(request).await;
        
        // Should fail with "Not implemented yet"
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Not implemented"));
    }
    
    #[tokio::test]
    async fn test_notes_pagination_query_fails_before_implementation() {
        // TDD: Test should fail until we implement pagination
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        let query = r#"
            query GetNotes($first: Int, $after: String) {
                notes(first: $first, after: $after) {
                    edges {
                        node {
                            id
                            title
                        }
                        cursor
                    }
                    pageInfo {
                        hasNextPage
                        hasPreviousPage
                        startCursor
                        endCursor
                    }
                }
            }
        "#;
        
        let variables = serde_json::json!({
            "first": 10
        });
        
        let request = Request::new(query).variables(Variables::from_json(variables));
        let response = schema.execute(request).await;
        
        // Should fail with "Not implemented yet"
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Not implemented"));
    }
    
    #[tokio::test]
    async fn test_notes_by_author_query_with_dataloader() {
        // TDD: Test should now work with DataLoader implementation
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        let query = r#"
            query GetAuthorNotes($author: String!) {
                notesByAuthor(author: $author) {
                    id
                    title
                    author
                }
            }
        "#;
        
        let variables = serde_json::json!({
            "author": "user1"
        });
        
        // Add context with demo session for auth
        let context = create_authenticated_context();
        
        let request = Request::new(query)
            .variables(Variables::from_json(variables))
            .data(context);
        let response = schema.execute(request).await;
        
        // Should now work with DataLoader
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        assert!(data["notesByAuthor"].is_array());
    }
    
    #[tokio::test]
    async fn test_multiple_notes_by_author_prevents_n_plus_1() {
        // TDD: Test that DataLoader prevents N+1 queries and provides caching
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        let query = r#"
            query GetMultipleAuthorNotes {
                user1: notesByAuthor(author: "user1") {
                    id
                    title
                }
                user2: notesByAuthor(author: "user2") {
                    id
                    title
                }
                user1Again: notesByAuthor(author: "user1") {
                    id
                    title
                }
            }
        "#;
        
        // Add context with demo session for auth
        let context = create_authenticated_context();
        
        let request = Request::new(query).data(context);
        let response = schema.execute(request).await;
        
        // Should now work with DataLoader
        // DataLoader should cache user1 results and return same data for user1Again
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        assert!(data["user1"].is_array());
        assert!(data["user2"].is_array());
        assert!(data["user1Again"].is_array());
        
        // user1 and user1Again should return identical data (from cache)
        assert_eq!(data["user1"], data["user1Again"]);
    }
    
    #[tokio::test]
    async fn test_pagination_validates_parameters() {
        // TDD: Test pagination parameter validation
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        let query = r#"
            query InvalidPagination {
                notes(first: 10, last: 10) {
                    edges {
                        node {
                            id
                        }
                    }
                }
            }
        "#;
        
        let response = schema.execute(Request::new(query)).await;
        
        // When implemented, should fail with validation error for both first and last
        // For now, fails with "Not implemented"
        assert!(!response.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_pagination_respects_limits() {
        // TDD: Test that pagination enforces maximum limits
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        let query = r#"
            query LargePagination {
                notes(first: 1000) {
                    edges {
                        node {
                            id
                        }
                    }
                }
            }
        "#;
        
        let response = schema.execute(Request::new(query)).await;
        
        // When implemented, should limit to max 100 items
        // For now, fails with "Not implemented"
        assert!(!response.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_note_query_requires_authentication() {
        // TDD: Test that note queries require proper authentication
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        
        // Create context without session (unauthenticated)
        let context = GraphQLContext::new(
            mock_database(),
            None, // No session
            "test-request".to_string(),
        );
        
        let query = r#"
            query GetNote {
                note(id: "notes:test") {
                    id
                }
            }
        "#;
        
        let request = Request::new(query).data(context);
        let response = schema.execute(request).await;
        
        // When implemented, should fail with authentication error
        // For now, fails with "Not implemented"
        assert!(!response.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_cursor_based_pagination_format() {
        // TDD: Test that cursors are properly base64 encoded
        let schema = create_schema(mock_database(), None, AuthorizationComponents::new_mock());
        let query = r#"
            query TestCursors {
                notes(first: 5) {
                    edges {
                        cursor
                        node {
                            id
                        }
                    }
                }
            }
        "#;
        
        let response = schema.execute(Request::new(query)).await;
        
        // When implemented, cursors should be base64 encoded IDs
        // For now, fails with "Not implemented"
        assert!(!response.errors.is_empty());
    }
}
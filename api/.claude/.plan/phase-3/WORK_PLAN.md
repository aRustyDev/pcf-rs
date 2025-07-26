# Phase 3: GraphQL Implementation - Work Plan

## Prerequisites

Before starting Phase 3, ensure you have:
- **Completed Phase 1 & 2**: Server foundation and database layer fully operational
- **GraphQL Knowledge**: Understanding of schemas, resolvers, subscriptions, and GraphQL best practices
- **Async Rust Experience**: Comfortable with async resolvers and stream handling
- **WebSocket Knowledge**: Basic understanding for subscription implementation
- **Security Awareness**: Understanding of GraphQL-specific vulnerabilities (depth attacks, N+1 queries)

## Quick Reference - Essential Resources

### Example Files
All example files are located in `/api/.claude/.spec/examples/`:
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Comprehensive test examples following TDD
- **[GraphQL Security Patterns](../../.spec/examples/graphql-security-patterns.rs)** - Query depth/complexity limiting
- **[Subscription Patterns](../../.spec/examples/subscription-patterns.rs)** - WebSocket subscription examples
- **[DataLoader Patterns](../../.spec/examples/dataloader-patterns.rs)** - N+1 query prevention

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[SPEC.md](../../SPEC.md)** - GraphQL requirements (lines 25-30)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 3 objectives (lines 68-95)
- **[error-handling.md](../../.spec/error-handling.md)** - Error type mappings for GraphQL

### Quick Links
- **Verification Script**: `scripts/verify-phase-3.sh`
- **GraphQL Playground**: `http://localhost:8080/graphql` (demo mode only)
- **Schema Export**: `http://localhost:8080/schema` (demo mode only)

## Overview
This work plan implements a complete GraphQL API with queries, mutations, and subscriptions. Focus is on security (depth/complexity limits), performance (DataLoader pattern), and proper error handling. The implementation follows TDD practices with clear checkpoint boundaries for review and correction.

## Build and Test Commands

Continue using `just` as the command runner:
- `just test` - Run all tests including GraphQL tests
- `just test-graphql` - Run only GraphQL-related tests
- `just graphql-playground` - Start server with GraphQL playground enabled
- `just build` - Build the release binary
- `just clean` - Clean up processes and build artifacts

Always use these commands instead of direct cargo commands to ensure consistency.

## IMPORTANT: Review Process

**This plan includes 5 mandatory review checkpoints where work MUST stop for external review.**

At each checkpoint:
1. **STOP all work** and commit your code
2. **Write any questions** to `api/.claude/.reviews/checkpoint-X-questions.md`
3. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
4. **Wait for feedback** in `api/.claude/.reviews/checkpoint-X-feedback.md`
5. **DO NOT PROCEED** until you receive explicit approval

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Continue following TDD practices from Phase 1:
1. **Write tests FIRST** - Before any implementation
2. **Run tests to see them FAIL** - Confirms test is valid
3. **Write minimal code to make tests PASS** - No more than necessary
4. **REFACTOR** - Clean up while keeping tests green
5. **Document as you go** - Add rustdoc comments and inline explanations

## Done Criteria Checklist
- [ ] GraphQL playground accessible in demo mode
- [ ] All queries functional (note, notes, notesByAuthor, health)
- [ ] All mutations functional (createNote, updateNote, deleteNote)
- [ ] All subscriptions functional (noteCreated, noteUpdated, noteDeleted)
- [ ] Security controls enforced (depth max 15, complexity max 1000)
- [ ] Error handling returns proper GraphQL errors
- [ ] Schema export available in demo mode only
- [ ] N+1 queries prevented with DataLoader
- [ ] Subscriptions work over WebSocket with proper cleanup
- [ ] All resolvers have proper authorization checks (demo mode bypass)
- [ ] Metrics track GraphQL operations
- [ ] No `.unwrap()` or `.expect()` in production code paths

## Work Breakdown with Review Checkpoints

### 3.1 GraphQL Foundation & Context (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Setting up async-graphql with proper types
- **Scope**: Target 400-600 lines across 5-6 files (MUST document justification if outside range)
- **Key Components**: 
  - GraphQL schema builder with type registry (~150 lines)
  - Request context with database/auth integration (~150 lines)
  - Error type mapping to GraphQL errors (~100 lines)
  - Schema export endpoint (demo mode only) (~50 lines)
  - Basic playground setup with security (~100 lines)
- **Patterns**: Dependency injection, context propagation, error mapping

#### Task 3.1.1: Write GraphQL Foundation Tests First
Create `src/graphql/mod.rs` with comprehensive test module. MUST write and run tests first to see them fail before implementing:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{Schema, EmptyMutation, EmptySubscription, Request};
    
    #[tokio::test]
    async fn test_schema_builds_successfully() {
        let schema = create_schema(mock_database(), None);
        assert!(!schema.sdl().is_empty());
    }
    
    #[tokio::test]
    async fn test_health_query_available() {
        let schema = create_schema(mock_database(), None);
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
        assert!(response.data.is_ok());
    }
    
    #[tokio::test]
    async fn test_introspection_disabled_in_production() {
        std::env::set_var("ENVIRONMENT", "production");
        let schema = create_schema(mock_database(), None);
        
        let query = r#"
            query {
                __schema {
                    types {
                        name
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Introspection"));
        std::env::remove_var("ENVIRONMENT");
    }
    
    #[tokio::test]
    async fn test_graphql_errors_mapped_correctly() {
        let schema = create_schema(mock_database(), None);
        // Test that AppError maps to proper GraphQL errors
        // This will fail until error mapping is implemented
    }
}
```

#### Task 3.1.2: Define GraphQL Context
Create request context for dependency injection:
```rust
// src/graphql/context.rs
use crate::services::database::DatabaseService;
use crate::auth::Session;
use async_graphql::{Context, Result, ErrorExtensions};
use std::sync::Arc;

/// GraphQL request context containing shared resources
pub struct GraphQLContext {
    pub database: Arc<dyn DatabaseService>,
    pub session: Option<Session>,
    pub request_id: String,
    #[cfg(feature = "demo")]
    pub demo_mode: bool,
}

impl GraphQLContext {
    pub fn new(
        database: Arc<dyn DatabaseService>,
        session: Option<Session>,
        request_id: String,
    ) -> Self {
        Self {
            database,
            session,
            request_id,
            #[cfg(feature = "demo")]
            demo_mode: true,
        }
    }
    
    /// Check if user is authenticated (demo mode bypass)
    pub fn require_auth(&self) -> Result<&Session> {
        #[cfg(feature = "demo")]
        if self.demo_mode {
            return Ok(self.session.as_ref().unwrap_or(&Session::demo()));
        }
        
        self.session.as_ref()
            .ok_or_else(|| {
                async_graphql::Error::new("Authentication required")
                    .extend_with(|_, e| e.set("code", "UNAUTHENTICATED"))
            })
    }
}

// Extension trait for easy context access
pub trait ContextExt {
    fn get_context(&self) -> Result<&GraphQLContext>;
}

impl<'a> ContextExt for Context<'a> {
    fn get_context(&self) -> Result<&GraphQLContext> {
        self.data::<GraphQLContext>()
            .map_err(|_| async_graphql::Error::new("Context not available"))
    }
}
```

#### Task 3.1.3: Implement Error Mapping
Map application errors to GraphQL errors:
```rust
// src/graphql/errors.rs
use crate::error::AppError;
use async_graphql::{Error as GraphQLError, ErrorExtensions};

impl From<AppError> for GraphQLError {
    fn from(err: AppError) -> Self {
        let (code, message) = match &err {
            AppError::NotFound => ("NOT_FOUND", err.to_string()),
            AppError::Unauthorized => ("UNAUTHORIZED", err.to_string()),
            AppError::InvalidInput(msg) => ("INVALID_INPUT", msg.clone()),
            AppError::DatabaseError(_) => ("DATABASE_ERROR", "Database operation failed".to_string()),
            AppError::Internal(_) => ("INTERNAL_ERROR", "Internal server error".to_string()),
            _ => ("UNKNOWN_ERROR", "An error occurred".to_string()),
        };
        
        GraphQLError::new(message).extend_with(|_, e| e.set("code", code))
    }
}

// Helper for field-level errors
pub fn field_error(field: &str, message: &str) -> GraphQLError {
    GraphQLError::new(message)
        .extend_with(|_, e| {
            e.set("code", "VALIDATION_ERROR");
            e.set("field", field);
        })
}
```

#### Task 3.1.4: Create Schema Builder
Build the GraphQL schema with proper configuration:
```rust
// src/graphql/mod.rs
use async_graphql::{
    Schema, SchemaBuilder, EmptyMutation, EmptySubscription,
    extensions::Logger,
};
use crate::services::database::DatabaseService;
use std::sync::Arc;

pub mod context;
pub mod errors;
pub mod query;
pub mod mutation;
pub mod subscription;

pub type AppSchema = Schema<Query, Mutation, Subscription>;

/// Create GraphQL schema with all resolvers and configuration
pub fn create_schema(
    database: Arc<dyn DatabaseService>,
    config: Option<GraphQLConfig>,
) -> AppSchema {
    let config = config.unwrap_or_default();
    
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .data(database)
        .limit_depth(config.max_depth)
        .limit_complexity(config.max_complexity);
    
    // Disable introspection in production
    if std::env::var("ENVIRONMENT").unwrap_or_default() == "production" {
        builder = builder.disable_introspection();
    }
    
    // Add extensions
    if config.enable_logging {
        builder = builder.extension(Logger);
    }
    
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

// Root query type (placeholder for now)
pub struct Query;

#[async_graphql::Object]
impl Query {
    /// Health check endpoint
    async fn health(&self, ctx: &async_graphql::Context<'_>) -> Result<HealthStatus> {
        Ok(HealthStatus {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

#[derive(async_graphql::SimpleObject)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

// Placeholder types
pub struct Mutation;
pub struct Subscription;
```

#### Task 3.1.5: Add GraphQL Endpoints
Integrate GraphQL with Axum server:
```rust
// src/graphql/handlers.rs
use axum::{
    response::{Html, IntoResponse},
    extract::{Extension, State},
    Json,
};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use crate::graphql::{AppSchema, GraphQLContext};

/// GraphQL playground handler (demo mode only)
pub async fn graphql_playground() -> impl IntoResponse {
    #[cfg(not(feature = "demo"))]
    return (StatusCode::NOT_FOUND, "Not found");
    
    #[cfg(feature = "demo")]
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Main GraphQL handler
pub async fn graphql_handler(
    State(schema): State<AppSchema>,
    Extension(session): Extension<Option<Session>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    let context = GraphQLContext::new(
        schema.data::<Arc<dyn DatabaseService>>().unwrap().clone(),
        session,
        request_id,
    );
    
    schema.execute(req.into_inner().data(context)).await.into()
}

/// Schema export handler (demo mode only)
pub async fn schema_handler(State(schema): State<AppSchema>) -> impl IntoResponse {
    #[cfg(not(feature = "demo"))]
    return (StatusCode::NOT_FOUND, "Not found");
    
    #[cfg(feature = "demo")]
    (StatusCode::OK, schema.sdl())
}
```

---
## 🛑 CHECKPOINT 1: GraphQL Foundation Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Ensure all GraphQL foundation tests are written and failing appropriately
2. Verify schema builds with health query working
3. Check introspection is disabled in production
4. Document all public APIs with rustdoc
5. Write any questions to `api/.claude/.reviews/checkpoint-1-questions.md`
6. Commit with message: "Checkpoint 1: GraphQL foundation complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 3.2 Query Resolvers & DataLoader (3-4 work units)

**Work Unit Context:**
- **Complexity**: High - Implementing efficient data fetching with N+1 prevention
- **Scope**: Target 600-800 lines across 4-5 files (MUST document justification if outside range)
- **Key Components**:
  - Query resolvers for all read operations (~200 lines)
  - DataLoader implementation for batching (~200 lines)
  - Field resolvers with proper error handling (~150 lines)
  - Pagination utilities (~100 lines)
  - Query complexity calculation (~150 lines)
- **Required Algorithms**: MUST implement batching algorithm, cursor-based pagination

#### Task 3.2.1: Write Query Resolver Tests First
Create comprehensive tests for all query operations:
```rust
#[cfg(test)]
mod query_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_note_by_id_query() {
        let schema = create_test_schema();
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
        
        let request = Request::new(query).variables(variables);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_notes_pagination() {
        let schema = create_test_schema();
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
        
        let request = Request::new(query).variables(variables);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_notes_by_author_with_dataloader() {
        // Test that multiple queries for same author use DataLoader
        let schema = create_test_schema();
        let query = r#"
            query GetAuthorNotes {
                user1: notesByAuthor(author: "user1") {
                    id
                }
                user2: notesByAuthor(author: "user2") {
                    id
                }
                user1Again: notesByAuthor(author: "user1") {
                    id
                }
            }
        "#;
        
        // Should only make 2 database queries, not 3
        let response = schema.execute(Request::new(query)).await;
        assert!(response.errors.is_empty());
    }
}
```

#### Task 3.2.2: Implement Query Resolvers
Create the query resolvers with proper error handling:
```rust
// src/graphql/query/mod.rs
use async_graphql::*;
use crate::graphql::context::ContextExt;
use crate::schema::Note;

pub struct Query;

#[Object]
impl Query {
    /// Get a single note by ID
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        let context = ctx.get_context()?;
        context.require_auth()?;
        
        let note = context.database
            .read("notes", &id.to_string())
            .await
            .map_err(|e| e.into())?;
        
        Ok(note.map(|data| serde_json::from_value(data).unwrap()))
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
        let context = ctx.get_context()?;
        context.require_auth()?;
        
        // Validate pagination parameters
        if first.is_some() && last.is_some() {
            return Err(Error::new("Cannot specify both 'first' and 'last'"));
        }
        
        let limit = first.or(last).unwrap_or(20).min(100);
        
        // Implement cursor-based pagination
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
        let context = ctx.get_context()?;
        context.require_auth()?;
        
        // Use DataLoader to batch requests
        let loader = ctx.data::<DataLoader<AuthorNotesLoader>>()?;
        loader.load_one(author).await
            .map(|opt| opt.unwrap_or_default())
    }
}
```

#### Task 3.2.3: Implement DataLoader for N+1 Prevention
Create DataLoader to batch database queries:
```rust
// src/graphql/dataloaders/mod.rs
use async_graphql::dataloader::{DataLoader, Loader};
use std::collections::HashMap;
use async_trait::async_trait;

pub struct AuthorNotesLoader {
    database: Arc<dyn DatabaseService>,
}

#[async_trait]
impl Loader<String> for AuthorNotesLoader {
    type Value = Vec<Note>;
    type Error = Arc<AppError>;
    
    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let mut results = HashMap::new();
        
        // Batch query for all authors at once
        let query = Query::And(
            keys.iter()
                .map(|author| Query::Eq("author".to_string(), author.clone()))
                .collect()
        );
        
        let notes = self.database
            .query("notes", query)
            .await
            .map_err(|e| Arc::new(e.into()))?;
        
        // Group by author
        for note_value in notes {
            let note: Note = serde_json::from_value(note_value).unwrap();
            results.entry(note.author.clone())
                .or_insert_with(Vec::new)
                .push(note);
        }
        
        // Ensure all keys have a value (empty vec if no notes)
        for key in keys {
            results.entry(key.clone()).or_insert_with(Vec::new);
        }
        
        Ok(results)
    }
}

// Factory function for creating DataLoaders
pub fn create_dataloaders(database: Arc<dyn DatabaseService>) -> DataLoaderRegistry {
    DataLoaderRegistry {
        author_notes: DataLoader::new(
            AuthorNotesLoader { database: database.clone() },
            tokio::spawn
        ),
    }
}

pub struct DataLoaderRegistry {
    pub author_notes: DataLoader<AuthorNotesLoader>,
}
```

#### Task 3.2.4: Implement Pagination Utilities
Create reusable pagination logic:
```rust
// src/graphql/pagination.rs
use async_graphql::*;
use base64::{Engine as _, engine::general_purpose};

/// Cursor-based pagination following Relay specification
pub async fn query_notes_paginated(
    database: Arc<dyn DatabaseService>,
    limit: i32,
    after: Option<String>,
    before: Option<String>,
) -> Result<Connection<String, Note>> {
    let mut connection = Connection::new(false, false);
    
    // Decode cursors
    let after_id = after.and_then(|c| decode_cursor(&c));
    let before_id = before.and_then(|c| decode_cursor(&c));
    
    // Build query with cursor constraints
    let mut query = Query::All;
    if let Some(id) = after_id {
        query = Query::And(vec![query, Query::Gt("id".to_string(), id)]);
    }
    if let Some(id) = before_id {
        query = Query::And(vec![query, Query::Lt("id".to_string(), id)]);
    }
    
    // Fetch one extra to determine if there are more pages
    let results = database
        .query_with_limit("notes", query, limit + 1)
        .await?;
    
    let has_next_page = results.len() > limit as usize;
    let notes: Vec<Note> = results
        .into_iter()
        .take(limit as usize)
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
    
    // Build edges with cursors
    for note in notes {
        let cursor = encode_cursor(&note.id);
        connection.edges.push(Edge::new(cursor.clone(), note));
    }
    
    // Set page info
    if let Some(first_edge) = connection.edges.first() {
        connection.page_info.start_cursor = Some(first_edge.cursor.clone());
    }
    if let Some(last_edge) = connection.edges.last() {
        connection.page_info.end_cursor = Some(last_edge.cursor.clone());
    }
    connection.page_info.has_next_page = has_next_page;
    connection.page_info.has_previous_page = after.is_some();
    
    Ok(connection)
}

fn encode_cursor(id: &str) -> String {
    general_purpose::STANDARD.encode(id)
}

fn decode_cursor(cursor: &str) -> Option<String> {
    general_purpose::STANDARD
        .decode(cursor)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}
```

---
## 🛑 CHECKPOINT 2: Query Implementation Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Test all query resolvers work correctly
2. Verify DataLoader prevents N+1 queries
3. Check pagination follows Relay specification
4. Ensure proper error handling throughout
5. Write any questions to `api/.claude/.reviews/checkpoint-2-questions.md`
6. Commit with message: "Checkpoint 2: Query resolvers complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 3.3 Mutation Resolvers (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - CRUD operations with validation
- **Scope**: Target 400-500 lines across 3-4 files (MUST document justification if outside range)
- **Key Components**:
  - Create, Update, Delete mutations (~200 lines)
  - Input type definitions with validation (~100 lines)
  - Authorization checks for mutations (~50 lines)
  - Side effect handling (~100 lines)
- **Patterns**: Input validation, authorization, transactional operations

#### Task 3.3.1: Write Mutation Tests First
Test all mutation operations:
```rust
#[cfg(test)]
mod mutation_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_note_mutation() {
        let schema = create_test_schema();
        let mutation = r#"
            mutation CreateNote($input: CreateNoteInput!) {
                createNote(input: $input) {
                    id
                    title
                    content
                    author
                    tags
                }
            }
        "#;
        
        let variables = serde_json::json!({
            "input": {
                "title": "Test Note",
                "content": "Test content",
                "tags": ["test", "graphql"]
            }
        });
        
        let request = Request::new(mutation).variables(variables);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_update_note_authorization() {
        let schema = create_test_schema();
        let mutation = r#"
            mutation UpdateNote($id: ID!, $input: UpdateNoteInput!) {
                updateNote(id: $id, input: $input) {
                    id
                    title
                    content
                }
            }
        "#;
        
        // Test that users can only update their own notes
        let variables = serde_json::json!({
            "id": "notes:other_user_note",
            "input": {
                "title": "Hacked!"
            }
        });
        
        let request = Request::new(mutation).variables(variables);
        let response = schema.execute(request).await;
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Unauthorized"));
    }
}
```

#### Task 3.3.2: Define Input Types with Validation
Create input types for mutations:
```rust
// src/graphql/mutation/inputs.rs
use async_graphql::*;
use garde::Validate;

#[derive(InputObject, Validate)]
pub struct CreateNoteInput {
    #[graphql(validator(min_length = 1, max_length = 200))]
    #[garde(length(min = 1, max = 200))]
    pub title: String,
    
    #[graphql(validator(min_length = 1, max_length = 10000))]
    #[garde(length(min = 1, max = 10000))]
    pub content: String,
    
    #[graphql(validator(max_items = 10))]
    #[garde(length(max = 10))]
    pub tags: Vec<String>,
}

#[derive(InputObject, Validate)]
pub struct UpdateNoteInput {
    #[graphql(validator(min_length = 1, max_length = 200))]
    #[garde(length(min = 1, max = 200))]
    pub title: Option<String>,
    
    #[graphql(validator(min_length = 1, max_length = 10000))]
    #[garde(length(min = 1, max = 10000))]
    pub content: Option<String>,
    
    #[graphql(validator(max_items = 10))]
    #[garde(length(max = 10))]
    pub tags: Option<Vec<String>>,
}

// Custom validation for input
impl CreateNoteInput {
    pub fn validate_and_sanitize(&mut self) -> Result<()> {
        // Trim whitespace
        self.title = self.title.trim().to_string();
        self.content = self.content.trim().to_string();
        
        // Validate with Garde
        self.validate()
            .map_err(|e| Error::new(format!("Validation failed: {}", e)))?;
        
        // Additional business logic validation
        if self.tags.iter().any(|tag| tag.len() > 50) {
            return Err(Error::new("Tag length cannot exceed 50 characters"));
        }
        
        Ok(())
    }
}
```

#### Task 3.3.3: Implement Mutation Resolvers
Create mutation resolvers with proper authorization:
```rust
// src/graphql/mutation/mod.rs
use async_graphql::*;
use crate::graphql::context::ContextExt;

pub mod inputs;
use inputs::*;

pub struct Mutation;

#[Object]
impl Mutation {
    /// Create a new note
    async fn create_note(
        &self,
        ctx: &Context<'_>,
        mut input: CreateNoteInput,
    ) -> Result<Note> {
        let context = ctx.get_context()?;
        let session = context.require_auth()?;
        
        // Validate and sanitize input
        input.validate_and_sanitize()?;
        
        // Create note with author from session
        let note = Note {
            id: None, // Will be generated by database
            title: input.title,
            content: input.content,
            author: session.user_id.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: input.tags,
        };
        
        let id = context.database
            .create("notes", serde_json::to_value(&note)?)
            .await?;
        
        // Emit event for subscriptions
        if let Ok(broadcaster) = ctx.data::<EventBroadcaster>() {
            broadcaster.send(DomainEvent::NoteCreated(note.clone())).await;
        }
        
        Ok(note.with_id(id))
    }
    
    /// Update an existing note
    async fn update_note(
        &self,
        ctx: &Context<'_>,
        id: ID,
        mut input: UpdateNoteInput,
    ) -> Result<Note> {
        let context = ctx.get_context()?;
        let session = context.require_auth()?;
        
        // Fetch existing note
        let existing = context.database
            .read("notes", &id.to_string())
            .await?
            .ok_or_else(|| Error::new("Note not found"))?;
        
        let mut note: Note = serde_json::from_value(existing)?;
        
        // Check authorization - users can only update their own notes
        if note.author != session.user_id {
            return Err(Error::new("Unauthorized to update this note")
                .extend_with(|_, e| e.set("code", "FORBIDDEN")));
        }
        
        // Apply updates
        if let Some(title) = input.title {
            note.title = title.trim().to_string();
        }
        if let Some(content) = input.content {
            note.content = content.trim().to_string();
        }
        if let Some(tags) = input.tags {
            note.tags = tags;
        }
        note.updated_at = chrono::Utc::now();
        
        // Save to database
        context.database
            .update("notes", &id.to_string(), serde_json::to_value(&note)?)
            .await?;
        
        // Emit event
        if let Ok(broadcaster) = ctx.data::<EventBroadcaster>() {
            broadcaster.send(DomainEvent::NoteUpdated {
                old: note.clone(),
                new: note.clone(),
            }).await;
        }
        
        Ok(note)
    }
    
    /// Delete a note
    async fn delete_note(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let context = ctx.get_context()?;
        let session = context.require_auth()?;
        
        // Fetch note to check ownership
        let existing = context.database
            .read("notes", &id.to_string())
            .await?
            .ok_or_else(|| Error::new("Note not found"))?;
        
        let note: Note = serde_json::from_value(existing)?;
        
        // Check authorization
        if note.author != session.user_id {
            return Err(Error::new("Unauthorized to delete this note")
                .extend_with(|_, e| e.set("code", "FORBIDDEN")));
        }
        
        // Delete from database
        context.database.delete("notes", &id.to_string()).await?;
        
        // Emit event
        if let Ok(broadcaster) = ctx.data::<EventBroadcaster>() {
            broadcaster.send(DomainEvent::NoteDeleted(id.to_string())).await;
        }
        
        Ok(true)
    }
}
```

---
## 🛑 CHECKPOINT 3: Mutation Implementation Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Test all mutations with valid and invalid inputs
2. Verify authorization checks work correctly
3. Check input validation and sanitization
4. Ensure events are emitted for subscriptions
5. Write any questions to `api/.claude/.reviews/checkpoint-3-questions.md`
6. Commit with message: "Checkpoint 3: Mutation resolvers complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 3.4 Subscriptions & WebSocket (3-4 work units)

**Work Unit Context:**
- **Complexity**: High - Real-time event streaming over WebSocket
- **Scope**: Target 600-800 lines across 5-6 files (MUST document justification if outside range)
- **Key Components**:
  - WebSocket protocol handling (~200 lines)
  - Subscription resolvers (~150 lines)
  - Event broadcasting system (~200 lines)
  - Connection lifecycle management (~150 lines)
  - Subscription filtering logic (~100 lines)
- **Required Algorithms**: MUST implement pub/sub pattern, connection cleanup

#### Task 3.4.1: Write Subscription Tests First
Test WebSocket subscriptions:
```rust
#[cfg(test)]
mod subscription_tests {
    use super::*;
    use futures_util::StreamExt;
    
    #[tokio::test]
    async fn test_note_created_subscription() {
        let schema = create_test_schema();
        let subscription = r#"
            subscription OnNoteCreated {
                noteCreated {
                    id
                    title
                    author
                }
            }
        "#;
        
        // Create subscription stream
        let mut stream = schema.execute_stream(subscription);
        
        // Trigger event in another task
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            // Create a note which should trigger the subscription
            create_test_note().await;
        });
        
        // Receive event
        let response = stream.next().await.unwrap();
        assert!(response.errors.is_empty());
        assert!(response.data.is_ok());
    }
    
    #[tokio::test]
    async fn test_filtered_subscription() {
        let schema = create_test_schema();
        let subscription = r#"
            subscription OnAuthorNotes($author: String!) {
                notesByAuthor(author: $author) {
                    id
                    title
                    author
                }
            }
        "#;
        
        let variables = serde_json::json!({
            "author": "test_user"
        });
        
        let request = Request::new(subscription).variables(variables);
        let mut stream = schema.execute_stream(request);
        
        // Should only receive events for specific author
        // Test implementation...
    }
}
```

#### Task 3.4.2: Implement Event Broadcasting
Create event broadcasting system:
```rust
// src/graphql/subscription/broadcaster.rs
use tokio::sync::broadcast;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone)]
pub enum DomainEvent {
    NoteCreated(Note),
    NoteUpdated { old: Note, new: Note },
    NoteDeleted(String),
}

pub struct EventBroadcaster {
    sender: broadcast::Sender<DomainEvent>,
    capacity: usize,
    subscriber_count: Arc<RwLock<usize>>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            capacity,
            subscriber_count: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Send an event to all subscribers
    pub async fn send(&self, event: DomainEvent) {
        let subscriber_count = *self.subscriber_count.read();
        
        if subscriber_count > 0 {
            // Only send if there are active subscribers
            match self.sender.send(event) {
                Ok(count) => {
                    tracing::debug!("Event sent to {} subscribers", count);
                }
                Err(_) => {
                    tracing::warn!("No active subscribers for event");
                }
            }
        }
    }
    
    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        *self.subscriber_count.write() += 1;
        self.sender.subscribe()
    }
    
    /// Get current subscriber count
    pub fn subscriber_count(&self) -> usize {
        *self.subscriber_count.read()
    }
}

// Implement Drop to track subscriber lifecycle
impl Drop for EventSubscription {
    fn drop(&mut self) {
        if let Some(counter) = self.counter.upgrade() {
            *counter.write() -= 1;
        }
    }
}
```

#### Task 3.4.3: Implement Subscription Resolvers
Create subscription resolvers with filtering:
```rust
// src/graphql/subscription/mod.rs
use async_graphql::*;
use futures_util::{Stream, StreamExt};
use crate::graphql::context::ContextExt;

pub mod broadcaster;
use broadcaster::*;

pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to all note creation events
    async fn note_created(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Note>> {
        let context = ctx.get_context()?;
        context.require_auth()?;
        
        let broadcaster = ctx.data::<EventBroadcaster>()?;
        let mut receiver = broadcaster.subscribe();
        
        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                if let DomainEvent::NoteCreated(note) = event {
                    yield note;
                }
            }
        })
    }
    
    /// Subscribe to note updates
    async fn note_updated(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = NoteUpdate>> {
        let context = ctx.get_context()?;
        context.require_auth()?;
        
        let broadcaster = ctx.data::<EventBroadcaster>()?;
        let mut receiver = broadcaster.subscribe();
        
        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                if let DomainEvent::NoteUpdated { old, new } = event {
                    yield NoteUpdate { old, new };
                }
            }
        })
    }
    
    /// Subscribe to notes by specific author
    async fn notes_by_author(
        &self,
        ctx: &Context<'_>,
        author: String,
    ) -> Result<impl Stream<Item = NoteEvent>> {
        let context = ctx.get_context()?;
        let session = context.require_auth()?;
        
        // Users can only subscribe to their own notes unless admin
        if author != session.user_id && !session.is_admin {
            return Err(Error::new("Unauthorized to subscribe to other users' notes"));
        }
        
        let broadcaster = ctx.data::<EventBroadcaster>()?;
        let mut receiver = broadcaster.subscribe();
        
        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                let note_event = match event {
                    DomainEvent::NoteCreated(note) if note.author == author => {
                        Some(NoteEvent::Created(note))
                    }
                    DomainEvent::NoteUpdated { old, new } if new.author == author => {
                        Some(NoteEvent::Updated(NoteUpdate { old, new }))
                    }
                    DomainEvent::NoteDeleted(id) => {
                        // Need to check if this was authored by target user
                        Some(NoteEvent::Deleted(id))
                    }
                    _ => None,
                };
                
                if let Some(event) = note_event {
                    yield event;
                }
            }
        })
    }
}

#[derive(SimpleObject)]
pub struct NoteUpdate {
    pub old: Note,
    pub new: Note,
}

#[derive(Union)]
pub enum NoteEvent {
    Created(Note),
    Updated(NoteUpdate),
    Deleted(String),
}
```

#### Task 3.4.4: Add WebSocket Protocol Support
Integrate WebSocket with Axum:
```rust
// src/graphql/websocket.rs
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use async_graphql::{Data, Schema};
use async_graphql_axum::GraphQLProtocol;
use futures_util::{SinkExt, StreamExt};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    protocol: GraphQLProtocol,
    State(schema): State<AppSchema>,
    Extension(session): Extension<Option<Session>>,
) -> Response {
    ws.protocols(["graphql-ws", "graphql-transport-ws"])
        .on_upgrade(move |socket| handle_websocket(socket, schema, protocol, session))
}

async fn handle_websocket(
    socket: WebSocket,
    schema: AppSchema,
    protocol: GraphQLProtocol,
    session: Option<Session>,
) {
    let (sink, stream) = socket.split();
    
    // Create context for WebSocket connection
    let context = GraphQLContext::new(
        schema.data::<Arc<dyn DatabaseService>>().unwrap().clone(),
        session,
        uuid::Uuid::new_v4().to_string(),
    );
    
    // Handle the GraphQL WebSocket protocol
    let output = async_graphql_axum::GraphQLWebSocket::new(schema.clone(), stream, protocol)
        .on_connection_init(|value| async move {
            // Validate connection parameters if needed
            Ok(Data::default())
        })
        .stream();
    
    // Forward messages
    output
        .map(|msg| match msg {
            Ok(msg) => Ok(msg.into()),
            Err(err) => {
                tracing::error!("WebSocket error: {}", err);
                Err(axum::Error::new(err))
            }
        })
        .forward(sink)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("WebSocket forward error: {}", e);
        });
}

// Add connection limiting middleware
pub struct ConnectionLimiter {
    max_connections: usize,
    current_connections: Arc<AtomicUsize>,
}

impl ConnectionLimiter {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            current_connections: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn check_limit(&self) -> Result<ConnectionGuard> {
        let current = self.current_connections.fetch_add(1, Ordering::Relaxed);
        
        if current >= self.max_connections {
            self.current_connections.fetch_sub(1, Ordering::Relaxed);
            Err(Error::new("Maximum subscription connections reached"))
        } else {
            Ok(ConnectionGuard {
                counter: self.current_connections.clone(),
            })
        }
    }
}
```

---
## 🛑 CHECKPOINT 4: Subscription Implementation Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review:**
1. Test WebSocket connections work properly
2. Verify subscriptions receive real-time events
3. Check connection limits are enforced
4. Test subscription filtering logic
5. Write any questions to `api/.claude/.reviews/checkpoint-4-questions.md`
6. Commit with message: "Checkpoint 4: Subscriptions complete"

**DO NOT PROCEED** until review is complete and approved.

---

### 3.5 Security Controls & Complete Integration (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Security hardening and final integration
- **Scope**: Target 500-600 lines across 4-5 files (MUST document justification if outside range)
- **Key Components**:
  - Query depth/complexity limiting (~150 lines)
  - Rate limiting implementation (~100 lines)
  - Metrics integration (~100 lines)
  - Complete integration tests (~150 lines)
  - Verification scripts (~100 lines)
- **Patterns**: Visitor pattern for query analysis, middleware integration

#### Task 3.5.1: Write Security Tests First
Test all security controls:
```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_query_depth_limit() {
        let schema = create_test_schema();
        
        // Create deeply nested query exceeding limit
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
        
        let response = schema.execute(query).await;
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("depth"));
    }
    
    #[tokio::test]
    async fn test_query_complexity_limit() {
        let schema = create_test_schema();
        
        // Create query with high complexity
        let query = r#"
            query ComplexQuery {
                n1: notes(first: 100) { edges { node { id } } }
                n2: notes(first: 100) { edges { node { id } } }
                n3: notes(first: 100) { edges { node { id } } }
                n4: notes(first: 100) { edges { node { id } } }
                n5: notes(first: 100) { edges { node { id } } }
                n6: notes(first: 100) { edges { node { id } } }
                n7: notes(first: 100) { edges { node { id } } }
                n8: notes(first: 100) { edges { node { id } } }
                n9: notes(first: 100) { edges { node { id } } }
                n10: notes(first: 100) { edges { node { id } } }
            }
        "#;
        
        let response = schema.execute(query).await;
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("complexity"));
    }
}
```

#### Task 3.5.2: Implement Security Extensions
Add query depth and complexity limiting:
```rust
// src/graphql/security/mod.rs
use async_graphql::extensions::*;
use async_graphql::parser::types::*;
use async_graphql::*;

/// Extension to limit query depth
pub struct DepthLimit {
    max_depth: usize,
}

impl DepthLimit {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

#[async_trait::async_trait]
impl Extension for DepthLimit {
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

/// Extension to limit query complexity
pub struct ComplexityLimit {
    max_complexity: usize,
}

impl ComplexityLimit {
    pub fn new(max_complexity: usize) -> Self {
        Self { max_complexity }
    }
}

#[async_trait::async_trait]
impl Extension for ComplexityLimit {
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

fn calculate_query_depth(doc: &ExecutableDocument) -> usize {
    // Implementation using visitor pattern
    // Traverse the query AST and calculate maximum depth
    let mut visitor = DepthCalculator { max_depth: 0, current_depth: 0 };
    visit_document(&mut visitor, doc);
    visitor.max_depth
}

fn calculate_query_complexity(doc: &ExecutableDocument, variables: &Variables) -> usize {
    // Calculate complexity based on:
    // - Number of fields
    // - List sizes (first/last arguments)
    // - Nested selections
    let mut visitor = ComplexityCalculator { 
        total_complexity: 0,
        variables,
    };
    visit_document(&mut visitor, doc);
    visitor.total_complexity
}
```

#### Task 3.5.3: Add GraphQL Metrics
Integrate metrics collection:
```rust
// src/graphql/metrics.rs
use async_graphql::extensions::*;
use prometheus::{HistogramVec, IntCounterVec};

lazy_static! {
    static ref GRAPHQL_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "graphql_request_duration_seconds",
        "GraphQL request duration by operation type and name",
        &["operation_type", "operation_name"]
    ).unwrap();
    
    static ref GRAPHQL_REQUEST_COUNT: IntCounterVec = register_int_counter_vec!(
        "graphql_request_total",
        "Total GraphQL requests by operation type",
        &["operation_type", "status"]
    ).unwrap();
    
    static ref GRAPHQL_FIELD_DURATION: HistogramVec = register_histogram_vec!(
        "graphql_field_resolution_duration_seconds",
        "Field resolution duration",
        &["parent_type", "field_name"]
    ).unwrap();
}

pub struct MetricsExtension;

#[async_trait::async_trait]
impl Extension for MetricsExtension {
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let operation_type = ctx.data::<OperationType>()
            .map(|t| t.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        let timer = GRAPHQL_REQUEST_DURATION
            .with_label_values(&[
                &operation_type,
                operation_name.unwrap_or("anonymous"),
            ])
            .start_timer();
        
        let response = next.run(ctx, operation_name).await;
        timer.observe_duration();
        
        let status = if response.errors.is_empty() { "success" } else { "error" };
        GRAPHQL_REQUEST_COUNT
            .with_label_values(&[&operation_type, status])
            .inc();
        
        response
    }
    
    async fn resolve(
        &self,
        ctx: &ExtensionContext<'_>,
        info: ResolveInfo<'_>,
        next: NextResolve<'_>,
    ) -> ServerResult<Option<Value>> {
        let timer = GRAPHQL_FIELD_DURATION
            .with_label_values(&[
                info.parent_type.name(),
                info.field_name,
            ])
            .start_timer();
        
        let result = next.run(ctx, info).await;
        timer.observe_duration();
        
        result
    }
}
```

#### Task 3.5.4: Complete Integration
Wire everything together:
```rust
// Update src/graphql/mod.rs
pub fn create_schema_with_extensions(
    database: Arc<dyn DatabaseService>,
    broadcaster: EventBroadcaster,
    config: GraphQLConfig,
) -> AppSchema {
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .data(database.clone())
        .data(broadcaster)
        .data(create_dataloaders(database))
        .extension(DepthLimit::new(config.max_depth))
        .extension(ComplexityLimit::new(config.max_complexity))
        .extension(MetricsExtension);
    
    if config.enable_logging {
        builder = builder.extension(Logger);
    }
    
    if std::env::var("ENVIRONMENT").unwrap_or_default() == "production" {
        builder = builder.disable_introspection();
    }
    
    builder.finish()
}

// Update main.rs to add GraphQL routes
pub fn create_graphql_router(schema: AppSchema) -> Router {
    Router::new()
        .route("/graphql", post(graphql_handler).get(graphql_playground))
        .route("/graphql/ws", get(websocket_handler))
        .route("/schema", get(schema_handler))
        .layer(Extension(schema))
}
```

#### Task 3.5.5: Create Verification Script
Add comprehensive verification:
```bash
#!/bin/bash
# scripts/verify-phase-3.sh
set -e

echo "=== Phase 3 GraphQL Implementation Verification ==="

# Check compilation
echo "✓ Checking compilation..."
just build || { echo "Build failed"; exit 1; }

# Run tests
echo "✓ Running GraphQL tests..."
just test-graphql || { echo "Tests failed"; exit 1; }

# Start server in background
echo "✓ Starting server..."
ENVIRONMENT=development cargo run --features demo &
SERVER_PID=$!
sleep 5

# Test GraphQL endpoint
echo "✓ Testing GraphQL health query..."
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ health { status } }"}' \
  | jq .

# Test introspection (should work in demo mode)
echo "✓ Testing introspection..."
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ __schema { types { name } } }"}' \
  | jq '.data.__schema.types | length'

# Test WebSocket connection
echo "✓ Testing WebSocket..."
websocat -t ws://localhost:8080/graphql/ws <<< '{"type":"connection_init"}'

# Check metrics
echo "✓ Verifying metrics..."
curl -s http://localhost:8080/metrics | grep -E "graphql_"

# Cleanup
kill $SERVER_PID

echo "=== All Phase 3 verification passed! ==="
```

---
## 🛑 CHECKPOINT 5: Complete Integration Review

**STOP HERE FOR FINAL EXTERNAL REVIEW**

**Before requesting review:**
1. Run full verification script successfully
2. Test security limits work correctly
3. Verify metrics are collected properly
4. Check all queries, mutations, and subscriptions work
5. Review all documentation
6. Write any questions to `api/.claude/.reviews/checkpoint-5-questions.md`
7. Commit with message: "Checkpoint 5: Phase 3 complete"

**Final Review Checklist:**
- [ ] All Done Criteria met
- [ ] Security controls enforced
- [ ] Metrics properly collected
- [ ] WebSocket subscriptions working
- [ ] Integration with Phase 1 & 2 complete
- [ ] Documentation complete
- [ ] No security vulnerabilities
- [ ] Performance acceptable

**DO NOT PROCEED** to Phase 4 until final approval received.

---

## Test Coverage Requirements

**MUST achieve:**
- ≥80% overall test coverage
- ≥95% coverage on critical paths:
  - Security validation (depth/complexity)
  - Authorization checks
  - Error handling paths
  - WebSocket lifecycle

**MAY exclude from coverage with documentation:**
- Generated GraphQL code
- Metrics collection (if tested manually)
- WebSocket protocol internals

## Common Issues and Solutions

### GraphQL Issues
- **"Cannot return null for non-nullable field"**: Check resolver returns proper Option types
- **Subscription not receiving events**: Verify EventBroadcaster is shared in context
- **WebSocket connection drops**: Check connection limits and timeouts

### Security Issues
- **Depth limit not working**: Ensure extension is added to schema builder
- **Complexity calculation wrong**: Review list size multipliers
- **Introspection still enabled**: Check ENVIRONMENT variable is set

### Performance Issues
- **N+1 queries**: Ensure DataLoader is used for all batch operations
- **Slow subscriptions**: Check event filtering efficiency
- **High memory usage**: Monitor subscription connection count

## Next Phase Preview

Phase 4 will implement Authorization & Authentication:
- SpiceDB integration for permissions
- Session management
- OAuth2/OIDC support
- Permission caching

---
*This work plan follows the same structure and TDD practices as Phase 1 & 2, adapted specifically for GraphQL implementation requirements.*
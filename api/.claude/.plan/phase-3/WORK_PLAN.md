# Phase 3: GraphQL Implementation - Work Plan

## Prerequisites

Before starting Phase 3, ensure you have:
- **Completed Phase 1 & 2**: Server foundation and database layer operational
- **GraphQL Knowledge**: Understanding of schemas, resolvers, subscriptions, and GraphQL best practices
- **Async Rust Experience**: Comfortable with async resolvers and stream handling
- **WebSocket Knowledge**: For subscription implementation
- **Security Awareness**: Understanding of GraphQL-specific vulnerabilities (depth attacks, N+1 queries)

## Quick Reference - Essential Resources

### Example Files
All example files are located in `/api/.claude/.spec/examples/`:
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Comprehensive test examples following TDD
- **[GraphQL Security Patterns](../../.spec/examples/graphql-security-patterns.rs)** - Query depth/complexity limiting (to be created)
- **[Subscription Patterns](../../.spec/examples/subscription-patterns.rs)** - WebSocket subscription examples (to be created)

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[graphql-schema.md](../../.spec/graphql-schema.md)** - Complete GraphQL schema specification
- **[SPEC.md](../../SPEC.md)** - GraphQL requirements (lines 25-30)
- **[ROADMAP.md](../../ROADMAP.md)** - Phase 3 objectives (lines 68-98)

### Quick Links
- **Verification Script**: `scripts/verify-phase-3.sh` (to be created)
- **GraphQL Playground**: `http://localhost:8080/graphql` (demo mode only)
- **Schema Export**: `http://localhost:8080/schema` (demo mode only)

## Overview
This work plan implements a complete GraphQL API with queries, mutations, and subscriptions. Focus is on security (depth/complexity limits), performance (DataLoader pattern), and proper error handling. Each checkpoint represents a natural boundary for review.

## Build and Test Commands

Continue using `just` as the command runner:
- `just test` - Run all tests including GraphQL tests
- `just test-graphql` - Run only GraphQL-related tests
- `just build` - Build the release binary
- `just clean` - Clean up processes and build artifacts

Always use these commands instead of direct cargo commands to ensure consistency.

## IMPORTANT: Review Process

**This plan includes 5 mandatory review checkpoints where work MUST stop for external review.**

At each checkpoint:
1. **STOP all work** and commit your code
2. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
3. **Wait for approval** before continuing to next section

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Continue following TDD practices from Phase 1:
1. **Write tests FIRST** - Before any implementation
2. **Run tests to see them FAIL** - Confirms test is valid
3. **Write minimal code to make tests PASS** - No more than necessary
4. **REFACTOR** - Clean up while keeping tests green
5. **Document as you go** - Add rustdoc comments and inline explanations

## Done Criteria Checklist
- [ ] GraphQL playground accessible in demo mode
- [ ] All queries, mutations, subscriptions functional
- [ ] Security controls enforced (depth, complexity, introspection)
- [ ] Error handling returns proper GraphQL errors
- [ ] Schema export available in demo mode
- [ ] N+1 queries prevented with DataLoader
- [ ] Subscriptions work over WebSocket
- [ ] All resolvers have proper authorization checks
- [ ] Metrics track GraphQL operations

## Work Breakdown with Review Checkpoints

### 3.1 GraphQL Schema Setup & Context (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - Setting up async-graphql with proper types
- **Scope**: ~500 lines across 4-5 files
- **Key Components**: 
  - GraphQL schema builder (~100 lines)
  - Request context with database/auth (~150 lines)
  - Error type mapping (~100 lines)
  - Schema export endpoint (~50 lines)
  - Basic playground setup (~100 lines)
- **Patterns**: Dependency injection, context propagation

#### Task 3.1.1: Write Schema Tests First
Create `src/graphql/mod.rs` with comprehensive test module:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{Schema, EmptyMutation, EmptySubscription};
    
    #[tokio::test]
    async fn test_schema_builds_successfully() {
        let schema = create_schema();
        assert!(!schema.sdl().is_empty());
    }
    
    #[tokio::test]
    async fn test_health_query_available() {
        let schema = create_schema();
        let query = r#"
            query {
                health {
                    status
                    timestamp
                }
            }
        "#;
        
        let result = schema.execute(query).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_introspection_disabled_in_production() {
        std::env::set_var("ENVIRONMENT", "production");
        let schema = create_schema();
        
        let query = r#"
            query {
                __schema {
                    types {
                        name
                    }
                }
            }
        "#;
        
        let result = schema.execute(query).await;
        assert!(result.errors.len() > 0);
        std::env::remove_var("ENVIRONMENT");
    }
}
```

#### Task 3.1.2: Define GraphQL Context
Create request context for dependency injection:
```rust
// src/graphql/context.rs
use crate::services::database::DatabaseService;
use crate::auth::Session;
use async_graphql::{Context, Result};
use std::sync::Arc;

pub struct GraphQLContext {
    pub database: Arc<dyn DatabaseService>,
    pub session: Option<Session>,
    pub trace_id: String,
    pub request_start: Instant,
    // DataLoaders will be added later
}

impl GraphQLContext {
    pub fn new(
        database: Arc<dyn DatabaseService>,
        session: Option<Session>,
        trace_id: String,
    ) -> Self {
        Self {
            database,
            session,
            trace_id,
            request_start: Instant::now(),
        }
    }
    
    /// Check if user is authenticated
    pub fn require_auth(&self) -> Result<&Session> {
        self.session.as_ref()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))
    }
    
    /// Get request duration for metrics
    pub fn request_duration(&self) -> Duration {
        self.request_start.elapsed()
    }
}

// Extension trait for easy access from resolvers
pub trait ContextExt {
    fn ctx(&self) -> Result<&GraphQLContext>;
}

impl<'a> ContextExt for Context<'a> {
    fn ctx(&self) -> Result<&GraphQLContext> {
        self.data::<GraphQLContext>()
            .map_err(|_| async_graphql::Error::new("Context not available"))
    }
}
```

#### Task 3.1.3: Implement Error Mapping
Map application errors to GraphQL errors:
```rust
// src/graphql/errors.rs
use async_graphql::{Error as GraphQLError, ErrorExtensions};
use crate::error::AppError;

impl From<AppError> for GraphQLError {
    fn from(err: AppError) -> Self {
        let (code, message) = match &err {
            AppError::InvalidInput(msg) => ("INVALID_INPUT", msg.clone()),
            AppError::NotFound(msg) => ("NOT_FOUND", msg.clone()),
            AppError::Unauthorized(msg) => ("UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => ("FORBIDDEN", msg.clone()),
            AppError::ServiceUnavailable(msg) => ("SERVICE_UNAVAILABLE", msg.clone()),
            AppError::Internal(_) => ("INTERNAL_ERROR", "An internal error occurred".to_string()),
        };
        
        GraphQLError::new(message)
            .extend_with(|_, e| {
                e.set("code", code);
                e.set("timestamp", chrono::Utc::now().to_rfc3339());
                if let Ok(trace_id) = std::env::var("TRACE_ID") {
                    e.set("traceId", trace_id);
                }
            })
    }
}

// Validation error mapping
impl From<garde::Errors> for GraphQLError {
    fn from(errors: garde::Errors) -> Self {
        let messages: Vec<String> = errors
            .into_iter()
            .map(|(path, errors)| {
                format!("{}: {}", path, errors.join(", "))
            })
            .collect();
        
        GraphQLError::new(messages.join("; "))
            .extend_with(|_, e| {
                e.set("code", "VALIDATION_ERROR");
            })
    }
}
```

#### Task 3.1.4: Create Schema Builder
Build the GraphQL schema with proper configuration:
```rust
// src/graphql/mod.rs
use async_graphql::{Schema, SchemaBuilder, EmptyMutation, EmptySubscription};
use async_graphql::extensions::Logger;

pub mod context;
pub mod errors;
pub mod resolvers;

use context::GraphQLContext;

pub type AppSchema = Schema<Query, Mutation, Subscription>;

/// Create the GraphQL schema with all configurations
pub fn create_schema() -> SchemaBuilder<Query, Mutation, Subscription> {
    let mut builder = Schema::build(Query, Mutation, Subscription)
        .extension(Logger);
    
    // Add security extensions
    if !is_demo_mode() {
        builder = builder
            .disable_introspection()
            .limit_depth(get_max_depth())
            .limit_complexity(get_max_complexity());
    }
    
    builder
}

fn is_demo_mode() -> bool {
    cfg!(feature = "demo") && cfg!(debug_assertions)
}

fn get_max_depth() -> usize {
    std::env::var("GRAPHQL_MAX_DEPTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(15)
}

fn get_max_complexity() -> usize {
    std::env::var("GRAPHQL_MAX_COMPLEXITY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000)
}

// Temporary empty types - will be replaced in next sections
pub struct Query;
pub struct Mutation;
pub struct Subscription;
```

---
## 🛑 CHECKPOINT 1: GraphQL Foundation Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Created GraphQL context with database and session
2. Implemented error mapping from AppError to GraphQL errors
3. Built schema with security configurations
4. Disabled introspection in production
5. Added depth and complexity limits
6. Written tests for schema building
7. Documented all public APIs
8. Committed all work with message: "Checkpoint 1: GraphQL foundation complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 1
- Your git commit hash

**DO NOT PROCEED** until you receive explicit approval.

---

### 3.2 Query Resolvers Implementation (3-4 work units)

**Work Unit Context:**
- **Complexity**: Medium - Implementing all query operations
- **Scope**: ~600 lines across 3-4 files
- **Key Components**:
  - Query root type with all operations (~200 lines)
  - Note queries with pagination (~200 lines)
  - Search functionality (~100 lines)
  - Health check query (~50 lines)
  - Authorization checks (~100 lines)
- **Patterns**: Async resolvers, pagination, authorization

#### Task 3.2.1: Write Query Resolver Tests First
Create comprehensive tests for all queries:
```rust
#[cfg(test)]
mod query_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_note_by_id_query() {
        let schema = create_test_schema().await;
        
        // Create a note first
        let note_id = create_test_note(&schema).await;
        
        let query = format!(r#"
            query {{
                note(id: "{}") {{
                    id
                    title
                    content
                    author
                }}
            }}
        "#, note_id);
        
        let result = schema.execute(query).await;
        assert!(result.is_ok());
        
        let data = result.data.into_json().unwrap();
        assert_eq!(data["note"]["id"], note_id);
    }
    
    #[tokio::test]
    async fn test_notes_pagination() {
        let schema = create_test_schema().await;
        
        // Create 15 notes
        for i in 0..15 {
            create_test_note_with_title(&schema, &format!("Note {}", i)).await;
        }
        
        let query = r#"
            query {
                notes(limit: 10, offset: 0) {
                    edges {
                        node {
                            title
                        }
                    }
                    pageInfo {
                        hasNextPage
                        totalCount
                    }
                }
            }
        "#;
        
        let result = schema.execute(query).await;
        let data = result.data.into_json().unwrap();
        
        assert_eq!(data["notes"]["edges"].as_array().unwrap().len(), 10);
        assert_eq!(data["notes"]["pageInfo"]["hasNextPage"], true);
        assert_eq!(data["notes"]["pageInfo"]["totalCount"], 15);
    }
    
    #[tokio::test]
    async fn test_search_notes() {
        let schema = create_test_schema().await;
        
        create_test_note_with_title(&schema, "GraphQL Tutorial").await;
        create_test_note_with_title(&schema, "REST API Guide").await;
        create_test_note_with_title(&schema, "GraphQL Best Practices").await;
        
        let query = r#"
            query {
                searchNotes(query: "GraphQL") {
                    title
                }
            }
        "#;
        
        let result = schema.execute(query).await;
        let data = result.data.into_json().unwrap();
        let results = data["searchNotes"].as_array().unwrap();
        
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|n| n["title"].as_str().unwrap().contains("GraphQL")));
    }
}
```

#### Task 3.2.2: Implement Query Root Type
Create the main Query type with all operations:
```rust
// src/graphql/resolvers/queries.rs
use async_graphql::*;
use crate::graphql::context::{GraphQLContext, ContextExt};
use crate::schema::demo::Note;

#[derive(Default)]
pub struct Query;

#[Object]
impl Query {
    /// Get a single note by ID
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        let context = ctx.ctx()?;
        
        // In demo mode, skip auth check
        #[cfg(not(feature = "demo"))]
        context.require_auth()?;
        
        let note = context.database
            .read("notes", &id.to_string())
            .await
            .map_err(|e| e.into())?;
        
        Ok(note.map(|n| serde_json::from_value(n).unwrap()))
    }
    
    /// List all notes with pagination
    #[graphql(complexity = "limit.unwrap_or(10) as usize")]
    async fn notes(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10, validator(minimum = 1, maximum = 100))]
        limit: Option<i32>,
        #[graphql(default = 0, validator(minimum = 0, maximum = 10000))]
        offset: Option<i32>,
    ) -> Result<NotesConnection> {
        let context = ctx.ctx()?;
        let limit = limit.unwrap_or(10) as usize;
        let offset = offset.unwrap_or(0) as usize;
        
        // Query database
        let query = Query::new()
            .limit(limit)
            .offset(offset)
            .order_by("created_at DESC");
            
        let notes = context.database
            .query("notes", query)
            .await
            .map_err(|e| e.into())?;
        
        // Get total count
        let total_count = context.database
            .count("notes")
            .await
            .map_err(|e| e.into())?;
        
        // Build connection
        Ok(build_connection(notes, total_count, limit, offset))
    }
    
    /// List notes by author with pagination
    #[graphql(complexity = "limit.unwrap_or(10) as usize")]
    async fn notes_by_author(
        &self,
        ctx: &Context<'_>,
        author: String,
        #[graphql(default = 10, validator(minimum = 1, maximum = 100))]
        limit: Option<i32>,
        #[graphql(default = 0, validator(minimum = 0, maximum = 10000))]
        offset: Option<i32>,
    ) -> Result<NotesConnection> {
        let context = ctx.ctx()?;
        
        // Validate author parameter
        if author.is_empty() || author.len() > 100 {
            return Err(Error::new("Invalid author parameter"));
        }
        
        let limit = limit.unwrap_or(10) as usize;
        let offset = offset.unwrap_or(0) as usize;
        
        let query = Query::new()
            .filter("author = $author")
            .bind("author", author.clone())
            .limit(limit)
            .offset(offset)
            .order_by("created_at DESC");
            
        let notes = context.database
            .query("notes", query)
            .await
            .map_err(|e| e.into())?;
        
        let total_count = context.database
            .count_where("notes", "author = $author", &[("author", &author)])
            .await
            .map_err(|e| e.into())?;
        
        Ok(build_connection(notes, total_count, limit, offset))
    }
    
    /// Search notes by title or content
    #[graphql(complexity = "50")] // Expensive operation
    async fn search_notes(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 10, validator(minimum = 1, maximum = 100))]
        limit: Option<i32>,
    ) -> Result<Vec<Note>> {
        let context = ctx.ctx()?;
        
        // Validate search query
        if query.is_empty() || query.len() > 200 {
            return Err(Error::new("Invalid search query"));
        }
        
        let limit = limit.unwrap_or(10) as usize;
        
        // Use full-text search if available, otherwise fallback to LIKE
        let search_query = Query::new()
            .filter("title ~ $query OR content ~ $query")
            .bind("query", query)
            .limit(limit);
            
        let notes = context.database
            .query("notes", search_query)
            .await
            .map_err(|e| e.into())?;
        
        Ok(notes.into_iter()
            .map(|n| serde_json::from_value(n).unwrap())
            .collect())
    }
    
    /// Health check query (always available)
    async fn health(&self, ctx: &Context<'_>) -> Result<HealthStatus> {
        let context = ctx.ctx()?;
        
        let db_health = context.database.health_check().await;
        
        Ok(HealthStatus {
            status: if db_health.is_healthy() { "healthy" } else { "unhealthy" }.to_string(),
            timestamp: chrono::Utc::now(),
            services: vec![
                ServiceHealth {
                    name: "database".to_string(),
                    status: format!("{:?}", db_health),
                }
            ],
        })
    }
}

#[derive(SimpleObject)]
struct HealthStatus {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    services: Vec<ServiceHealth>,
}

#[derive(SimpleObject)]
struct ServiceHealth {
    name: String,
    status: String,
}
```

#### Task 3.2.3: Implement Pagination Types
Create connection types for cursor-based pagination:
```rust
// src/graphql/resolvers/pagination.rs
use async_graphql::*;

#[derive(SimpleObject)]
pub struct NotesConnection {
    edges: Vec<NoteEdge>,
    page_info: PageInfo,
    total_count: i32,
}

#[derive(SimpleObject)]
pub struct NoteEdge {
    node: Note,
    cursor: String,
}

#[derive(SimpleObject)]
pub struct PageInfo {
    has_next_page: bool,
    has_previous_page: bool,
    start_cursor: Option<String>,
    end_cursor: Option<String>,
}

pub fn build_connection(
    notes: Vec<serde_json::Value>,
    total_count: usize,
    limit: usize,
    offset: usize,
) -> NotesConnection {
    let edges: Vec<NoteEdge> = notes
        .into_iter()
        .enumerate()
        .map(|(idx, value)| {
            let note: Note = serde_json::from_value(value).unwrap();
            NoteEdge {
                cursor: base64::encode(format!("cursor:{}", offset + idx)),
                node: note,
            }
        })
        .collect();
    
    let page_info = PageInfo {
        has_next_page: offset + limit < total_count,
        has_previous_page: offset > 0,
        start_cursor: edges.first().map(|e| e.cursor.clone()),
        end_cursor: edges.last().map(|e| e.cursor.clone()),
    };
    
    NotesConnection {
        edges,
        page_info,
        total_count: total_count as i32,
    }
}
```

### 3.3 Mutation Resolvers Implementation (2-3 work units)

**Work Unit Context:**
- **Complexity**: Medium - CRUD operations with validation
- **Scope**: ~500 lines across 2-3 files
- **Key Components**:
  - Mutation root type (~150 lines)
  - Create/Update/Delete operations (~200 lines)
  - Input validation with Garde (~100 lines)
  - Authorization checks (~50 lines)
- **Patterns**: Input validation, authorization, optimistic updates

#### Task 3.3.1: Write Mutation Tests First
Test all mutation operations:
```rust
#[cfg(test)]
mod mutation_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_note_mutation() {
        let schema = create_test_schema().await;
        
        let mutation = r#"
            mutation {
                createNote(input: {
                    title: "Test Note"
                    content: "Test content"
                    author: "test_user"
                }) {
                    id
                    title
                    content
                    author
                }
            }
        "#;
        
        let result = schema.execute(mutation).await;
        assert!(result.is_ok());
        
        let data = result.data.into_json().unwrap();
        assert_eq!(data["createNote"]["title"], "Test Note");
        assert!(!data["createNote"]["id"].as_str().unwrap().is_empty());
    }
    
    #[tokio::test]
    async fn test_update_note_mutation() {
        let schema = create_test_schema().await;
        let note_id = create_test_note(&schema).await;
        
        let mutation = format!(r#"
            mutation {{
                updateNote(id: "{}", input: {{
                    title: "Updated Title"
                }}) {{
                    id
                    title
                    updatedAt
                }}
            }}
        "#, note_id);
        
        let result = schema.execute(mutation).await;
        assert!(result.is_ok());
        
        let data = result.data.into_json().unwrap();
        assert_eq!(data["updateNote"]["title"], "Updated Title");
    }
    
    #[tokio::test]
    async fn test_delete_note_mutation() {
        let schema = create_test_schema().await;
        let note_id = create_test_note(&schema).await;
        
        let mutation = format!(r#"
            mutation {{
                deleteNote(id: "{}")
            }}
        "#, note_id);
        
        let result = schema.execute(mutation).await;
        assert!(result.is_ok());
        
        let data = result.data.into_json().unwrap();
        assert_eq!(data["deleteNote"], true);
        
        // Verify note is deleted
        let query = format!(r#"
            query {{
                note(id: "{}") {{
                    id
                }}
            }}
        "#, note_id);
        
        let result = schema.execute(query).await;
        let data = result.data.into_json().unwrap();
        assert!(data["note"].is_null());
    }
}
```

#### Task 3.3.2: Implement Mutation Root Type
Create mutations with proper validation:
```rust
// src/graphql/resolvers/mutations.rs
use async_graphql::*;
use garde::Validate;
use crate::graphql::context::{GraphQLContext, ContextExt};

#[derive(Default)]
pub struct Mutation;

#[derive(Debug, Validate, InputObject)]
pub struct CreateNoteInput {
    #[garde(length(min = 1, max = 200))]
    title: String,
    
    #[garde(length(min = 1, max = 10000))]
    content: String,
    
    #[garde(length(min = 1, max = 100))]
    author: String,
}

#[derive(Debug, Validate, InputObject)]
pub struct UpdateNoteInput {
    #[garde(length(min = 1, max = 200))]
    title: Option<String>,
    
    #[garde(length(min = 1, max = 10000))]
    content: Option<String>,
}

#[Object]
impl Mutation {
    /// Create a new note
    async fn create_note(
        &self,
        ctx: &Context<'_>,
        input: CreateNoteInput,
    ) -> Result<Note> {
        let context = ctx.ctx()?;
        
        // Validate input
        input.validate()
            .map_err(|e| GraphQLError::from(e))?;
        
        // Check authorization (in demo mode, allow all)
        #[cfg(not(feature = "demo"))]
        {
            let session = context.require_auth()?;
            // Additional permission checks here
        }
        
        let note = Note {
            id: None,
            title: input.title,
            content: input.content,
            author: input.author,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        let id = context.database
            .create("notes", serde_json::to_value(&note)?)
            .await
            .map_err(|e| e.into())?;
        
        // Fetch created note
        let created = context.database
            .read("notes", &id)
            .await
            .map_err(|e| e.into())?
            .ok_or_else(|| Error::new("Failed to fetch created note"))?;
        
        Ok(serde_json::from_value(created)?)
    }
    
    /// Update an existing note
    async fn update_note(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateNoteInput,
    ) -> Result<Note> {
        let context = ctx.ctx()?;
        
        // Validate input
        input.validate()
            .map_err(|e| GraphQLError::from(e))?;
        
        // Check if at least one field is being updated
        if input.title.is_none() && input.content.is_none() {
            return Err(Error::new("At least one field must be updated"));
        }
        
        // Fetch existing note
        let existing = context.database
            .read("notes", &id.to_string())
            .await
            .map_err(|e| e.into())?
            .ok_or_else(|| Error::new("Note not found"))?;
        
        let mut note: Note = serde_json::from_value(existing)?;
        
        // Check authorization
        #[cfg(not(feature = "demo"))]
        {
            let session = context.require_auth()?;
            if session.user_id != note.author {
                return Err(Error::new("Not authorized to update this note"));
            }
        }
        
        // Apply updates
        if let Some(title) = input.title {
            note.title = title;
        }
        if let Some(content) = input.content {
            note.content = content;
        }
        note.updated_at = chrono::Utc::now();
        
        // Save to database
        context.database
            .update("notes", &id.to_string(), serde_json::to_value(&note)?)
            .await
            .map_err(|e| e.into())?;
        
        Ok(note)
    }
    
    /// Delete a note
    async fn delete_note(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<bool> {
        let context = ctx.ctx()?;
        
        // Fetch note to check ownership
        let existing = context.database
            .read("notes", &id.to_string())
            .await
            .map_err(|e| e.into())?
            .ok_or_else(|| Error::new("Note not found"))?;
        
        let note: Note = serde_json::from_value(existing)?;
        
        // Check authorization
        #[cfg(not(feature = "demo"))]
        {
            let session = context.require_auth()?;
            if session.user_id != note.author {
                return Err(Error::new("Not authorized to delete this note"));
            }
        }
        
        // Delete from database
        context.database
            .delete("notes", &id.to_string())
            .await
            .map_err(|e| e.into())?;
        
        Ok(true)
    }
}
```

---
## 🛑 CHECKPOINT 2: Query and Mutation Resolvers Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Implemented all query resolvers (note, notes, notesByAuthor, searchNotes, health)
2. Created pagination with connection types
3. Implemented all mutations (createNote, updateNote, deleteNote)
4. Added input validation with Garde
5. Included authorization checks (skipped in demo mode)
6. Written comprehensive tests for all operations
7. Added proper error handling and messages
8. Documented all GraphQL operations
9. Committed all work with message: "Checkpoint 2: Query and mutation resolvers complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 2
- Your git commit hash
- GraphQL playground showing working queries/mutations

**DO NOT PROCEED** until you receive explicit approval.

---

### 3.4 Subscription Implementation (3-4 work units)

**Work Unit Context:**
- **Complexity**: High - WebSocket handling and event streaming
- **Scope**: ~700 lines across 4-5 files
- **Key Components**:
  - Subscription root type (~150 lines)
  - WebSocket transport setup (~200 lines)
  - Event broadcasting system (~200 lines)
  - Connection management (~100 lines)
  - Subscription tests (~150 lines)
- **Algorithms**: Pub/sub pattern, connection pooling, event filtering

#### Task 3.4.1: Write Subscription Tests First
Create tests for real-time subscriptions:
```rust
#[cfg(test)]
mod subscription_tests {
    use super::*;
    use futures_util::StreamExt;
    
    #[tokio::test]
    async fn test_note_created_subscription() {
        let schema = create_test_schema().await;
        
        // Start subscription
        let subscription = r#"
            subscription {
                noteCreated {
                    id
                    title
                    author
                }
            }
        "#;
        
        let mut stream = schema.execute_stream(subscription);
        
        // Create a note in another task
        let schema_clone = schema.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            let mutation = r#"
                mutation {
                    createNote(input: {
                        title: "Subscription Test"
                        content: "Testing subscriptions"
                        author: "test_user"
                    }) {
                        id
                    }
                }
            "#;
            
            schema_clone.execute(mutation).await;
        });
        
        // Receive subscription event
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            stream.next()
        ).await;
        
        assert!(result.is_ok());
        let response = result.unwrap().unwrap();
        let data = response.data.into_json().unwrap();
        assert_eq!(data["noteCreated"]["title"], "Subscription Test");
    }
    
    #[tokio::test]
    async fn test_note_updated_subscription_with_filter() {
        let schema = create_test_schema().await;
        let note_id = create_test_note(&schema).await;
        
        // Subscribe to updates for specific note
        let subscription = format!(r#"
            subscription {{
                noteUpdated(id: "{}") {{
                    id
                    title
                    updatedAt
                }}
            }}
        "#, note_id);
        
        let mut stream = schema.execute_stream(subscription);
        
        // Update the note
        let schema_clone = schema.clone();
        let note_id_clone = note_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            let mutation = format!(r#"
                mutation {{
                    updateNote(id: "{}", input: {{
                        title: "Updated via subscription"
                    }}) {{
                        id
                    }}
                }}
            "#, note_id_clone);
            
            schema_clone.execute(mutation).await;
        });
        
        // Should receive update event
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            stream.next()
        ).await;
        
        assert!(result.is_ok());
        let response = result.unwrap().unwrap();
        let data = response.data.into_json().unwrap();
        assert_eq!(data["noteUpdated"]["title"], "Updated via subscription");
    }
}
```

#### Task 3.4.2: Implement Event Broadcasting
Create pub/sub system for events:
```rust
// src/graphql/subscriptions/broadcaster.rs
use tokio::sync::broadcast;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum NoteEvent {
    Created(Note),
    Updated(Note),
    Deleted(String), // Note ID
}

pub struct EventBroadcaster {
    sender: broadcast::Sender<NoteEvent>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<NoteEvent> {
        self.sender.subscribe()
    }
    
    pub fn broadcast(&self, event: NoteEvent) -> Result<()> {
        self.sender.send(event)
            .map_err(|_| Error::new("No active subscribers"))?;
        Ok(())
    }
    
    pub fn active_receivers(&self) -> usize {
        self.sender.receiver_count()
    }
}

// Add to GraphQL context
impl GraphQLContext {
    pub fn broadcaster(&self) -> &EventBroadcaster {
        &self.broadcaster
    }
}
```

#### Task 3.4.3: Implement Subscription Root Type
Create subscription resolvers:
```rust
// src/graphql/resolvers/subscriptions.rs
use async_graphql::*;
use futures_util::Stream;
use tokio_stream::StreamExt;

#[derive(Default)]
pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to new notes
    async fn note_created(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Note>> {
        let context = ctx.ctx()?;
        
        // Check subscription limits
        if context.broadcaster().active_receivers() >= MAX_SUBSCRIPTIONS_PER_INSTANCE {
            return Err(Error::new("Subscription limit reached"));
        }
        
        let mut receiver = context.broadcaster().subscribe();
        
        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                if let NoteEvent::Created(note) = event {
                    yield note;
                }
            }
        })
    }
    
    /// Subscribe to updates on a specific note or all notes
    async fn note_updated(
        &self,
        ctx: &Context<'_>,
        id: Option<ID>,
    ) -> Result<impl Stream<Item = Note>> {
        let context = ctx.ctx()?;
        
        let filter_id = id.map(|id| id.to_string());
        let mut receiver = context.broadcaster().subscribe();
        
        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                if let NoteEvent::Updated(note) = event {
                    // Apply filter if ID provided
                    if let Some(ref filter) = filter_id {
                        if note.id.as_ref().map(|id| id.to_string()) == Some(filter.clone()) {
                            yield note;
                        }
                    } else {
                        yield note;
                    }
                }
            }
        })
    }
    
    /// Subscribe to note deletions
    async fn note_deleted(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = String>> {
        let context = ctx.ctx()?;
        
        let mut receiver = context.broadcaster().subscribe();
        
        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                if let NoteEvent::Deleted(id) = event {
                    yield id;
                }
            }
        })
    }
}

// Update mutations to broadcast events
impl Mutation {
    // In create_note, after successful creation:
    context.broadcaster().broadcast(NoteEvent::Created(created_note.clone()))?;
    
    // In update_note, after successful update:
    context.broadcaster().broadcast(NoteEvent::Updated(updated_note.clone()))?;
    
    // In delete_note, after successful deletion:
    context.broadcaster().broadcast(NoteEvent::Deleted(id.to_string()))?;
}
```

#### Task 3.4.4: Set Up WebSocket Transport
Configure WebSocket for subscriptions:
```rust
// src/graphql/transport.rs
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::{WebSocketUpgrade, State},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

pub fn graphql_routes(schema: AppSchema) -> Router {
    Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .route("/graphql/ws", get(graphql_ws_handler))
        .route("/schema", get(schema_handler))
        .with_state(schema)
}

async fn graphql_playground() -> impl IntoResponse {
    // Only available in demo mode
    if !is_demo_mode() {
        return StatusCode::NOT_FOUND.into_response();
    }
    
    Html(playground_source(
        GraphQLPlaygroundConfig::new("/graphql")
            .subscription_endpoint("/graphql/ws")
    ))
}

async fn graphql_handler(
    State(schema): State<AppSchema>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = req.into_inner();
    
    // Create context from request
    let context = create_context_from_request(&headers).await;
    request = request.data(context);
    
    schema.execute(request).await.into()
}

async fn graphql_ws_handler(
    State(schema): State<AppSchema>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let context = create_context_from_request(&headers).await;
    
    ws.protocols(["graphql-ws"])
        .on_upgrade(move |socket| {
            GraphQLSubscription::new(socket)
                .on_connection_init(|_| async {
                    Ok(context)
                })
                .serve(schema)
                .await
        })
}

async fn schema_handler(State(schema): State<AppSchema>) -> impl IntoResponse {
    // Only available in demo mode
    if !is_demo_mode() {
        return StatusCode::NOT_FOUND.into_response();
    }
    
    schema.sdl()
}
```

### 3.5 Security & Performance (2-3 work units)

**Work Unit Context:**
- **Complexity**: High - Complex security validations and performance optimizations
- **Scope**: ~600 lines across 4-5 files
- **Key Components**:
  - Query depth limiting (~150 lines)
  - Complexity calculation (~150 lines)
  - DataLoader implementation (~200 lines)
  - Rate limiting (~100 lines)
  - Security tests (~100 lines)
- **Algorithms**: Tree traversal for depth, cost calculation, batch loading

#### Task 3.5.1: Write Security Tests First
Test depth and complexity limits:
```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_query_depth_limit() {
        let schema = create_schema_with_limits(5, 1000);
        
        // Query with depth 6 (should fail)
        let deep_query = r#"
            query {
                notes {
                    edges {
                        node {
                            relatedNotes {
                                edges {
                                    node {
                                        title
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;
        
        let result = schema.execute(deep_query).await;
        assert!(result.errors.len() > 0);
        assert!(result.errors[0].message.contains("depth"));
    }
    
    #[tokio::test]
    async fn test_query_complexity_limit() {
        let schema = create_schema_with_limits(15, 100);
        
        // Query requesting 101 items (complexity > 100)
        let complex_query = r#"
            query {
                notes(limit: 101) {
                    edges {
                        node {
                            id
                            title
                            content
                        }
                    }
                }
            }
        "#;
        
        let result = schema.execute(complex_query).await;
        assert!(result.errors.len() > 0);
        assert!(result.errors[0].message.contains("complexity"));
    }
    
    #[tokio::test]
    async fn test_introspection_disabled_in_production() {
        std::env::set_var("ENVIRONMENT", "production");
        let schema = create_schema();
        
        let introspection_query = r#"
            query {
                __schema {
                    types {
                        name
                    }
                }
            }
        "#;
        
        let result = schema.execute(introspection_query).await;
        assert!(result.errors.len() > 0);
        
        std::env::remove_var("ENVIRONMENT");
    }
}
```

#### Task 3.5.2: Implement Query Depth Limiting
Add custom depth limiter:
```rust
// src/graphql/security/depth.rs
use async_graphql::{ValidationResult, Visitor, VisitorContext};
use async_graphql::parser::types::*;

pub struct DepthLimit {
    max_depth: usize,
}

impl DepthLimit {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

impl Visitor for DepthLimit {
    fn enter_field(
        &mut self,
        ctx: &mut VisitorContext<'_>,
        field: &Field,
    ) -> Result<(), String> {
        let depth = ctx.field_stack.len();
        
        if depth > self.max_depth {
            ctx.report_error(
                vec![field.pos],
                format!(
                    "Query depth of {} exceeds maximum allowed depth of {}",
                    depth, self.max_depth
                ),
            );
        }
        
        Ok(())
    }
}

// Add to schema builder
builder = builder.validation_mode(ValidationMode::Custom(Box::new(DepthLimit::new(15))));
```

#### Task 3.5.3: Implement Complexity Calculation
Calculate query complexity:
```rust
// src/graphql/security/complexity.rs
#[derive(Debug)]
pub struct ComplexityCalculator {
    max_complexity: usize,
    current_complexity: usize,
}

impl ComplexityCalculator {
    pub fn new(max_complexity: usize) -> Self {
        Self {
            max_complexity,
            current_complexity: 0,
        }
    }
    
    pub fn add_field_cost(&mut self, cost: usize) -> Result<(), String> {
        self.current_complexity += cost;
        
        if self.current_complexity > self.max_complexity {
            Err(format!(
                "Query complexity of {} exceeds maximum allowed complexity of {}",
                self.current_complexity, self.max_complexity
            ))
        } else {
            Ok(())
        }
    }
}

// Field complexity definitions
impl Query {
    #[graphql(complexity = "1")]
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        // ...
    }
    
    #[graphql(complexity = "limit.unwrap_or(10) as usize")]
    async fn notes(&self, ctx: &Context<'_>, limit: Option<i32>) -> Result<NotesConnection> {
        // ...
    }
    
    #[graphql(complexity = "50")] // Expensive search operation
    async fn search_notes(&self, ctx: &Context<'_>, query: String) -> Result<Vec<Note>> {
        // ...
    }
}
```

#### Task 3.5.4: Implement DataLoader
Prevent N+1 queries with batch loading:
```rust
// src/graphql/dataloaders/note_loader.rs
use async_graphql::dataloader::{DataLoader, Loader};
use std::collections::HashMap;

pub struct NoteLoader {
    database: Arc<dyn DatabaseService>,
}

#[async_trait::async_trait]
impl Loader<String> for NoteLoader {
    type Value = Note;
    type Error = Arc<AppError>;
    
    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        // Batch load notes
        let query = Query::new()
            .filter("id IN $ids")
            .bind("ids", keys);
            
        let notes = self.database
            .query("notes", query)
            .await
            .map_err(|e| Arc::new(e.into()))?;
        
        // Convert to HashMap for DataLoader
        let map: HashMap<String, Note> = notes
            .into_iter()
            .map(|value| {
                let note: Note = serde_json::from_value(value).unwrap();
                (note.id.clone().unwrap().to_string(), note)
            })
            .collect();
        
        Ok(map)
    }
}

// Add to context
impl GraphQLContext {
    pub fn note_loader(&self) -> &DataLoader<NoteLoader> {
        &self.note_loader
    }
}

// Use in resolvers
impl Query {
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        let context = ctx.ctx()?;
        
        // Use DataLoader instead of direct database access
        context.note_loader()
            .load_one(id.to_string())
            .await
            .map_err(|e| e.into())
    }
}
```

---
## 🛑 CHECKPOINT 3: Subscriptions and Security Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Implemented all subscription types (noteCreated, noteUpdated, noteDeleted)
2. Created event broadcasting system
3. Set up WebSocket transport
4. Added query depth limiting (default: 15)
5. Implemented complexity calculation (default: 1000)
6. Created DataLoader for N+1 prevention
7. Disabled introspection in production
8. Written comprehensive security tests
9. Committed all work with message: "Checkpoint 3: Subscriptions and security complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 3
- Your git commit hash
- Demonstration of working subscriptions
- Evidence of security limits working

**DO NOT PROCEED** until you receive explicit approval.

---

### 3.6 Integration & Metrics (2 work units)

**Work Unit Context:**
- **Complexity**: Medium - Integrating all components
- **Scope**: ~400 lines across 3-4 files
- **Key Components**:
  - GraphQL route integration (~100 lines)
  - Metrics collection (~150 lines)
  - Integration tests (~100 lines)
  - Documentation updates (~50 lines)
- **No complex algorithms** - Just integration and metrics

#### Task 3.6.1: Write Integration Tests
Test complete GraphQL system:
```rust
// tests/graphql_integration.rs
#[tokio::test]
async fn test_complete_graphql_flow() {
    let app = create_test_app().await;
    
    // Create a note
    let create_response = app
        .post("/graphql")
        .json(&json!({
            "query": r#"
                mutation {
                    createNote(input: {
                        title: "Integration Test"
                        content: "Testing complete flow"
                        author: "test_user"
                    }) {
                        id
                        title
                    }
                }
            "#
        }))
        .send()
        .await;
    
    assert_eq!(create_response.status(), 200);
    let data: serde_json::Value = create_response.json().await;
    let note_id = data["data"]["createNote"]["id"].as_str().unwrap();
    
    // Query the note
    let query_response = app
        .post("/graphql")
        .json(&json!({
            "query": format!(r#"
                query {{
                    note(id: "{}") {{
                        title
                        content
                    }}
                }}
            "#, note_id)
        }))
        .send()
        .await;
    
    assert_eq!(query_response.status(), 200);
    
    // Update the note
    let update_response = app
        .post("/graphql")
        .json(&json!({
            "query": format!(r#"
                mutation {{
                    updateNote(id: "{}", input: {{
                        title: "Updated Title"
                    }}) {{
                        title
                        updatedAt
                    }}
                }}
            "#, note_id)
        }))
        .send()
        .await;
    
    assert_eq!(update_response.status(), 200);
    
    // Delete the note
    let delete_response = app
        .post("/graphql")
        .json(&json!({
            "query": format!(r#"
                mutation {{
                    deleteNote(id: "{}")
                }}
            "#, note_id)
        }))
        .send()
        .await;
    
    assert_eq!(delete_response.status(), 200);
}

#[tokio::test]
async fn test_graphql_metrics() {
    let app = create_test_app().await;
    
    // Execute some queries
    for _ in 0..5 {
        app.post("/graphql")
            .json(&json!({
                "query": "query { health { status } }"
            }))
            .send()
            .await;
    }
    
    // Check metrics
    let metrics_response = app.get("/metrics").send().await;
    let metrics_body = metrics_response.text().await;
    
    assert!(metrics_body.contains("graphql_request_total"));
    assert!(metrics_body.contains("graphql_request_duration_seconds"));
    assert!(metrics_body.contains("graphql_field_resolution_duration_seconds"));
}
```

#### Task 3.6.2: Add GraphQL Metrics
Implement Prometheus metrics:
```rust
// src/graphql/metrics.rs
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge,
    HistogramVec, IntCounterVec, IntGauge,
};

lazy_static! {
    static ref GRAPHQL_REQUEST_TOTAL: IntCounterVec = register_int_counter_vec!(
        "graphql_request_total",
        "Total number of GraphQL requests",
        &["operation_type", "operation_name", "status"]
    ).unwrap();
    
    static ref GRAPHQL_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "graphql_request_duration_seconds",
        "GraphQL request duration in seconds",
        &["operation_type", "operation_name"]
    ).unwrap();
    
    static ref GRAPHQL_FIELD_DURATION: HistogramVec = register_histogram_vec!(
        "graphql_field_resolution_duration_seconds",
        "GraphQL field resolution duration in seconds",
        &["type_name", "field_name"]
    ).unwrap();
    
    static ref GRAPHQL_ACTIVE_SUBSCRIPTIONS: IntGauge = register_int_gauge!(
        "graphql_active_subscriptions",
        "Number of active GraphQL subscriptions"
    ).unwrap();
}

// Metrics extension
pub struct MetricsExtension;

#[async_trait::async_trait]
impl Extension for MetricsExtension {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let operation_type = ctx.operation_type.to_string();
        let operation_name = ctx.operation_name.unwrap_or("unnamed");
        
        let timer = GRAPHQL_REQUEST_DURATION
            .with_label_values(&[&operation_type, operation_name])
            .start_timer();
        
        let response = next.run(ctx).await;
        
        timer.observe_duration();
        
        let status = if response.is_err() { "error" } else { "success" };
        GRAPHQL_REQUEST_TOTAL
            .with_label_values(&[&operation_type, operation_name, status])
            .inc();
        
        response
    }
}
```

#### Task 3.6.3: Integrate with Main Server
Add GraphQL to the main application:
```rust
// In src/main.rs
use crate::graphql::{create_schema, graphql_routes};

#[tokio::main]
async fn main() -> Result<()> {
    // ... existing initialization ...
    
    // Create GraphQL schema
    let schema = create_schema()
        .data(database.clone())
        .data(event_broadcaster)
        .extension(MetricsExtension)
        .finish();
    
    // Build app with GraphQL routes
    let app = Router::new()
        .merge(health_routes())
        .merge(graphql_routes(schema))
        .route("/metrics", get(metrics_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);
    
    // ... rest of server setup ...
}
```

#### Task 3.6.4: Create Verification Script
Add Phase 3 verification:
```bash
#!/bin/bash
# scripts/verify-phase-3.sh
set -e

echo "=== Phase 3 Verification ==="

# 1. Check compilation
echo "✓ Checking compilation..."
just build

# 2. Run unit tests
echo "✓ Running GraphQL tests..."
just test-graphql

# 3. Start server
echo "✓ Starting server..."
cargo run --features demo &
SERVER_PID=$!
sleep 5

# 4. Test GraphQL endpoint
echo "✓ Testing GraphQL queries..."
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ health { status } }"}' | jq .

# 5. Test mutations
echo "✓ Testing mutations..."
RESULT=$(curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { createNote(input: { title: \"Test\", content: \"Test\", author: \"test\" }) { id } }"}')
NOTE_ID=$(echo $RESULT | jq -r .data.createNote.id)

# 6. Test subscriptions
echo "✓ Testing WebSocket subscriptions..."
# Use wscat or similar to test WebSocket

# 7. Check metrics
echo "✓ Checking GraphQL metrics..."
curl -s http://localhost:8080/metrics | grep graphql_

# 8. Test playground (demo mode)
echo "✓ Testing GraphQL playground..."
curl -s http://localhost:8080/graphql | grep -q "GraphQL Playground"

# 9. Test schema export (demo mode)
echo "✓ Testing schema export..."
curl -s http://localhost:8080/schema | grep -q "type Query"

# 10. Cleanup
kill $SERVER_PID

echo "=== All Phase 3 checks passed! ==="
```

---
## 🛑 CHECKPOINT 4: Complete Phase 3 System Review

**STOP HERE FOR FINAL EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Integrated GraphQL with main server
2. All routes properly configured
3. Metrics collection working
4. Complete integration test suite
5. GraphQL playground accessible (demo mode)
6. Schema export working (demo mode)
7. All security controls enforced:
   - Query depth limiting
   - Complexity calculation
   - Introspection disabled in production
8. Performance optimizations:
   - DataLoader preventing N+1 queries
   - Subscription limits enforced
9. Documentation updated
10. Committed all work with message: "Checkpoint 4: Phase 3 complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 4
- Your git commit hash
- Output from `scripts/verify-phase-3.sh`
- GraphQL playground demonstration
- Metrics showing GraphQL operations

**Review Checklist for Reviewer**:

### GraphQL Implementation
- [ ] All queries working (note, notes, notesByAuthor, searchNotes, health)
- [ ] All mutations working (createNote, updateNote, deleteNote)
- [ ] All subscriptions working (noteCreated, noteUpdated, noteDeleted)
- [ ] Pagination implemented correctly
- [ ] Input validation with clear errors

### Security Controls
- [ ] Query depth limited (configurable, default 15)
- [ ] Query complexity limited (configurable, default 1000)
- [ ] Introspection disabled in production
- [ ] Playground only in demo mode
- [ ] Authorization checks in place

### Performance
- [ ] DataLoader prevents N+1 queries
- [ ] Subscription limits enforced
- [ ] Query timeouts implemented
- [ ] Metrics track performance

### Integration
- [ ] GraphQL routes integrated with server
- [ ] Context properly propagated
- [ ] Errors mapped correctly
- [ ] Health check includes GraphQL status

### Testing & Documentation
- [ ] Unit tests for all resolvers
- [ ] Integration tests for complete flows
- [ ] Security tests for limits
- [ ] Performance tests for DataLoader
- [ ] API documentation complete

### Operational Readiness
- [ ] Verification script passes
- [ ] Metrics exposed properly
- [ ] Logs include GraphQL operations
- [ ] Configuration documented
- [ ] All Phase 3 "Done Criteria" met

**Final Approval Required**: The reviewer must explicitly approve before Phase 4 can begin.

---

## Final Phase 3 Deliverables

Before marking Phase 3 complete, ensure these artifacts exist:

1. **Documentation**
   - [ ] GraphQL API documentation
   - [ ] Schema documentation (auto-generated)
   - [ ] Security configuration guide
   - [ ] Performance tuning guide

2. **Tests**
   - [ ] Unit tests for all resolvers
   - [ ] Integration tests for complete flows
   - [ ] WebSocket subscription tests
   - [ ] Security limit tests
   - [ ] Performance/DataLoader tests

3. **Scripts**
   - [ ] `scripts/verify-phase-3.sh` - Automated verification
   - [ ] `scripts/test-graphql.sh` - GraphQL-specific tests
   - [ ] `scripts/load-test-graphql.sh` - Performance testing

4. **Metrics**
   - [ ] Request counts by operation
   - [ ] Request duration histograms
   - [ ] Field resolution times
   - [ ] Active subscription gauge

## Next Steps

Once all checkpoints pass:
1. Commit with message: "Complete Phase 3: GraphQL Implementation"
2. Tag as `v0.3.0-phase3`
3. Create PR for review if working in team
4. Document any deviations from original plan
5. Begin Phase 4 planning (Authorization & Authentication)

## Important Notes

- **DO NOT PROCEED** past a checkpoint until all verification steps pass
- **MAINTAIN** security-first approach - all limits must be enforced
- **DOCUMENT** GraphQL-specific configuration options
- **TEST** edge cases thoroughly - malicious queries, deep nesting, etc.
- **MONITOR** subscription connections to prevent resource exhaustion

## Troubleshooting Guide

### Common Issues and Solutions

#### GraphQL Schema Issues

**Issue**: Schema fails to build
**Solution**: 
- Check all types are properly defined
- Verify no circular dependencies
- Ensure all resolvers return correct types

**Issue**: Subscriptions not working
**Solution**:
- Verify WebSocket route is configured
- Check event broadcaster is initialized
- Ensure mutations broadcast events
- Test WebSocket connection directly

#### Security Issues

**Issue**: Deep queries not rejected
**Solution**:
- Verify depth limit is configured
- Check visitor is added to schema
- Test with deeper query

**Issue**: Complexity not calculated
**Solution**:
- Ensure complexity attributes on fields
- Verify calculator is initialized
- Check max complexity is set

#### Performance Issues

**Issue**: N+1 queries detected
**Solution**:
- Implement DataLoader for relationship
- Batch database queries
- Check resolver implementations

**Issue**: Slow subscription delivery
**Solution**:
- Check broadcaster capacity
- Monitor active subscriptions
- Review event filtering logic

### Debugging Tips

1. **Enable GraphQL debug mode**: `GRAPHQL_DEBUG=true`
2. **Log all operations**: Add logging extension
3. **Monitor WebSocket connections**: Use browser dev tools
4. **Test with GraphQL playground**: Available in demo mode
5. **Check metrics**: GraphQL-specific metrics at `/metrics`

### Useful Resources

- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)
- [DataLoader Pattern](https://github.com/graphql/dataloader)
- [GraphQL Security](https://www.howtographql.com/advanced/4-security/)

---
*This work plan follows the same structure and practices as Phase 1 & 2, adapted for GraphQL implementation.*
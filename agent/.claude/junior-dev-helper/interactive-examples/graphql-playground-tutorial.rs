/// Interactive GraphQL Playground Tutorial
/// 
/// This example creates a simple GraphQL server that you can interact with
/// to learn GraphQL concepts hands-on.
/// 
/// Run with: cargo run --example graphql-playground-tutorial --features demo

use async_graphql::{
    Context, EmptySubscription, Error, Object, Result, Schema, SimpleObject, ID,
    InputObject, Variables, Request,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Our simple in-memory database
#[derive(Default)]
struct Database {
    notes: RwLock<HashMap<String, Note>>,
    users: RwLock<HashMap<String, User>>,
    next_id: RwLock<u64>,
}

impl Database {
    async fn create_note(&self, input: CreateNoteInput, author_id: String) -> Result<Note> {
        let mut notes = self.notes.write().await;
        let mut next_id = self.next_id.write().await;
        
        *next_id += 1;
        let id = next_id.to_string();
        
        let note = Note {
            id: id.clone(),
            title: input.title,
            content: input.content,
            author_id,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        
        notes.insert(id, note.clone());
        Ok(note)
    }
    
    async fn get_note(&self, id: &str) -> Option<Note> {
        self.notes.read().await.get(id).cloned()
    }
    
    async fn list_notes(&self, limit: Option<usize>) -> Vec<Note> {
        let notes = self.notes.read().await;
        let limit = limit.unwrap_or(20).min(100);
        
        notes.values()
            .take(limit)
            .cloned()
            .collect()
    }
    
    async fn get_user(&self, id: &str) -> Option<User> {
        self.users.read().await.get(id).cloned()
    }
}

/// Our GraphQL types
#[derive(Clone, SimpleObject, Serialize, Deserialize)]
struct Note {
    id: String,
    title: String,
    content: String,
    author_id: String,
    created_at: String,
}

#[derive(Clone, SimpleObject)]
struct User {
    id: String,
    name: String,
    email: String,
}

#[derive(InputObject)]
struct CreateNoteInput {
    #[graphql(validator(min_length = 1, max_length = 200))]
    title: String,
    
    #[graphql(validator(min_length = 1, max_length = 10000))]
    content: String,
}

/// Root Query type
struct Query;

#[Object]
impl Query {
    /// Get a note by ID
    /// 
    /// Example query:
    /// ```graphql
    /// query {
    ///   note(id: "1") {
    ///     id
    ///     title
    ///     content
    ///   }
    /// }
    /// ```
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        let db = ctx.data::<Arc<Database>>()?;
        Ok(db.get_note(id.as_str()).await)
    }
    
    /// List all notes with optional limit
    /// 
    /// Example query:
    /// ```graphql
    /// query {
    ///   notes(limit: 10) {
    ///     id
    ///     title
    ///   }
    /// }
    /// ```
    async fn notes(&self, ctx: &Context<'_>, limit: Option<i32>) -> Result<Vec<Note>> {
        let db = ctx.data::<Arc<Database>>()?;
        Ok(db.list_notes(limit.map(|l| l as usize)).await)
    }
    
    /// Get current user (demo always returns Alice)
    async fn me(&self) -> User {
        User {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        }
    }
    
    /// Health check endpoint
    async fn health(&self) -> &'static str {
        "OK"
    }
}

/// Root Mutation type
struct Mutation;

#[Object]
impl Mutation {
    /// Create a new note
    /// 
    /// Example mutation:
    /// ```graphql
    /// mutation {
    ///   createNote(input: {
    ///     title: "My Note"
    ///     content: "Note content"
    ///   }) {
    ///     id
    ///     title
    ///   }
    /// }
    /// ```
    async fn create_note(
        &self,
        ctx: &Context<'_>,
        input: CreateNoteInput,
    ) -> Result<Note> {
        let db = ctx.data::<Arc<Database>>()?;
        
        // In real app, get from auth context
        let author_id = "alice".to_string();
        
        db.create_note(input, author_id).await
    }
    
    /// Delete a note by ID
    async fn delete_note(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let db = ctx.data::<Arc<Database>>()?;
        let mut notes = db.notes.write().await;
        
        Ok(notes.remove(id.as_str()).is_some())
    }
}

/// GraphQL handler
async fn graphql_handler(
    Extension(schema): Extension<Schema<Query, Mutation, EmptySubscription>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// Serve GraphQL playground UI
async fn playground() -> impl IntoResponse {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>GraphQL Playground Tutorial</title>
        <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
        <style>
            body { margin: 0; height: 100vh; }
            #tutorial { 
                position: fixed; 
                right: 20px; 
                top: 20px; 
                background: white; 
                border: 1px solid #ddd;
                padding: 20px;
                max-width: 400px;
                max-height: 80vh;
                overflow-y: auto;
                z-index: 1000;
            }
            .example { 
                background: #f5f5f5; 
                padding: 10px; 
                margin: 10px 0;
                cursor: pointer;
            }
            .example:hover { background: #e0e0e0; }
        </style>
    </head>
    <body>
        <div id="tutorial">
            <h2>GraphQL Tutorial</h2>
            <p>Click any example to try it!</p>
            
            <h3>1. Basic Query</h3>
            <div class="example" onclick='setQuery(`{
  health
}`)'>
{
  health
}</div>

            <h3>2. Query with Fields</h3>
            <div class="example" onclick='setQuery(`{
  me {
    id
    name
    email
  }
}`)'>
{
  me {
    id
    name
    email
  }
}</div>

            <h3>3. Query with Arguments</h3>
            <div class="example" onclick='setQuery(`{
  notes(limit: 5) {
    id
    title
    createdAt
  }
}`)'>
{
  notes(limit: 5) {
    id
    title
    createdAt
  }
}</div>

            <h3>4. Create a Note</h3>
            <div class="example" onclick='setQuery(`mutation {
  createNote(input: {
    title: "My First Note"
    content: "This is the content"
  }) {
    id
    title
    content
    createdAt
  }
}`)'>
mutation {
  createNote(input: {
    title: "My First Note"
    content: "This is the content"
  }) {
    id
    title
    content
    createdAt
  }
}</div>

            <h3>5. Query with Variables</h3>
            <div class="example" onclick='setQueryWithVars(
`query GetNote($id: ID!) {
  note(id: $id) {
    id
    title
    content
  }
}`,
`{
  "id": "1"
}`)'>
query GetNote($id: ID!) {
  note(id: $id) {
    id
    title
    content
  }
}</div>

            <h3>6. Multiple Operations</h3>
            <div class="example" onclick='setQuery(`{
  myProfile: me {
    name
  }
  
  recentNotes: notes(limit: 3) {
    title
    createdAt
  }
}`)'>
{
  myProfile: me {
    name
  }
  
  recentNotes: notes(limit: 3) {
    title
    createdAt
  }
}</div>

            <h3>7. Introspection</h3>
            <div class="example" onclick='setQuery(`{
  __type(name: "Query") {
    fields {
      name
      description
    }
  }
}`)'>
{
  __type(name: "Query") {
    fields {
      name
      description
    }
  }
}</div>
        </div>
        
        <div id="graphiql">Loading...</div>
        
        <script crossorigin src="https://unpkg.com/react@17/umd/react.production.min.js"></script>
        <script crossorigin src="https://unpkg.com/react-dom@17/umd/react-dom.production.min.js"></script>
        <script crossorigin src="https://unpkg.com/graphiql/graphiql.min.js"></script>
        
        <script>
            let graphiqlEditor;
            
            function setQuery(query) {
                if (graphiqlEditor) {
                    graphiqlEditor.setQuery(query);
                    graphiqlEditor.setVariables('');
                }
            }
            
            function setQueryWithVars(query, variables) {
                if (graphiqlEditor) {
                    graphiqlEditor.setQuery(query);
                    graphiqlEditor.setVariables(variables);
                }
            }
            
            const fetcher = GraphiQL.createFetcher({
                url: '/graphql',
            });
            
            const root = ReactDOM.createRoot(document.getElementById('graphiql'));
            const element = React.createElement(GraphiQL, {
                fetcher: fetcher,
                defaultEditorToolsVisibility: true,
                onEditQuery: (newQuery) => console.log('Query changed:', newQuery),
                ref: (ref) => { graphiqlEditor = ref; }
            });
            
            root.render(element);
        </script>
    </body>
    </html>
    "#)
}

/// Initialize demo data
async fn init_demo_data(db: Arc<Database>) {
    // Add demo users
    db.users.write().await.insert(
        "alice".to_string(),
        User {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
    );
    
    // Add demo notes
    let demo_notes = vec![
        ("GraphQL Basics", "GraphQL is a query language for APIs"),
        ("Mutations", "Mutations are used to modify data"),
        ("Subscriptions", "Subscriptions provide real-time updates"),
    ];
    
    for (title, content) in demo_notes {
        db.create_note(
            CreateNoteInput {
                title: title.to_string(),
                content: content.to_string(),
            },
            "alice".to_string(),
        ).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Create database
    let db = Arc::new(Database::default());
    init_demo_data(db.clone()).await;
    
    // Create GraphQL schema
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(db)
        .limit_depth(10)  // Security: limit query depth
        .limit_complexity(100)  // Security: limit query complexity
        .finish();
    
    // Create router
    let app = Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/playground", get(playground))
        .route("/", get(|| async { 
            "GraphQL Playground Tutorial - Navigate to /playground" 
        }))
        .layer(Extension(schema));
    
    println!("🚀 GraphQL Playground Tutorial running at http://localhost:8080/playground");
    println!("📚 This is an interactive tutorial - try the examples in the sidebar!");
    println!();
    println!("Tips:");
    println!("- Click any example to load it");
    println!("- Press Ctrl+Enter to execute queries");
    println!("- Use the Docs tab to explore the schema");
    println!("- Try modifying the examples");
    
    // Start server
    axum::Server::bind(&"127.0.0.1:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tutorial_queries() {
        let db = Arc::new(Database::default());
        init_demo_data(db.clone()).await;
        
        let schema = Schema::build(Query, Mutation, EmptySubscription)
            .data(db)
            .finish();
        
        // Test health query
        let response = schema.execute("{ health }").await;
        assert!(response.errors.is_empty());
        assert_eq!(response.data.get("health").unwrap(), "OK");
        
        // Test notes query
        let response = schema.execute("{ notes { id title } }").await;
        assert!(response.errors.is_empty());
        assert!(response.data.get("notes").unwrap().as_array().unwrap().len() > 0);
    }
}
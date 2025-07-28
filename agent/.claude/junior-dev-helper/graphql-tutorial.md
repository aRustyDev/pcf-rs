# GraphQL Tutorial for Phase 3

## Introduction

GraphQL is a query language for APIs that allows clients to request exactly what data they need. Unlike REST where you might need multiple endpoints, GraphQL provides a single endpoint with a flexible query structure.

## Core Concepts

### 1. Schema Definition Language (SDL)

GraphQL uses a type system to define what data can be queried:

```graphql
type Note {
  id: ID!              # ! means non-nullable
  title: String!
  content: String!
  author: String!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Query {
  note(id: ID!): Note           # Single note by ID
  notes(first: Int): [Note!]!    # List of notes
}

type Mutation {
  createNote(input: CreateNoteInput!): Note!
  updateNote(id: ID!, input: UpdateNoteInput!): Note!
  deleteNote(id: ID!): Boolean!
}

type Subscription {
  noteCreated: Note!
  noteUpdated: Note!
  noteDeleted: ID!
}
```

### 2. Query vs Mutation vs Subscription

- **Query**: Read operations (GET in REST)
- **Mutation**: Write operations (POST/PUT/DELETE in REST)
- **Subscription**: Real-time updates over WebSocket

### 3. Resolvers

Resolvers are functions that return data for each field:

```rust
// In async-graphql, resolvers are methods on structs
#[Object]
impl Query {
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        // Your logic to fetch the note
    }
}
```

## async-graphql Basics

### Setting Up a Schema

```rust
use async_graphql::{Schema, EmptyMutation, EmptySubscription};

// Create schema with Query, Mutation, and Subscription types
let schema = Schema::build(Query, Mutation, Subscription)
    .data(database_service)  // Add context data
    .data(auth_service)
    .finish();
```

### Context Pattern

Context provides access to shared resources:

```rust
// Accessing context in resolvers
async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
    // Get database from context
    let db = ctx.data::<Arc<dyn DatabaseService>>()?;
    
    // Get auth context
    let auth = ctx.data::<AuthContext>()?;
    
    // Your logic here
}
```

## Common Patterns

### 1. Error Handling

GraphQL errors should be user-friendly:

```rust
// Good
Err(Error::new("Note not found"))

// Better - with error extensions
Err(Error::new("Note not found")
    .extend_with(|_, e| e.set("code", "NOT_FOUND")))

// Avoid exposing internals
// Bad: Err(Error::new("Database error: connection timeout at 192.168.1.1"))
```

### 2. Pagination (Relay Cursor-Based)

```rust
use async_graphql::connection::*;

async fn notes(
    &self,
    ctx: &Context<'_>,
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
) -> Result<Connection<String, Note>> {
    // Relay-style pagination
    query(
        after,
        before,
        first,
        last,
        |after, before, first, last| async move {
            // Your pagination logic
            let edges = vec![];
            let has_previous = false;
            let has_next = false;
            
            Ok(Connection::new(has_previous, has_next))
        }
    ).await
}
```

### 3. Input Validation

```rust
#[derive(InputObject)]
pub struct CreateNoteInput {
    #[graphql(validator(min_length = 1, max_length = 200))]
    title: String,
    
    #[graphql(validator(min_length = 1))]
    content: String,
}
```

## Security Considerations

### 1. Query Depth Limiting

Prevent deeply nested queries:

```rust
// Configuration
let schema = schema
    .limit_depth(15)  // Maximum nesting depth
    .limit_complexity(1000);  // Maximum query complexity
```

### 2. N+1 Query Prevention

Use DataLoader to batch database queries:

```rust
// Without DataLoader - N+1 problem
query {
  notes {
    author {
      name  # Each note triggers a separate user query
    }
  }
}

// With DataLoader - batched
let loader = DataLoader::new(UserLoader::new(db), tokio::spawn);
ctx.data::<DataLoader<UserLoader>>()?.load_one(author_id).await
```

### 3. Authorization in Resolvers

Always check permissions:

```rust
async fn update_note(&self, ctx: &Context<'_>, id: ID, input: UpdateNoteInput) -> Result<Note> {
    // Check authentication
    let auth = ctx.data::<AuthContext>()?;
    if !auth.is_authenticated() {
        return Err(Error::new("Authentication required")
            .extend_with(|_, e| e.set("code", "UNAUTHORIZED")));
    }
    
    // Check authorization
    let db = ctx.data::<Arc<dyn DatabaseService>>()?;
    let note = db.read("notes", &id.to_string()).await?;
    
    if note.author != auth.user_id {
        return Err(Error::new("Not authorized to update this note")
            .extend_with(|_, e| e.set("code", "FORBIDDEN")));
    }
    
    // Perform update
}
```

## Testing GraphQL

### 1. Unit Testing Resolvers

```rust
#[tokio::test]
async fn test_note_query() {
    let schema = create_test_schema();
    
    let query = r#"
        query {
            note(id: "123") {
                id
                title
                content
            }
        }
    "#;
    
    let response = schema.execute(query).await;
    assert!(response.errors.is_empty());
    assert_eq!(response.data.get("note").get("id"), "123");
}
```

### 2. Testing with Variables

```rust
let query = r#"
    query GetNote($id: ID!) {
        note(id: $id) {
            title
        }
    }
"#;

let variables = Variables::from_value(json!({
    "id": "123"
}));

let request = Request::new(query).variables(variables);
let response = schema.execute(request).await;
```

### 3. Testing Subscriptions

```rust
#[tokio::test]
async fn test_note_created_subscription() {
    let schema = create_test_schema();
    
    let subscription = r#"
        subscription {
            noteCreated {
                id
                title
            }
        }
    "#;
    
    let mut stream = schema.execute_stream(subscription);
    
    // Trigger event
    create_note_in_system().await;
    
    // Check subscription received event
    let event = stream.next().await.unwrap();
    assert!(event.errors.is_empty());
}
```

## Common Gotchas

1. **Forgetting `await`**: All resolvers are async
2. **Not handling `Option<T>`**: Database might return None
3. **Exposing internal errors**: Always map errors to user-friendly messages
4. **Missing context data**: Ensure all required services are added to schema
5. **Infinite loops in schema**: Be careful with circular references

## Next Steps

1. Read the [async-graphql book](https://async-graphql.github.io/async-graphql/en/index.html)
2. Experiment with the GraphQL Playground at `http://localhost:8080/graphql`
3. Study the example implementations in `.spec/examples/`
4. Practice writing tests first (TDD approach)

Remember: GraphQL is just a specification. The concepts transfer across languages and frameworks!
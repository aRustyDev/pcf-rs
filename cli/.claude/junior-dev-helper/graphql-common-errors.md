# Common GraphQL Errors and Solutions

## Compilation Errors

### 1. "the trait `Object` is not implemented"

**Error:**
```
error[E0277]: the trait `Object` is not implemented for `Query`
```

**Cause:** Forgot to add the `#[Object]` macro

**Solution:**
```rust
// Wrong
impl Query {
    async fn note(&self) -> Result<Note> { ... }
}

// Correct
#[Object]  // <-- Add this!
impl Query {
    async fn note(&self) -> Result<Note> { ... }
}
```

### 2. "cannot find type `Context` in this scope"

**Error:**
```
error[E0412]: cannot find type `Context` in this scope
```

**Cause:** Missing import

**Solution:**
```rust
use async_graphql::{Context, Object, Result, ID};  // Import all needed types
```

### 3. "mismatched types: expected Result<T>, found T"

**Error:**
```
error[E0308]: mismatched types
expected enum `Result<Note>`
found struct `Note`
```

**Cause:** GraphQL resolvers must return `Result<T>`

**Solution:**
```rust
// Wrong
async fn note(&self) -> Note {
    Note { id: "123".into() }
}

// Correct
async fn note(&self) -> Result<Note> {
    Ok(Note { id: "123".into() })
}
```

## Runtime Errors

### 1. "Cannot query field 'X' on type 'Y'"

**Error in GraphQL response:**
```json
{
  "errors": [{
    "message": "Cannot query field 'email' on type 'Note'"
  }]
}
```

**Cause:** Field doesn't exist in GraphQL schema

**Solution:** Check your type definition:
```rust
#[derive(SimpleObject)]
pub struct Note {
    id: String,
    title: String,
    // If email isn't here, you can't query it!
}
```

### 2. "Variable '$id' of required type 'ID!' was not provided"

**Error:**
```json
{
  "errors": [{
    "message": "Variable '$id' of required type 'ID!' was not provided"
  }]
}
```

**Cause:** Query uses variables but none provided

**Solution:**
```rust
// Query with variable
let query = r#"
    query GetNote($id: ID!) {
        note(id: $id) { title }
    }
"#;

// Must provide variables
let request = Request::new(query)
    .variables(Variables::from_value(json!({
        "id": "123"  // <-- This was missing
    })));
```

### 3. "Data source not found in context"

**Error:**
```
thread 'main' panicked at 'Data source not found in context'
```

**Cause:** Trying to access context data that wasn't added to schema

**Solution:**
```rust
// When building schema, add all needed data
let schema = Schema::build(Query, Mutation, Subscription)
    .data(database_service)    // <-- Add this
    .data(auth_context)        // <-- And this
    .finish();

// Then in resolver
let db = ctx.data::<Arc<dyn DatabaseService>>()?;  // Now this works
```

### 4. "Query depth 16 exceeds maximum allowed depth of 15"

**Error:**
```json
{
  "errors": [{
    "message": "Query depth 16 exceeds maximum allowed depth of 15"
  }]
}
```

**Cause:** Query is too deeply nested (security limit)

**Solution:** Flatten your query:
```graphql
# Too deep
query {
  note(id: "1") {
    author {
      posts {
        comments {
          author {
            posts {  # Getting too deep!
              ...
            }
          }
        }
      }
    }
  }
}

# Better - use multiple queries
query GetNote {
  note(id: "1") {
    author { id name }
  }
}

query GetAuthorPosts($authorId: ID!) {
  user(id: $authorId) {
    posts { id title }
  }
}
```

## Async/Await Errors

### 1. "future cannot be sent between threads safely"

**Error:**
```
error: future cannot be sent between threads safely
```

**Cause:** Using non-Send types in async resolvers

**Solution:** Use Arc instead of Rc, Mutex instead of RefCell:
```rust
// Wrong
use std::rc::Rc;
let data = Rc::new(data);

// Correct
use std::sync::Arc;
let data = Arc::new(data);
```

### 2. "await is only allowed inside async functions"

**Error:**
```
error[E0728]: `await` is only allowed inside `async` functions and blocks
```

**Cause:** Forgot to mark resolver as async

**Solution:**
```rust
#[Object]
impl Query {
    // Wrong
    fn note(&self, ctx: &Context<'_>) -> Result<Note> {
        let db = ctx.data::<Database>()?;
        db.get_note().await  // Error!
    }
    
    // Correct
    async fn note(&self, ctx: &Context<'_>) -> Result<Note> {
        let db = ctx.data::<Database>()?;
        db.get_note().await  // OK!
    }
}
```

## Subscription Errors

### 1. "the trait `Stream` is not implemented"

**Error:**
```
error[E0277]: the trait `Stream` is not implemented
```

**Cause:** Subscription must return a Stream

**Solution:**
```rust
use futures_util::stream::{Stream, StreamExt};

#[Subscription]
impl Subscription {
    async fn note_created(&self) -> impl Stream<Item = Note> {
        // Return a stream, not a single value
        let receiver = get_event_receiver();
        receiver.filter_map(|event| {
            match event {
                Event::NoteCreated(note) => Some(note),
                _ => None,
            }
        })
    }
}
```

### 2. WebSocket Connection Drops

**Symptom:** Subscriptions stop working after a while

**Common Causes:**
1. No heartbeat/ping-pong
2. Token expiration
3. Server timeout

**Solution:**
```rust
// Configure WebSocket with keepalive
let websocket_config = WebSocketConfig {
    keep_alive_interval: Duration::from_secs(30),
    ..Default::default()
};
```

## DataLoader Errors

### 1. "the trait `Loader` is not implemented"

**Error:**
```
error[E0277]: the trait `Loader<K>` is not implemented
```

**Cause:** DataLoader requires specific trait implementation

**Solution:**
```rust
use async_graphql::dataloader::Loader;
use async_trait::async_trait;

struct UserLoader {
    db: Arc<Database>,
}

#[async_trait]
impl Loader<String> for UserLoader {
    type Value = User;
    type Error = Arc<anyhow::Error>;
    
    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        // Batch load implementation
        let users = self.db.get_users_by_ids(keys).await?;
        Ok(users.into_iter().map(|u| (u.id.clone(), u)).collect())
    }
}
```

## Debug Tips

### 1. Enable GraphQL Query Logging

```rust
use async_graphql::extensions::Logger;

let schema = Schema::build(Query, Mutation, Subscription)
    .extension(Logger)  // Logs all queries
    .finish();
```

### 2. Test in GraphQL Playground

Navigate to `http://localhost:8080/graphql` and use the interactive playground to:
- Test queries
- View schema documentation
- See real-time error messages

### 3. Use Descriptive Error Messages

```rust
// Bad
Err(Error::new("Error"))

// Good
Err(Error::new("Note not found with id: 123")
    .extend_with(|_, e| {
        e.set("code", "NOT_FOUND");
        e.set("id", "123");
    }))
```

### 4. Check Your Imports

Common missing imports:
```rust
use async_graphql::{
    Context, Object, Result, Error, ID,
    SimpleObject, InputObject, Subscription,
    Schema, Request, Response, Variables,
};
use async_graphql::connection::*;  // For pagination
use futures_util::stream::{Stream, StreamExt};  // For subscriptions
```

Remember: Most GraphQL errors are either:
1. Schema definition issues (missing fields, wrong types)
2. Context data not available
3. Async/await problems
4. Security limits being hit

When in doubt, check the GraphQL Playground first!
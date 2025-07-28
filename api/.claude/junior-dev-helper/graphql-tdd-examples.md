# GraphQL Test-Driven Development Examples

## TDD Workflow for GraphQL

1. **Define the GraphQL schema** (what you want)
2. **Write the test** (how it should behave)
3. **Run test and see it fail** (verify test is valid)
4. **Implement minimal code** (make test pass)
5. **Refactor** (clean up implementation)
6. **Document** (add comments and docs)

## Example 1: Basic Query

### Step 1: Define What We Want

```graphql
type Query {
  note(id: ID!): Note
}

type Note {
  id: ID!
  title: String!
  content: String!
}
```

### Step 2: Write the Test First

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{Schema, Request, Variables};
    
    #[tokio::test]
    async fn test_note_query_returns_note_when_exists() {
        // Arrange
        let schema = create_test_schema();
        let query = r#"
            query GetNote($id: ID!) {
                note(id: $id) {
                    id
                    title
                    content
                }
            }
        "#;
        
        let variables = Variables::from_value(json!({
            "id": "test-note-123"
        }));
        
        // Act
        let request = Request::new(query).variables(variables);
        let response = schema.execute(request).await;
        
        // Assert
        assert!(response.errors.is_empty());
        
        let note_data = response.data.get("note").unwrap();
        assert_eq!(note_data.get("id").unwrap(), "test-note-123");
        assert_eq!(note_data.get("title").unwrap(), "Test Note");
        assert_eq!(note_data.get("content").unwrap(), "Test content");
    }
    
    #[tokio::test]
    async fn test_note_query_returns_null_when_not_found() {
        // Arrange
        let schema = create_test_schema();
        let query = r#"
            query {
                note(id: "non-existent-id") {
                    id
                }
            }
        "#;
        
        // Act
        let response = schema.execute(query).await;
        
        // Assert
        assert!(response.errors.is_empty());
        assert!(response.data.get("note").is_null());
    }
}
```

### Step 3: Run Test (Should Fail)

```bash
cargo test test_note_query
# Error: cannot find function `create_test_schema`
```

### Step 4: Implement Minimal Code

```rust
use async_graphql::{Object, Schema, Context, Result, ID};

pub struct Query;

#[Object]
impl Query {
    async fn note(&self, _ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        // Minimal implementation to pass test
        if id.as_str() == "test-note-123" {
            Ok(Some(Note {
                id: id.to_string(),
                title: "Test Note".to_string(),
                content: "Test content".to_string(),
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
}

#[Object]
impl Note {
    async fn id(&self) -> &str { &self.id }
    async fn title(&self) -> &str { &self.title }
    async fn content(&self) -> &str { &self.content }
}

fn create_test_schema() -> Schema<Query, EmptyMutation, EmptySubscription> {
    Schema::new(Query, EmptyMutation, EmptySubscription)
}
```

### Step 5: Refactor

```rust
// Extract to database service
#[Object]
impl Query {
    async fn note(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Note>> {
        let db = ctx.data::<Arc<dyn DatabaseService>>()?;
        
        match db.read("notes", id.as_str()).await {
            Ok(Some(data)) => {
                let note: Note = serde_json::from_value(data)?;
                Ok(Some(note))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::error!("Failed to fetch note: {}", e);
                Err(Error::new("Failed to fetch note"))
            }
        }
    }
}
```

## Example 2: Mutation with Input Validation

### Step 1: Define Schema

```graphql
type Mutation {
  createNote(input: CreateNoteInput!): Note!
}

input CreateNoteInput {
  title: String!
  content: String!
}
```

### Step 2: Write Tests First

```rust
#[tokio::test]
async fn test_create_note_with_valid_input() {
    // Arrange
    let schema = create_test_schema();
    let mutation = r#"
        mutation CreateNote($input: CreateNoteInput!) {
            createNote(input: $input) {
                id
                title
                content
            }
        }
    "#;
    
    let variables = Variables::from_value(json!({
        "input": {
            "title": "New Note",
            "content": "Note content"
        }
    }));
    
    // Act
    let request = Request::new(mutation).variables(variables);
    let response = schema.execute(request).await;
    
    // Assert
    assert!(response.errors.is_empty());
    
    let note = response.data.get("createNote").unwrap();
    assert!(!note.get("id").unwrap().as_str().unwrap().is_empty());
    assert_eq!(note.get("title").unwrap(), "New Note");
    assert_eq!(note.get("content").unwrap(), "Note content");
}

#[tokio::test]
async fn test_create_note_validates_empty_title() {
    // Arrange
    let schema = create_test_schema();
    let mutation = r#"
        mutation {
            createNote(input: {
                title: ""
                content: "Content"
            }) {
                id
            }
        }
    "#;
    
    // Act
    let response = schema.execute(mutation).await;
    
    // Assert
    assert!(!response.errors.is_empty());
    assert!(response.errors[0].message.contains("title"));
}

#[tokio::test]
async fn test_create_note_requires_authentication() {
    // Arrange
    let schema = create_test_schema_without_auth();
    let mutation = r#"
        mutation {
            createNote(input: {
                title: "Test"
                content: "Test"
            }) {
                id
            }
        }
    "#;
    
    // Act
    let response = schema.execute(mutation).await;
    
    // Assert
    assert!(!response.errors.is_empty());
    assert_eq!(
        response.errors[0].extensions.get("code").unwrap(),
        "UNAUTHORIZED"
    );
}
```

### Step 3: Implement

```rust
use async_graphql::{InputObject, Object};

#[derive(InputObject)]
pub struct CreateNoteInput {
    #[graphql(validator(min_length = 1, max_length = 200))]
    pub title: String,
    
    #[graphql(validator(min_length = 1))]
    pub content: String,
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn create_note(
        &self,
        ctx: &Context<'_>,
        input: CreateNoteInput,
    ) -> Result<Note> {
        // Check authentication
        let auth = ctx.data::<AuthContext>()
            .map_err(|_| Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHORIZED")))?;
        
        if !auth.is_authenticated() {
            return Err(Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHORIZED")));
        }
        
        // Create note
        let note = Note {
            id: Uuid::new_v4().to_string(),
            title: input.title,
            content: input.content,
            author: auth.user_id.clone().unwrap(),
            created_at: Utc::now(),
        };
        
        // Save to database
        let db = ctx.data::<Arc<dyn DatabaseService>>()?;
        db.create("notes", &note.id, serde_json::to_value(&note)?).await?;
        
        Ok(note)
    }
}
```

## Example 3: Subscription Testing

### Step 1: Define Subscription

```graphql
type Subscription {
  noteCreated: Note!
}
```

### Step 2: Write Test First

```rust
#[tokio::test]
async fn test_note_created_subscription() {
    // Arrange
    let broadcaster = Arc::new(EventBroadcaster::new(100));
    let schema = create_test_schema_with_broadcaster(broadcaster.clone());
    
    let subscription = r#"
        subscription {
            noteCreated {
                id
                title
            }
        }
    "#;
    
    // Act - Start subscription
    let mut stream = schema.execute_stream(subscription);
    tokio::pin!(stream);
    
    // Trigger event
    let note = Note {
        id: "sub-test-123".to_string(),
        title: "Subscription Test".to_string(),
        content: "Content".to_string(),
    };
    broadcaster.send(Event::NoteCreated(note.clone())).unwrap();
    
    // Assert - Check we received the event
    let response = tokio::time::timeout(
        Duration::from_secs(1),
        stream.next()
    ).await.unwrap().unwrap();
    
    assert!(response.errors.is_empty());
    let received_note = response.data.get("noteCreated").unwrap();
    assert_eq!(received_note.get("id").unwrap(), "sub-test-123");
    assert_eq!(received_note.get("title").unwrap(), "Subscription Test");
}
```

### Step 3: Implement

```rust
use futures_util::stream::{Stream, StreamExt};
use tokio::sync::broadcast;

pub struct Subscription;

#[Subscription]
impl Subscription {
    async fn note_created(
        &self,
        ctx: &Context<'_>,
    ) -> Result<impl Stream<Item = Note>> {
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        let receiver = broadcaster.subscribe();
        
        Ok(receiver
            .filter_map(|event| async move {
                match event {
                    Ok(Event::NoteCreated(note)) => Some(note),
                    _ => None,
                }
            }))
    }
}
```

## Example 4: DataLoader Testing

### Step 1: Test N+1 Prevention

```rust
#[tokio::test]
async fn test_dataloader_prevents_n_plus_one() {
    // Arrange
    let query_count = Arc::new(AtomicUsize::new(0));
    let db = MockDatabase::new(query_count.clone());
    let schema = create_test_schema_with_db(db);
    
    let query = r#"
        query {
            notes {
                id
                author {
                    id
                    name
                }
            }
        }
    "#;
    
    // Act
    let response = schema.execute(query).await;
    
    // Assert
    assert!(response.errors.is_empty());
    
    // Should be 2 queries: 1 for notes, 1 for all authors
    assert_eq!(query_count.load(Ordering::SeqCst), 2);
}
```

### Step 2: Implement DataLoader

```rust
pub struct AuthorLoader {
    db: Arc<dyn DatabaseService>,
}

#[async_trait]
impl Loader<String> for AuthorLoader {
    type Value = User;
    type Error = Arc<anyhow::Error>;
    
    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        // Single query for all authors
        let users = self.db
            .query("SELECT * FROM users WHERE id = ANY($1)")
            .bind(keys)
            .fetch_all()
            .await?;
        
        Ok(users.into_iter().map(|u| (u.id.clone(), u)).collect())
    }
}
```

## Example 5: Error Handling

### Step 1: Test Error Cases

```rust
#[tokio::test]
async fn test_database_error_returns_user_friendly_message() {
    // Arrange
    let schema = create_test_schema_with_failing_db();
    
    let query = r#"
        query {
            note(id: "any-id") {
                id
            }
        }
    "#;
    
    // Act
    let response = schema.execute(query).await;
    
    // Assert
    assert!(!response.errors.is_empty());
    let error = &response.errors[0];
    
    // Should not expose internal details
    assert!(!error.message.contains("database"));
    assert!(!error.message.contains("connection"));
    assert_eq!(error.message, "Failed to fetch note");
}
```

## Best Practices for GraphQL TDD

### 1. Test Query Structure

```rust
#[test]
fn test_schema_structure() {
    let schema = create_schema();
    let sdl = schema.sdl();
    
    // Verify schema includes expected types
    assert!(sdl.contains("type Query"));
    assert!(sdl.contains("type Mutation"));
    assert!(sdl.contains("type Note"));
}
```

### 2. Test Input Validation

```rust
#[tokio::test]
async fn test_input_validation() {
    let test_cases = vec![
        ("", "Title cannot be empty"),
        ("A".repeat(201), "Title too long"),
        ("Valid Title", "should succeed"),
    ];
    
    for (title, expected) in test_cases {
        let result = create_note_with_title(title).await;
        // Assert based on expected outcome
    }
}
```

### 3. Test Authorization

```rust
#[tokio::test]
async fn test_authorization_matrix() {
    let test_cases = vec![
        (Some("user1"), "note1", "read", true),   // Owner can read
        (Some("user2"), "note1", "read", false),  // Others cannot
        (None, "note1", "read", false),           // Anon cannot
    ];
    
    for (user_id, note_id, action, should_succeed) in test_cases {
        let result = check_authorization(user_id, note_id, action).await;
        assert_eq!(result.is_ok(), should_succeed);
    }
}
```

### 4. Test Performance

```rust
#[tokio::test]
async fn test_query_performance() {
    let schema = create_schema();
    let start = Instant::now();
    
    let query = r#"
        query {
            notes(first: 100) {
                edges {
                    node { id title }
                }
            }
        }
    "#;
    
    schema.execute(query).await;
    
    let duration = start.elapsed();
    assert!(duration < Duration::from_millis(100), "Query too slow");
}
```

## Testing Tips

1. **Use test builders** for complex objects
2. **Create test fixtures** for common data
3. **Test edge cases** (empty lists, nulls, limits)
4. **Test concurrent access** for subscriptions
5. **Mock external services** (database, auth)
6. **Use snapshot testing** for schema changes
7. **Test error paths** as much as success paths

Remember: In TDD, if you're not seeing red tests first, you're not doing TDD!
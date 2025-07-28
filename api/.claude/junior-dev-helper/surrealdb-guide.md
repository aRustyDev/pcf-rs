# SurrealDB Guide for Junior Developers

## What is SurrealDB?

SurrealDB is a modern, multi-model database that combines the flexibility of NoSQL with the power of SQL queries. Think of it as:
- Document store (like MongoDB)
- Graph database (like Neo4j)  
- Relational database (like PostgreSQL)
- All in one!

## Key Concepts

### 1. Thing IDs

SurrealDB uses a unique ID format called "Thing IDs":

```
table:id
```

Examples:
```
note:01234567-89ab-cdef      # A note with UUID
user:alice                    # A user with string ID
session:1234567890           # A session with numeric ID
```

This format tells you:
- What type of record it is (table name)
- The unique identifier within that table

### 2. Tables are Flexible

Unlike traditional SQL databases, you don't need to create tables first:

```rust
// Just start inserting - table created automatically!
db.create("note", json!({
    "title": "My First Note",
    "content": "Hello SurrealDB!"
})).await?;
```

### 3. Built-in Relations

SurrealDB handles relationships natively:

```rust
// Create a user
let user_id = db.create("user", json!({
    "name": "Alice"
})).await?;

// Create a note that references the user
let note_id = db.create("note", json!({
    "title": "Alice's Note",
    "author": user_id,  // Direct reference!
})).await?;

// Query with relation
let query = "SELECT *, author.name as author_name FROM note";
```

## Setting Up SurrealDB

### Local Development with Docker

```bash
# Start SurrealDB
docker run -d \
  --name surrealdb \
  -p 8000:8000 \
  surrealdb/surrealdb:latest \
  start --user root --pass root

# Or use the project command
just surrealdb-up
```

### Connecting from Rust

```rust
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::{Ws, Wss};
use surrealdb::opt::auth::Root;

// Create connection
let db = Surreal::new::<Ws>("localhost:8000").await?;

// Sign in
db.signin(Root {
    username: "root",
    password: "root",
}).await?;

// Select namespace and database
db.use_ns("test").use_db("test").await?;
```

## Basic Operations

### CREATE - Adding Records

```rust
// Simple create
let created: Vec<Record> = db
    .create("note")
    .content(Note {
        title: "Shopping List".to_string(),
        content: "Milk, Eggs, Bread".to_string(),
        completed: false,
    })
    .await?;

// Create with specific ID
let created: Option<Record> = db
    .create(("note", "shopping-list"))
    .content(note_data)
    .await?;
```

### READ - Getting Records

```rust
// Get by Thing ID
let note: Option<Note> = db
    .select(("note", "shopping-list"))
    .await?;

// Get all records from a table
let notes: Vec<Note> = db
    .select("note")
    .await?;
```

### UPDATE - Modifying Records

```rust
// Update specific fields (merge)
let updated: Option<Note> = db
    .update(("note", "shopping-list"))
    .merge(json!({
        "completed": true,
        "completed_at": Utc::now()
    }))
    .await?;

// Replace entire record
let replaced: Option<Note> = db
    .update(("note", "shopping-list"))
    .content(new_note_data)
    .await?;
```

### DELETE - Removing Records

```rust
// Delete by ID
let deleted: Option<Note> = db
    .delete(("note", "shopping-list"))
    .await?;

// Delete all in table (careful!)
let deleted: Vec<Note> = db
    .delete("note")
    .await?;
```

## Querying with SurrealQL

SurrealQL is like SQL but with superpowers:

### Basic Queries

```rust
// Simple SELECT
let notes: Vec<Note> = db
    .query("SELECT * FROM note WHERE completed = false")
    .await?
    .take(0)?;

// With parameters (prevents injection)
let notes: Vec<Note> = db
    .query("SELECT * FROM note WHERE author = $author")
    .bind(("author", "user:alice"))
    .await?
    .take(0)?;

// Ordering and limits
let recent: Vec<Note> = db
    .query("SELECT * FROM note ORDER BY created_at DESC LIMIT 10")
    .await?
    .take(0)?;
```

### Advanced Queries

```rust
// Graph traversal
let user_notes = db.query(r#"
    SELECT name, ->wrote->note.* as notes 
    FROM user:alice
"#).await?;

// Aggregations
let stats = db.query(r#"
    SELECT 
        count() as total,
        count(completed = true) as completed,
        count(completed = false) as pending
    FROM note
"#).await?;

// Subqueries
let active_users = db.query(r#"
    SELECT * FROM user WHERE 
        count(->wrote->note[WHERE created_at > $last_week]) > 0
"#)
.bind(("last_week", Utc::now() - Duration::days(7)))
.await?;
```

## Working with Thing IDs

### Parsing Thing IDs

```rust
use surrealdb::sql::Thing;

// Parse a Thing ID string
let thing_id = "note:01234567-89ab-cdef";
let thing: Thing = thing_id.parse()?;

println!("Table: {}", thing.tb);  // "note"
println!("ID: {}", thing.id);     // "01234567-89ab-cdef"

// Create Thing ID programmatically
let thing = Thing {
    tb: "note".to_string(),
    id: Id::String("my-custom-id".to_string()),
};
```

### ID Strategies

```rust
// 1. Let SurrealDB generate (ULIDs by default)
let auto_id = db.create("note").content(data).await?;

// 2. Use UUIDs
let uuid_id = Thing {
    tb: "note".to_string(),
    id: Id::String(Uuid::new_v4().to_string()),
};

// 3. Use custom IDs (slugs, etc)
let slug_id = Thing {
    tb: "post".to_string(),
    id: Id::String("my-first-blog-post".to_string()),
};

// 4. Numeric IDs
let numeric_id = Thing {
    tb: "order".to_string(),
    id: Id::Number(12345),
};
```

## Data Modeling Tips

### 1. Embed vs Reference

```rust
// Embedding - Good for data that's always accessed together
let post = json!({
    "title": "My Post",
    "content": "...",
    "metadata": {
        "views": 0,
        "likes": 0,
        "published": true
    }
});

// Referencing - Good for shared data
let post = json!({
    "title": "My Post",
    "content": "...",
    "author": "user:alice",  // Reference to user
    "tags": ["tag:rust", "tag:database"]  // References to tags
});
```

### 2. Use Record Links

```rust
// Create relationships as records
let liked = db.create("liked")
    .content(json!({
        "user": "user:alice",
        "post": "post:123",
        "liked_at": Utc::now()
    }))
    .await?;

// Query relationships
let user_likes = db.query(r#"
    SELECT post.* FROM liked 
    WHERE user = user:alice
    ORDER BY liked_at DESC
"#).await?;
```

### 3. Schema Validation

While SurrealDB is schemaless, you can add constraints:

```sql
-- Define schema rules
DEFINE FIELD title ON TABLE note TYPE string 
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD created_at ON TABLE note TYPE datetime 
    VALUE $value OR time::now();

DEFINE FIELD author ON TABLE note TYPE record(user)
    ASSERT $value != NONE;
```

## Common Patterns

### 1. Soft Deletes

```rust
// Instead of DELETE, mark as deleted
let soft_deleted = db
    .update(("note", id))
    .merge(json!({
        "deleted": true,
        "deleted_at": Utc::now()
    }))
    .await?;

// Query only active records
let active_notes = db
    .query("SELECT * FROM note WHERE deleted != true")
    .await?;
```

### 2. Audit Trail

```rust
// Create audit records for changes
async fn audit_change(db: &Surreal<Db>, action: &str, record: Thing) {
    db.create("audit")
        .content(json!({
            "action": action,
            "record": record,
            "user": current_user(),
            "timestamp": Utc::now(),
            "ip_address": request_ip()
        }))
        .await?;
}
```

### 3. Full-Text Search

```rust
// Create search index
db.query("DEFINE INDEX search_idx ON TABLE note COLUMNS title, content SEARCH ANALYZER ascii").await?;

// Search records
let results = db
    .query("SELECT * FROM note WHERE title @@ $search OR content @@ $search")
    .bind(("search", "rust database"))
    .await?;
```

## Error Handling

### Common SurrealDB Errors

```rust
use surrealdb::Error;

match db.select(("note", "invalid-id")).await {
    Ok(Some(note)) => { /* Found */ },
    Ok(None) => { /* Not found - normal case */ },
    Err(Error::Api(e)) => {
        // API errors (network, auth, etc)
        eprintln!("API Error: {}", e);
    },
    Err(Error::Db(e)) => {
        // Database errors (query, constraints)
        eprintln!("Database Error: {}", e);
    },
    Err(e) => {
        // Other errors
        eprintln!("Error: {}", e);
    }
}
```

### Best Practices

1. **Always handle None for single selects**
   ```rust
   // This returns Option<T>
   let note: Option<Note> = db.select(("note", id)).await?;
   
   if let Some(note) = note {
       // Process note
   } else {
       // Handle not found
   }
   ```

2. **Use transactions for consistency**
   ```rust
   let transaction = db.transaction().await?;
   
   transaction.query("CREATE note:1 SET title = 'Test'").await?;
   transaction.query("UPDATE user:alice SET note_count += 1").await?;
   
   transaction.commit().await?;
   ```

3. **Validate Thing IDs**
   ```rust
   fn parse_thing_id(id: &str) -> Result<Thing, Error> {
       id.parse::<Thing>()
           .map_err(|_| Error::InvalidThingId(id.to_string()))
   }
   ```

## Performance Tips

### 1. Use Indexes

```sql
-- Index for queries
DEFINE INDEX email_idx ON TABLE user COLUMNS email UNIQUE;
DEFINE INDEX created_idx ON TABLE note COLUMNS created_at;
```

### 2. Batch Operations

```rust
// Instead of many individual queries
for data in items {
    db.create("note").content(data).await?;  // Slow!
}

// Use a single query
let query = items.iter().enumerate()
    .map(|(i, data)| format!("CREATE note CONTENT $data_{}", i))
    .collect::<Vec<_>>()
    .join(";");

let mut bindings = HashMap::new();
for (i, data) in items.iter().enumerate() {
    bindings.insert(format!("data_{}", i), data);
}

db.query(query).bind(bindings).await?;
```

### 3. Projection

```rust
// Only select needed fields
let summaries: Vec<_> = db
    .query("SELECT id, title, created_at FROM note")
    .await?
    .take(0)?;
```

## Testing with SurrealDB

### In-Memory for Tests

```rust
use surrealdb::engine::local::Mem;

#[tokio::test]
async fn test_note_creation() {
    // In-memory database - no persistence
    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("test").use_db("test").await?;
    
    // Run your tests
    let note: Vec<Record> = db
        .create("note")
        .content(test_data)
        .await?;
    
    assert_eq!(note.len(), 1);
    // Database cleaned up automatically
}
```

### TestContainers for Integration Tests

```rust
#[tokio::test]
async fn test_real_surrealdb() {
    let container = SurrealDbContainer::new().start().await;
    let port = container.get_port().await;
    
    let db = Surreal::new::<Ws>(format!("localhost:{}", port)).await?;
    // ... run tests
}
```

## Common Pitfalls

1. **Forgetting await on queries**
   ```rust
   // Wrong - returns Future, not result!
   let notes = db.query("SELECT * FROM note");
   
   // Correct
   let notes = db.query("SELECT * FROM note").await?;
   ```

2. **Not handling Thing ID format**
   ```rust
   // Wrong - passing just the ID part
   db.select("123").await?;
   
   // Correct - full Thing ID
   db.select(("note", "123")).await?;
   ```

3. **Assuming order without ORDER BY**
   ```rust
   // Records have no guaranteed order
   let notes = db.select("note").await?;
   
   // Always specify order if needed
   let notes = db.query("SELECT * FROM note ORDER BY created_at").await?;
   ```

## Resources

- [SurrealDB Documentation](https://surrealdb.com/docs)
- [SurrealQL Tutorial](https://surrealdb.com/docs/surrealql)
- [Rust SDK Docs](https://docs.rs/surrealdb/latest/surrealdb/)
- [Example Projects](https://github.com/surrealdb/surrealdb/tree/main/examples)

Remember: SurrealDB is powerful but different. Take time to understand Thing IDs and the query language - they're the keys to using it effectively!
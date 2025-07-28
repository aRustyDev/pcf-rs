# DataLoader Guide - Preventing N+1 Queries

## What is the N+1 Query Problem?

The N+1 query problem occurs when you fetch a list of items (1 query) and then fetch related data for each item (N queries), resulting in N+1 total queries.

### Example of N+1 Problem

```graphql
query {
  notes {         # 1 query to get all notes
    id
    title
    author {      # N queries - one per note to get author
      name
      email
    }
  }
}
```

If you have 100 notes, this results in:
- 1 query to fetch notes
- 100 queries to fetch each author
- Total: 101 queries! 😱

## How DataLoader Solves This

DataLoader batches and caches requests within a single GraphQL request:

1. Collects all author IDs needed
2. Makes ONE query to fetch all authors
3. Distributes results to each resolver

Result: 2 queries total instead of 101!

## Implementation Guide

### Step 1: Create a Loader

```rust
use async_graphql::dataloader::Loader;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AuthorLoader {
    database: Arc<dyn DatabaseService>,
}

impl AuthorLoader {
    pub fn new(database: Arc<dyn DatabaseService>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl Loader<String> for AuthorLoader {
    type Value = User;
    type Error = Arc<anyhow::Error>;
    
    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        // This is called with ALL author IDs at once!
        println!("Loading users: {:?}", keys);  // For debugging
        
        // Batch query - get all users in one go
        let users = self.database
            .query("SELECT * FROM users WHERE id IN (?)")
            .bind(keys)
            .fetch_all()
            .await?;
        
        // Return HashMap mapping ID -> User
        let mut map = HashMap::new();
        for user in users {
            map.insert(user.id.clone(), user);
        }
        
        Ok(map)
    }
}
```

### Step 2: Add DataLoader to Schema

```rust
use async_graphql::dataloader::DataLoader;

// Create the DataLoader
let author_loader = DataLoader::new(
    AuthorLoader::new(database.clone()),
    tokio::spawn  // Requires tokio runtime
);

// Add to schema
let schema = Schema::build(Query, Mutation, Subscription)
    .data(database)
    .data(author_loader)  // Add the loader
    .finish();
```

### Step 3: Use in Resolvers

```rust
#[Object]
impl Note {
    async fn id(&self) -> &str {
        &self.id
    }
    
    async fn title(&self) -> &str {
        &self.title
    }
    
    // This is where the magic happens!
    async fn author(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        // Get the DataLoader from context
        let loader = ctx.data::<DataLoader<AuthorLoader>>()?;
        
        // Load the author - this will be batched!
        loader.load_one(self.author_id.clone()).await
            .map_err(|e| Error::new(e.to_string()))
    }
}
```

## Common DataLoader Patterns

### 1. Loading Multiple Relations

```rust
// Create multiple loaders
let author_loader = DataLoader::new(AuthorLoader::new(db.clone()), tokio::spawn);
let tags_loader = DataLoader::new(TagsLoader::new(db.clone()), tokio::spawn);
let comments_loader = DataLoader::new(CommentsLoader::new(db.clone()), tokio::spawn);

// Add all to schema
let schema = Schema::build(Query, Mutation, Subscription)
    .data(author_loader)
    .data(tags_loader)
    .data(comments_loader)
    .finish();
```

### 2. Loading Lists (One-to-Many)

```rust
pub struct NoteTagsLoader {
    database: Arc<dyn DatabaseService>,
}

#[async_trait]
impl Loader<String> for NoteTagsLoader {
    type Value = Vec<Tag>;  // Note: Vec for one-to-many
    type Error = Arc<anyhow::Error>;
    
    async fn load(&self, note_ids: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        // Get all tags for all notes
        let tags = self.database
            .query("SELECT * FROM tags WHERE note_id IN (?)")
            .bind(note_ids)
            .fetch_all()
            .await?;
        
        // Group by note_id
        let mut map: HashMap<String, Vec<Tag>> = HashMap::new();
        for tag in tags {
            map.entry(tag.note_id.clone())
                .or_insert_with(Vec::new)
                .push(tag);
        }
        
        // Ensure every requested ID has an entry (even if empty)
        for id in note_ids {
            map.entry(id.clone()).or_insert_with(Vec::new);
        }
        
        Ok(map)
    }
}
```

### 3. Caching Within Request

DataLoader automatically caches within a single GraphQL request:

```graphql
query {
  noteById(id: "123") {
    author { name }  # Loads author "abc"
  }
  notesByAuthor(authorId: "abc") {
    author { name }  # Uses cached author "abc" - no new query!
  }
}
```

### 4. Prime the Cache

You can pre-load data:

```rust
// In a resolver that knows it will need certain data
let loader = ctx.data::<DataLoader<AuthorLoader>>()?;

// Prime the cache with known IDs
let author_ids: Vec<String> = notes.iter().map(|n| n.author_id.clone()).collect();
loader.load_many(author_ids).await?;

// Later accesses will use cache
```

## Best Practices

### 1. One Loader Per Entity Type

```rust
// Good - separate loaders
DataLoader<UserLoader>
DataLoader<NoteLoader>
DataLoader<TagLoader>

// Bad - generic loader
DataLoader<GenericLoader>  // What does this load?
```

### 2. Handle Missing Data

Always return HashMap entries for all requested keys:

```rust
async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
    let mut map = HashMap::new();
    
    // ... load data ...
    
    // IMPORTANT: Ensure all keys have entries
    for key in keys {
        map.entry(key.clone()).or_insert(default_value());
    }
    
    Ok(map)
}
```

### 3. Use Appropriate Scope

DataLoader caches within a single request. Don't try to share between requests:

```rust
// ❌ BAD: Shared across requests (data leakage!)
static LOADER: Lazy<DataLoader<UserLoader>> = Lazy::new(|| {
    DataLoader::new(UserLoader::new())
});

// ✅ GOOD: Fresh per request
impl GraphQLContext {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            user_loader: DataLoader::new(UserLoader::new(db.clone())),
            // ... other fields
        }
    }
}
```

## Phase 6 Integration Example

Here's how DataLoader integrates with all Phase 6 optimizations:

```rust
use async_graphql::*;
use crate::performance::{DataLoader, ResponseCache, TimeoutContext};

/// Complete GraphQL context with Phase 6 optimizations
pub struct GraphQLContext {
    // DataLoaders prevent N+1
    pub user_loader: DataLoader<UserLoader>,
    pub note_loader: DataLoader<NoteLoader>,
    pub tag_loader: DataLoader<TagLoader>,
    
    // Response caching
    pub cache: Arc<ResponseCache>,
    
    // Timeout management
    pub timeout: TimeoutContext,
    
    // Database with connection pooling
    pub db: Arc<DatabaseService>,
}

/// Note resolver using all optimizations
#[Object]
impl Note {
    /// Author field - no N+1!
    async fn author(&self, ctx: &Context<'_>) -> Result<User> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check timeout budget
        if context.timeout.remaining() < Duration::from_millis(100) {
            return Err(Error::new("Insufficient time for author lookup"));
        }
        
        // DataLoader batches this with other author lookups
        context.user_loader
            .load_one(self.author_id.clone())
            .await
            .map(|arc| (*arc).clone())
    }
    
    /// Tags field - also batched
    async fn tags(&self, ctx: &Context<'_>) -> Result<Vec<Tag>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Pre-load all tags for this note
        context.tag_loader
            .load_tags_for_note(self.id.clone())
            .await
    }
}

/// Query resolver with optimization
#[Object]
impl Query {
    async fn notes_with_authors(
        &self,
        ctx: &Context<'_>,
        first: Option<i32>,
    ) -> Result<Vec<Note>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // 1. Check cache first
        let cache_key = format!("notes_with_authors:{}", first.unwrap_or(20));
        if let Some(cached) = context.cache.get(&cache_key).await {
            return Ok(cached);
        }
        
        // 2. Load notes from database
        let notes = context.db
            .query_with_timeout(
                "SELECT * FROM notes ORDER BY created_at DESC LIMIT $1",
                &[&first.unwrap_or(20)],
                context.timeout.child_budget("database"),
            )
            .await?;
        
        // 3. Pre-warm DataLoader cache to prevent N+1
        let author_ids: Vec<String> = notes.iter()
            .map(|n| n.author_id.clone())
            .collect();
        
        // This single call loads ALL authors at once
        context.user_loader.load_many(&author_ids).await?;
        
        // 4. Cache the result
        context.cache.set(&cache_key, &notes, Duration::from_secs(300)).await;
        
        Ok(notes)
    }
}
```

## Performance Metrics

Monitor DataLoader effectiveness:

```rust
// Add to your metrics
metrics! {
    histogram!("dataloader_batch_size", "loader" => "user").record(batch_size);
    counter!("dataloader_batch_count", "loader" => "user").increment(1);
    gauge!("dataloader_cache_size", "loader" => "user").set(cache_size);
    histogram!("dataloader_load_time", "loader" => "user").record(load_time);
}

// Calculate efficiency
let efficiency = total_keys_loaded / total_batches_executed;
info!("DataLoader efficiency: {}x", efficiency);
```

## Testing DataLoader

```rust
#[tokio::test]
async fn test_prevents_n_plus_one() {
    let db = MockDatabase::new();
    let query_counter = Arc::new(AtomicUsize::new(0));
    db.set_query_counter(query_counter.clone());
    
    let schema = create_schema();
    let context = GraphQLContext::new(Arc::new(db));
    
    // Execute query that would trigger N+1
    let query = r#"
        {
            notes(first: 10) {
                id
                title
                author {
                    name
                    email
                }
            }
        }
    "#;
    
    schema.execute(query).data(context).await;
    
    // Should be 2 queries: 1 for notes, 1 for all authors
    assert_eq!(query_counter.load(Ordering::SeqCst), 2);
}
```

## Common Pitfalls

1. **Forgetting to clear cache between requests**
   - Always create fresh DataLoaders per request
   
2. **Not handling missing data**
   - Always return HashMap entries for all requested keys
   
3. **Batching too much**
   - Set reasonable batch size limits (e.g., 1000 keys max)
   
4. **Not measuring effectiveness**
   - Monitor batch sizes and cache hit rates

```rust
// Wrong - trying to share DataLoader
lazy_static! {
    static ref LOADER: DataLoader<UserLoader> = DataLoader::new(...);
}

// Right - create per request
let loader = DataLoader::new(UserLoader::new(db), tokio::spawn);
```

### 4. Error Handling

Return consistent errors:

```rust
#[async_trait]
impl Loader<String> for AuthorLoader {
    type Value = User;
    type Error = Arc<anyhow::Error>;  // Arc for cloning
    
    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        match self.database.get_users(keys).await {
            Ok(users) => Ok(create_map(users)),
            Err(e) => Err(Arc::new(e.into())),  // Wrap in Arc
        }
    }
}
```

## Testing DataLoaders

### 1. Unit Test the Loader

```rust
#[tokio::test]
async fn test_author_loader() {
    let db = create_test_database();
    let loader = AuthorLoader::new(db);
    
    // Test batch loading
    let result = loader.load(&["1", "2", "3"]).await.unwrap();
    
    assert_eq!(result.len(), 3);
    assert_eq!(result.get("1").unwrap().name, "Alice");
}
```

### 2. Integration Test with GraphQL

```rust
#[tokio::test]
async fn test_no_n_plus_one() {
    let schema = create_test_schema();
    let query_count = Arc::new(AtomicUsize::new(0));
    
    // Track database queries
    let db = MockDatabase::new(query_count.clone());
    
    let query = r#"
        query {
            notes {
                author { name }
            }
        }
    "#;
    
    schema.execute(query).await;
    
    // Should only be 2 queries: notes + authors
    assert_eq!(query_count.load(Ordering::SeqCst), 2);
}
```

## Common Mistakes

### 1. Forgetting to Return All Keys

```rust
// Wrong - missing keys cause errors
async fn load(&self, keys: &[String]) -> Result<HashMap<String, User>> {
    let users = fetch_users(keys).await?;
    Ok(users.into_iter().map(|u| (u.id, u)).collect())
}

// Right - include None for missing
async fn load(&self, keys: &[String]) -> Result<HashMap<String, Option<User>>> {
    let users = fetch_users(keys).await?;
    let mut map = HashMap::new();
    
    // Add found users
    for user in users {
        map.insert(user.id.clone(), Some(user));
    }
    
    // Add None for missing
    for key in keys {
        map.entry(key.clone()).or_insert(None);
    }
    
    Ok(map)
}
```

### 2. Not Handling Errors Properly

```rust
// Wrong - panics on error
async fn author(&self, ctx: &Context<'_>) -> Result<User> {
    let loader = ctx.data::<DataLoader<AuthorLoader>>()?;
    Ok(loader.load_one(self.author_id).await.unwrap().unwrap())
}

// Right - proper error handling
async fn author(&self, ctx: &Context<'_>) -> Result<Option<User>> {
    let loader = ctx.data::<DataLoader<AuthorLoader>>()?;
    loader.load_one(self.author_id.clone()).await
        .map_err(|e| Error::new(format!("Failed to load author: {}", e)))
}
```

### 3. Over-caching

Remember: DataLoader cache is per-request only!

```rust
// This is fine - cache within request
query {
  note1: note(id: "1") { author { name } }
  note2: note(id: "2") { author { name } }
  # If both have same author, only 1 author query
}

// Different requests = different cache
# Request 1
query { note(id: "1") { author { name } } }

# Request 2 (new cache, will query author again)
query { note(id: "1") { author { name } } }
```

## Performance Tips

1. **Batch Size**: Most databases perform well with batches of 100-1000 items
2. **Timeout**: Add timeouts to prevent hanging on large batches
3. **Metrics**: Track batch sizes and query times
4. **Database Indexes**: Ensure your IN queries use indexes

```rust
// Add metrics
async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
    let start = Instant::now();
    let batch_size = keys.len();
    
    let result = self.load_batch(keys).await;
    
    metrics::histogram!("dataloader.batch_size", batch_size as f64);
    metrics::histogram!("dataloader.duration", start.elapsed().as_secs_f64());
    
    result
}
```

Remember: DataLoader is one of the most important optimizations for GraphQL APIs. Use it for any field that could be requested multiple times in a single query!
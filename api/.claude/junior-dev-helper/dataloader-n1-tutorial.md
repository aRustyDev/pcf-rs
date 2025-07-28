# DataLoader and N+1 Query Problem Tutorial

## What is the N+1 Query Problem?

The N+1 query problem is one of the most common performance issues in applications that work with databases. It happens when you make 1 query to get a list of items, then N additional queries to get related data for each item.

### Visual Example

Imagine you're building a blog that shows posts and their authors:

```
// 1 query to get all posts
SELECT * FROM posts LIMIT 10;

// Then for EACH post, you query for its author
SELECT * FROM users WHERE id = 1;  // For post 1
SELECT * FROM users WHERE id = 2;  // For post 2
SELECT * FROM users WHERE id = 3;  // For post 3
... (7 more queries)

Total: 1 + 10 = 11 queries! 😱
```

### Why is this bad?

1. **Database Load**: Each query has overhead (network, parsing, execution)
2. **Latency**: Sequential queries can't run in parallel
3. **Scalability**: 100 posts = 101 queries, 1000 posts = 1001 queries!

### Real GraphQL Example

```graphql
query {
  posts {
    id
    title
    author {  # This triggers N+1!
      name
      email
    }
  }
}
```

Without DataLoader, each `author` field makes a separate database query.

## How DataLoader Solves N+1

DataLoader is a utility that:
1. **Batches** multiple requests into a single query
2. **Caches** results within a single request
3. **Deduplicates** repeated requests

### The DataLoader Pattern

```rust
// Instead of this (N+1):
for post in posts {
    let author = db.get_user(post.author_id).await?;
}

// DataLoader does this:
let author_ids: Vec<_> = posts.iter().map(|p| p.author_id).collect();
let authors = db.get_users_batch(&author_ids).await?; // 1 query!
```

## Implementing DataLoader in Rust

### Step 1: Define Your Loader

```rust
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Loader: Send + Sync + 'static {
    type Key: Send + Sync + Clone + Eq + Hash;
    type Value: Send + Sync + Clone;
    type Error: Send + Sync;
    
    /// Load multiple keys in a single batch
    async fn load_batch(
        &self,
        keys: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, Self::Error>;
}
```

### Step 2: Implement for Your Type

```rust
pub struct UserLoader {
    db: Arc<DatabaseService>,
}

#[async_trait]
impl Loader for UserLoader {
    type Key = String;      // User ID
    type Value = User;      // User struct
    type Error = Error;
    
    async fn load_batch(
        &self,
        user_ids: &[String],
    ) -> Result<HashMap<String, User>, Error> {
        // One query for all users!
        let users = self.db
            .query("SELECT * FROM users WHERE id = ANY($1)")
            .bind(user_ids)
            .fetch_all()
            .await?;
        
        // Return as HashMap for O(1) lookup
        Ok(users.into_iter()
            .map(|user| (user.id.clone(), user))
            .collect())
    }
}
```

### Step 3: Use in GraphQL Resolvers

```rust
impl Post {
    async fn author(&self, ctx: &Context<'_>) -> Result<User> {
        // Get the DataLoader from context
        let loader = ctx.data::<DataLoader<UserLoader>>()?;
        
        // This doesn't query immediately - it batches!
        loader.load_one(self.author_id.clone())
            .await
            .map_err(|e| e.into())
    }
}
```

## Request-Scoped Caching

DataLoader also prevents duplicate queries within the same request:

```graphql
query {
  posts {
    author { name }        # Loads user ID "123"
  }
  recentComments {
    author { name }        # User "123" already cached!
  }
}
```

### Implementation

```rust
pub struct DataLoader<L: Loader> {
    loader: Arc<L>,
    // Cache is per-request, not global!
    cache: Arc<RwLock<HashMap<L::Key, Arc<L::Value>>>>,
}

impl<L: Loader> DataLoader<L> {
    pub async fn load_one(&self, key: L::Key) -> Result<Arc<L::Value>, L::Error> {
        // Check cache first
        if let Some(cached) = self.cache.read().await.get(&key) {
            return Ok(cached.clone());
        }
        
        // Not cached - add to batch
        self.load_uncached(key).await
    }
}
```

## Common Patterns and Best Practices

### 1. Pre-Loading (Look-Ahead)

```rust
// In a list resolver, pre-load all children
async fn posts(&self, ctx: &Context<'_>) -> Result<Vec<Post>> {
    let posts = self.db.get_posts().await?;
    
    // Pre-load all authors
    let author_ids: Vec<_> = posts.iter()
        .map(|p| p.author_id.clone())
        .collect();
    
    ctx.data::<DataLoader<UserLoader>>()?
        .load_many(&author_ids)
        .await?;
    
    Ok(posts)
}
```

### 2. Batch Size Limits

```rust
pub struct BatchConfig {
    /// Maximum keys per batch (prevent huge queries)
    pub max_batch_size: usize,
    /// How long to wait for more keys
    pub batch_delay: Duration,
}

// Split large batches
for chunk in keys.chunks(self.config.max_batch_size) {
    self.execute_batch(chunk).await?;
}
```

### 3. Error Handling

```rust
// Don't let one bad key fail the whole batch
async fn load_batch(&self, keys: &[String]) -> Result<HashMap<String, User>, Error> {
    let results = self.db.get_users_maybe(keys).await?;
    
    // Return what we found, missing keys return None
    Ok(results)
}
```

## Testing for N+1 Queries

### Write Tests That Detect N+1

```rust
#[tokio::test]
async fn test_no_n_plus_one_queries() {
    let db = MockDatabase::new();
    let query_count = Arc::new(AtomicUsize::new(0));
    
    // Track every database query
    db.on_query(move |_| {
        query_count.fetch_add(1, Ordering::SeqCst);
    });
    
    // Execute GraphQL query
    let query = r#"
        query {
            posts(first: 10) {
                title
                author { name }
                comments {
                    text
                    author { name }
                }
            }
        }
    "#;
    
    execute_query(query).await;
    
    // Should be:
    // 1 query for posts
    // 1 query for post authors
    // 1 query for comments
    // 1 query for comment authors
    // Total: 4 queries (not 30+!)
    assert!(query_count.load(Ordering::SeqCst) <= 4);
}
```

## Common Mistakes to Avoid

### 1. Global Caching

```rust
// ❌ BAD: Cache persists across requests
static CACHE: Lazy<RwLock<HashMap<String, User>>> = Lazy::new(Default::default);

// ✅ GOOD: Cache per request
pub fn create_context() -> GraphQLContext {
    GraphQLContext {
        user_loader: DataLoader::new(UserLoader::new()),
        // Fresh cache for each request
    }
}
```

### 2. Not Batching Related Queries

```rust
// ❌ BAD: Separate loaders for same data
let user_by_id_loader = DataLoader::new(UserByIdLoader);
let user_by_email_loader = DataLoader::new(UserByEmailLoader);

// ✅ GOOD: Single loader with composite keys
let user_loader = DataLoader::new(UserLoader);
// Can load by ID or email using enum key
```

### 3. Forgetting to Pre-Load

```rust
// ❌ BAD: Let each field load individually
impl Query {
    async fn posts(&self) -> Vec<Post> {
        self.db.get_posts().await
    }
}

// ✅ GOOD: Pre-load known relationships
impl Query {
    async fn posts(&self, ctx: &Context<'_>) -> Vec<Post> {
        let posts = self.db.get_posts().await?;
        
        // Pre-load authors while we have the list
        let author_ids: Vec<_> = posts.iter()
            .map(|p| p.author_id.clone())
            .collect();
        
        ctx.user_loader.load_many(&author_ids).await?;
        
        Ok(posts)
    }
}
```

## Debugging Tips

### 1. Add Logging

```rust
impl<L: Loader> DataLoader<L> {
    async fn flush_batch(&self, keys: Vec<L::Key>) {
        let batch_size = keys.len();
        let start = Instant::now();
        
        info!(
            batch_size = batch_size,
            loader = std::any::type_name::<L>(),
            "DataLoader batching queries"
        );
        
        let result = self.loader.load_batch(&keys).await;
        
        info!(
            batch_size = batch_size,
            duration_ms = start.elapsed().as_millis(),
            "DataLoader batch complete"
        );
    }
}
```

### 2. Metrics

```rust
// Track batch efficiency
counter!("dataloader_batch_total", "loader" => "UserLoader").increment(1);
histogram!("dataloader_batch_size", "loader" => "UserLoader").record(keys.len() as f64);
```

### 3. Development Tools

```rust
#[cfg(debug_assertions)]
impl DataLoader {
    pub fn stats(&self) -> DataLoaderStats {
        DataLoaderStats {
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            batches_executed: self.batches.load(Ordering::Relaxed),
            average_batch_size: self.total_keys.load(Ordering::Relaxed) / 
                              self.batches.load(Ordering::Relaxed),
        }
    }
}
```

## Summary

DataLoader is essential for GraphQL performance because it:
1. **Prevents N+1 queries** through batching
2. **Reduces duplicate work** through caching
3. **Improves response times** through parallel loading

Remember:
- Always use DataLoader for relationships
- Cache is per-request, not global
- Test specifically for N+1 problems
- Monitor batch sizes and efficiency

With DataLoader properly implemented, your GraphQL API can handle complex queries efficiently without database overload!
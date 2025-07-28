/// DataLoader Patterns - Phase 6 Implementation Examples
///
/// This file demonstrates DataLoader implementation patterns for preventing
/// N+1 queries in GraphQL resolvers with async batching and caching.

use async_trait::async_trait;
use async_graphql::{Context, dataloader::Loader as DataLoaderTrait};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, oneshot, Mutex};
use std::time::{Duration, Instant};
use futures::future::join_all;

/// Generic DataLoader trait for batch loading
#[async_trait]
pub trait BatchLoader: Send + Sync + 'static {
    /// The key type for loading
    type Key: Send + Sync + Clone + Eq + std::hash::Hash + std::fmt::Debug;
    /// The value type being loaded
    type Value: Send + Sync + Clone;
    /// The error type
    type Error: Send + Sync + std::fmt::Display;
    
    /// Load multiple keys in a single batch operation
    async fn load_batch(
        &self,
        keys: Vec<Self::Key>,
    ) -> Result<HashMap<Self::Key, Self::Value>, Self::Error>;
}

/// DataLoader configuration
#[derive(Debug, Clone)]
pub struct DataLoaderConfig {
    /// Maximum number of keys per batch
    pub max_batch_size: usize,
    /// Delay before executing batch (allows aggregation)
    pub batch_delay: Duration,
    /// Cache TTL within request
    pub cache_ttl: Duration,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for DataLoaderConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            batch_delay: Duration::from_millis(10),
            cache_ttl: Duration::from_secs(60),
            enable_metrics: true,
        }
    }
}

/// Generic DataLoader implementation
pub struct DataLoader<L: BatchLoader> {
    loader: Arc<L>,
    config: DataLoaderConfig,
    /// Request-scoped cache
    cache: Arc<RwLock<HashMap<L::Key, Arc<L::Value>>>>,
    /// Pending loads waiting to be batched
    pending: Arc<Mutex<HashMap<L::Key, Vec<oneshot::Sender<Arc<L::Value>>>>>>,
    /// Batch scheduling state
    batch_scheduled: Arc<Mutex<bool>>,
    /// Metrics collector
    metrics: Arc<DataLoaderMetrics>,
}

/// DataLoader metrics
#[derive(Default)]
pub struct DataLoaderMetrics {
    pub total_loads: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub batches_executed: AtomicU64,
    pub keys_per_batch: AtomicU64,
}

use std::sync::atomic::{AtomicU64, Ordering};

impl<L: BatchLoader> DataLoader<L> {
    /// Create a new DataLoader with default config
    pub fn new(loader: L) -> Self {
        Self::with_config(loader, DataLoaderConfig::default())
    }
    
    /// Create a new DataLoader with custom config
    pub fn with_config(loader: L, config: DataLoaderConfig) -> Self {
        Self {
            loader: Arc::new(loader),
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(Mutex::new(HashMap::new())),
            batch_scheduled: Arc::new(Mutex::new(false)),
            metrics: Arc::new(DataLoaderMetrics::default()),
        }
    }
    
    /// Load a single key
    pub async fn load_one(&self, key: L::Key) -> Result<Arc<L::Value>, L::Error> {
        self.metrics.total_loads.fetch_add(1, Ordering::Relaxed);
        
        // Check cache first
        if let Some(value) = self.get_cached(&key).await {
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(value);
        }
        
        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        
        // Add to pending batch
        let receiver = self.add_to_pending(key.clone()).await;
        
        // Schedule batch execution
        self.schedule_batch().await;
        
        // Wait for result
        match receiver.await {
            Ok(value) => Ok(value),
            Err(_) => Err(self.make_error("Batch execution cancelled")),
        }
    }
    
    /// Load multiple keys
    pub async fn load_many(&self, keys: &[L::Key]) -> Result<HashMap<L::Key, Arc<L::Value>>, L::Error> {
        let mut results = HashMap::new();
        let mut to_load = Vec::new();
        
        // Check cache for each key
        for key in keys {
            if let Some(value) = self.get_cached(key).await {
                results.insert(key.clone(), value);
            } else {
                to_load.push(key.clone());
            }
        }
        
        // Load missing keys
        if !to_load.is_empty() {
            let futures: Vec<_> = to_load.iter()
                .map(|key| self.load_one(key.clone()))
                .collect();
            
            let loaded = join_all(futures).await;
            
            for (key, result) in to_load.into_iter().zip(loaded) {
                if let Ok(value) = result {
                    results.insert(key, value);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get value from cache if present and not expired
    async fn get_cached(&self, key: &L::Key) -> Option<Arc<L::Value>> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }
    
    /// Add key to pending batch
    async fn add_to_pending(&self, key: L::Key) -> oneshot::Receiver<Arc<L::Value>> {
        let (tx, rx) = oneshot::channel();
        
        let mut pending = self.pending.lock().await;
        pending.entry(key).or_insert_with(Vec::new).push(tx);
        
        rx
    }
    
    /// Schedule batch execution if not already scheduled
    async fn schedule_batch(&self) {
        let mut scheduled = self.batch_scheduled.lock().await;
        
        if !*scheduled {
            *scheduled = true;
            
            let loader = self.clone();
            tokio::spawn(async move {
                // Wait for batch delay to allow aggregation
                tokio::time::sleep(loader.config.batch_delay).await;
                loader.execute_batch().await;
            });
        }
    }
    
    /// Execute pending batch
    async fn execute_batch(&self) {
        // Reset scheduled flag
        {
            let mut scheduled = self.batch_scheduled.lock().await;
            *scheduled = false;
        }
        
        // Get all pending keys
        let pending_map = {
            let mut pending = self.pending.lock().await;
            std::mem::take(&mut *pending)
        };
        
        if pending_map.is_empty() {
            return;
        }
        
        let keys: Vec<L::Key> = pending_map.keys().cloned().collect();
        
        // Split into chunks based on max batch size
        for chunk in keys.chunks(self.config.max_batch_size) {
            self.execute_batch_chunk(chunk, &pending_map).await;
        }
    }
    
    /// Execute a single batch chunk
    async fn execute_batch_chunk(
        &self,
        keys: &[L::Key],
        pending_map: &HashMap<L::Key, Vec<oneshot::Sender<Arc<L::Value>>>>,
    ) {
        self.metrics.batches_executed.fetch_add(1, Ordering::Relaxed);
        self.metrics.keys_per_batch.fetch_add(keys.len() as u64, Ordering::Relaxed);
        
        // Execute batch load
        match self.loader.load_batch(keys.to_vec()).await {
            Ok(results) => {
                // Update cache and notify waiters
                let mut cache = self.cache.write().await;
                
                for key in keys {
                    if let Some(value) = results.get(key) {
                        let arc_value = Arc::new(value.clone());
                        cache.insert(key.clone(), arc_value.clone());
                        
                        // Notify all waiters for this key
                        if let Some(senders) = pending_map.get(key) {
                            for sender in senders {
                                let _ = sender.send(arc_value.clone());
                            }
                        }
                    } else {
                        // Key not found in results - notify with error
                        if let Some(senders) = pending_map.get(key) {
                            for sender in senders {
                                // Sender dropped means receiver cancelled
                                drop(sender);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Batch failed - notify all waiters
                for senders in pending_map.values() {
                    for sender in senders {
                        drop(sender);
                    }
                }
            }
        }
    }
    
    /// Clear the cache (useful between requests)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
    
    /// Get metrics
    pub fn metrics(&self) -> DataLoaderStats {
        DataLoaderStats {
            total_loads: self.metrics.total_loads.load(Ordering::Relaxed),
            cache_hits: self.metrics.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.metrics.cache_misses.load(Ordering::Relaxed),
            batches_executed: self.metrics.batches_executed.load(Ordering::Relaxed),
            avg_keys_per_batch: if self.metrics.batches_executed.load(Ordering::Relaxed) > 0 {
                self.metrics.keys_per_batch.load(Ordering::Relaxed) as f64 / 
                self.metrics.batches_executed.load(Ordering::Relaxed) as f64
            } else {
                0.0
            },
        }
    }
    
    fn make_error(&self, msg: &str) -> L::Error {
        // This would need to be implemented based on the actual error type
        panic!("Error creation not implemented: {}", msg)
    }
}

impl<L: BatchLoader> Clone for DataLoader<L> {
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
            config: self.config.clone(),
            cache: self.cache.clone(),
            pending: self.pending.clone(),
            batch_scheduled: self.batch_scheduled.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

#[derive(Debug)]
pub struct DataLoaderStats {
    pub total_loads: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub batches_executed: u64,
    pub avg_keys_per_batch: f64,
}

/// Example: User loader implementation
pub struct UserLoader {
    db: Arc<DatabaseService>,
}

impl UserLoader {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BatchLoader for UserLoader {
    type Key = String;
    type Value = User;
    type Error = DataLoaderError;
    
    async fn load_batch(
        &self,
        keys: Vec<Self::Key>,
    ) -> Result<HashMap<Self::Key, Self::Value>, Self::Error> {
        tracing::debug!("Loading users batch: {} keys", keys.len());
        
        // Single database query for all users
        let users = self.db
            .get_users_by_ids(&keys)
            .await
            .map_err(|e| DataLoaderError::Database(e.to_string()))?;
        
        // Convert to HashMap for O(1) lookup
        let mut result = HashMap::new();
        for user in users {
            result.insert(user.id.clone(), user);
        }
        
        Ok(result)
    }
}

/// Example: Note loader with relationship pre-loading
pub struct NoteLoader {
    db: Arc<DatabaseService>,
}

#[async_trait]
impl BatchLoader for NoteLoader {
    type Key = String;
    type Value = Note;
    type Error = DataLoaderError;
    
    async fn load_batch(
        &self,
        keys: Vec<Self::Key>,
    ) -> Result<HashMap<Self::Key, Self::Value>, Self::Error> {
        // Load notes with their relationships in single query
        let notes = self.db
            .get_notes_by_ids_with_relations(&keys)
            .await
            .map_err(|e| DataLoaderError::Database(e.to_string()))?;
        
        let mut result = HashMap::new();
        for note in notes {
            result.insert(note.id.clone(), note);
        }
        
        Ok(result)
    }
}

/// GraphQL Context with DataLoaders
pub struct GraphQLContext {
    pub user_loader: DataLoader<UserLoader>,
    pub note_loader: DataLoader<NoteLoader>,
    pub tag_loader: DataLoader<TagLoader>,
    pub db: Arc<DatabaseService>,
}

impl GraphQLContext {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        Self {
            user_loader: DataLoader::new(UserLoader::new(db.clone())),
            note_loader: DataLoader::new(NoteLoader::new(db.clone())),
            tag_loader: DataLoader::new(TagLoader::new(db.clone())),
            db,
        }
    }
    
    /// Clear all caches (call between requests)
    pub async fn clear_caches(&self) {
        self.user_loader.clear_cache().await;
        self.note_loader.clear_cache().await;
        self.tag_loader.clear_cache().await;
    }
}

/// Example resolver using DataLoader
pub mod resolvers {
    use super::*;
    
    pub struct Note {
        pub id: String,
        pub title: String,
        pub content: String,
        pub author_id: String,
        pub tag_ids: Vec<String>,
    }
    
    impl Note {
        /// Resolve author relationship without N+1
        pub async fn author(&self, ctx: &Context<'_>) -> Result<User, Error> {
            let context = ctx.data::<GraphQLContext>()?;
            
            context.user_loader
                .load_one(self.author_id.clone())
                .await
                .map(|arc| (*arc).clone())
                .map_err(|e| Error::new(e.to_string()))
        }
        
        /// Resolve tags relationship without N+1
        pub async fn tags(&self, ctx: &Context<'_>) -> Result<Vec<Tag>, Error> {
            let context = ctx.data::<GraphQLContext>()?;
            
            let tag_map = context.tag_loader
                .load_many(&self.tag_ids)
                .await
                .map_err(|e| Error::new(e.to_string()))?;
            
            // Preserve order
            let tags: Vec<Tag> = self.tag_ids.iter()
                .filter_map(|id| tag_map.get(id).map(|arc| (*arc).clone()))
                .collect();
            
            Ok(tags)
        }
    }
    
    pub struct Query;
    
    impl Query {
        /// Get all notes with pre-loaded authors to prevent N+1
        pub async fn notes(&self, ctx: &Context<'_>) -> Result<Vec<Note>, Error> {
            let context = ctx.data::<GraphQLContext>()?;
            
            // Get all notes
            let notes = context.db.get_all_notes().await?;
            
            // Pre-load all authors to warm cache
            let author_ids: Vec<String> = notes.iter()
                .map(|n| n.author_id.clone())
                .collect();
            
            context.user_loader.load_many(&author_ids).await?;
            
            Ok(notes)
        }
    }
}

/// N+1 Detection Helper
pub mod n_plus_one_detection {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    
    /// Wrapper that tracks database calls for N+1 detection
    pub struct N1DetectionWrapper<T> {
        inner: T,
        call_count: Arc<AtomicUsize>,
    }
    
    impl<T> N1DetectionWrapper<T> {
        pub fn new(inner: T) -> Self {
            Self {
                inner,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }
        
        pub fn call_count(&self) -> usize {
            self.call_count.load(Ordering::Relaxed)
        }
        
        pub fn reset_count(&self) {
            self.call_count.store(0, Ordering::Relaxed);
        }
    }
    
    /// Test helper to detect N+1 queries
    pub async fn test_for_n_plus_one<F, Fut>(
        query: &str,
        executor: F,
    ) -> Result<N1Report, Box<dyn std::error::Error>>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<serde_json::Value, Box<dyn std::error::Error>>>,
    {
        // Execute query and count DB calls
        let start_count = get_db_call_count();
        let result = executor().await?;
        let total_calls = get_db_call_count() - start_count;
        
        // Analyze result structure
        let entity_count = count_entities(&result);
        
        // Detect N+1 pattern
        let expected_calls = 1 + unique_relationships(&result);
        let has_n_plus_one = total_calls > expected_calls * 2; // Allow some margin
        
        Ok(N1Report {
            query: query.to_string(),
            total_db_calls: total_calls,
            entity_count,
            expected_calls,
            has_n_plus_one,
            efficiency_ratio: expected_calls as f64 / total_calls as f64,
        })
    }
    
    #[derive(Debug)]
    pub struct N1Report {
        pub query: String,
        pub total_db_calls: usize,
        pub entity_count: usize,
        pub expected_calls: usize,
        pub has_n_plus_one: bool,
        pub efficiency_ratio: f64,
    }
    
    fn get_db_call_count() -> usize {
        // Implementation would track actual DB calls
        0
    }
    
    fn count_entities(result: &serde_json::Value) -> usize {
        // Count entities in GraphQL result
        0
    }
    
    fn unique_relationships(result: &serde_json::Value) -> usize {
        // Count unique relationships accessed
        0
    }
}

/// Common types used in examples
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DataLoaderError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Not found")]
    NotFound,
    
    #[error("Batch execution failed: {0}")]
    BatchFailed(String),
}

/// Mock database service for examples
pub struct DatabaseService;

impl DatabaseService {
    async fn get_users_by_ids(&self, ids: &[String]) -> Result<Vec<User>, Box<dyn std::error::Error>> {
        // Mock implementation
        Ok(vec![])
    }
    
    async fn get_notes_by_ids_with_relations(&self, ids: &[String]) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
        // Mock implementation
        Ok(vec![])
    }
    
    async fn get_all_notes(&self) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
        // Mock implementation
        Ok(vec![])
    }
}

pub struct TagLoader {
    db: Arc<DatabaseService>,
}

impl TagLoader {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BatchLoader for TagLoader {
    type Key = String;
    type Value = Tag;
    type Error = DataLoaderError;
    
    async fn load_batch(
        &self,
        keys: Vec<Self::Key>,
    ) -> Result<HashMap<Self::Key, Self::Value>, Self::Error> {
        // Mock implementation
        Ok(HashMap::new())
    }
}

type Error = Box<dyn std::error::Error>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dataloader_batching() {
        let db = Arc::new(DatabaseService);
        let loader = DataLoader::new(UserLoader::new(db));
        
        // Load multiple keys concurrently
        let futures = vec![
            loader.load_one("user1".to_string()),
            loader.load_one("user2".to_string()),
            loader.load_one("user3".to_string()),
        ];
        
        let results = join_all(futures).await;
        
        // Check metrics
        let stats = loader.metrics();
        assert_eq!(stats.total_loads, 3);
        assert_eq!(stats.batches_executed, 1); // All batched together
    }
    
    #[tokio::test]
    async fn test_cache_deduplication() {
        let db = Arc::new(DatabaseService);
        let loader = DataLoader::new(UserLoader::new(db));
        
        // Load same key multiple times
        let _user1 = loader.load_one("user1".to_string()).await;
        let _user2 = loader.load_one("user1".to_string()).await;
        
        let stats = loader.metrics();
        assert_eq!(stats.cache_hits, 1); // Second load hit cache
    }
}
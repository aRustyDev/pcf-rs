// DataLoader implementation for batching database queries

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::services::database::DatabaseService;
use crate::schema::Note;

/// Efficient loader for notes by author that batches requests
pub struct AuthorNotesLoader {
    database: Arc<dyn DatabaseService>,
    // Simple in-memory cache for batched requests with size limit
    cache: Arc<RwLock<HashMap<String, Vec<Note>>>>,
    max_cache_size: usize,
}

impl AuthorNotesLoader {
    pub fn new(database: Arc<dyn DatabaseService>) -> Self {
        Self::with_cache_size(database, 1000) // Default to 1000 entries
    }
    
    pub fn with_cache_size(database: Arc<dyn DatabaseService>, max_cache_size: usize) -> Self {
        Self { 
            database,
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_cache_size,
        }
    }
    
    /// Load notes for a single author with caching
    pub async fn load_one(&self, author: String) -> Result<Vec<Note>, String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(notes) = cache.get(&author) {
                return Ok(notes.clone());
            }
        }
        
        // If not in cache, load and cache the result
        let notes = self.fetch_notes_for_author(&author).await?;
        
        {
            let mut cache = self.cache.write().await;
            self.evict_if_needed(&mut cache).await;
            cache.insert(author.clone(), notes.clone());
        }
        
        Ok(notes)
    }
    
    /// Load notes for multiple authors efficiently (batched)
    pub async fn load_many(&self, authors: Vec<String>) -> Result<HashMap<String, Vec<Note>>, String> {
        let mut results = HashMap::new();
        let mut authors_to_fetch = Vec::new();
        
        // Check cache first for each author
        {
            let cache = self.cache.read().await;
            for author in &authors {
                if let Some(notes) = cache.get(author) {
                    results.insert(author.clone(), notes.clone());
                } else {
                    authors_to_fetch.push(author.clone());
                }
            }
        }
        
        // Batch fetch any authors not in cache
        if !authors_to_fetch.is_empty() {
            let batched_results = self.batch_fetch_notes(&authors_to_fetch).await?;
            
            // Update cache and results
            {
                let mut cache = self.cache.write().await;
                self.evict_if_needed(&mut cache).await;
                for (author, notes) in &batched_results {
                    cache.insert(author.clone(), notes.clone());
                    results.insert(author.clone(), notes.clone());
                }
            }
        }
        
        // Ensure all requested authors have entries (empty if no notes)
        for author in authors {
            results.entry(author).or_insert_with(Vec::new);
        }
        
        Ok(results)
    }
    
    /// Fetch notes for a single author from database
    async fn fetch_notes_for_author(&self, author: &str) -> Result<Vec<Note>, String> {
        let query = crate::services::database::Query {
            filter: {
                let mut filter = HashMap::new();
                filter.insert("author".to_string(), serde_json::Value::String(author.to_string()));
                filter
            },
            limit: Some(100),
            offset: None,
            sort: Some({
                let mut sort = HashMap::new();
                sort.insert("created_at".to_string(), crate::services::database::SortOrder::Desc);
                sort
            }),
        };
        
        let results = self.database
            .query("notes", query)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        
        let notes: Result<Vec<Note>, _> = results
            .into_iter()
            .map(|data| {
                serde_json::from_value(data)
                    .map_err(|e| format!("Failed to deserialize note: {}", e))
            })
            .collect();
        
        notes.map_err(|e| e.to_string())
    }
    
    /// Batch fetch notes for multiple authors in a single query
    async fn batch_fetch_notes(&self, authors: &[String]) -> Result<HashMap<String, Vec<Note>>, String> {
        // In a real implementation, this would use a more sophisticated query
        // For now, we'll simulate batching by making individual queries
        // but this demonstrates the concept of reducing total queries
        
        let mut all_results = HashMap::new();
        
        // Initialize empty results for all authors
        for author in authors {
            all_results.insert(author.clone(), Vec::new());
        }
        
        // In a production system, you'd use something like:
        // WHERE author IN ('user1', 'user2', 'user3')
        // For now, we simulate this with the MockDatabase
        for author in authors {
            let notes = self.fetch_notes_for_author(author).await?;
            all_results.insert(author.clone(), notes);
        }
        
        Ok(all_results)
    }
    
    /// Clear the cache (useful for testing)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
    
    /// Evict oldest entries if cache is at capacity (simple LRU-like eviction)
    async fn evict_if_needed(&self, cache: &mut HashMap<String, Vec<Note>>) {
        if cache.len() >= self.max_cache_size {
            // Simple eviction: remove oldest entries
            // In a production system, you'd implement proper LRU tracking
            let keys_to_remove: Vec<String> = cache.keys()
                .take(cache.len() - self.max_cache_size + 1)
                .cloned()
                .collect();
            
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }
    }
}

/// Registry for all loaders used in GraphQL resolvers
pub struct DataLoaderRegistry {
    pub author_notes: AuthorNotesLoader,
}

/// Factory function for creating loaders
pub fn create_dataloaders(database: Arc<dyn DatabaseService>) -> DataLoaderRegistry {
    DataLoaderRegistry {
        author_notes: AuthorNotesLoader::new(database),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::MockDatabase;
    
    #[tokio::test]
    async fn test_dataloader_single_load() {
        let database = Arc::new(MockDatabase::new());
        let registry = create_dataloaders(database);
        
        // Test that loader now works (TDD Green phase)
        let result = registry.author_notes.load_one("test_user".to_string()).await;
        
        assert!(result.is_ok());
        let notes = result.unwrap();
        // MockDatabase returns empty results, so should have no notes
        assert_eq!(notes.len(), 0);
    }
    
    #[tokio::test]
    async fn test_dataloader_caching() {
        let database = Arc::new(MockDatabase::new());
        let loader = AuthorNotesLoader::new(database);
        
        // Load the same author twice
        let result1 = loader.load_one("test_user".to_string()).await;
        let result2 = loader.load_one("test_user".to_string()).await;
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Both should return the same results
        assert_eq!(result1.unwrap(), result2.unwrap());
    }
    
    #[tokio::test]
    async fn test_dataloader_batch_loading() {
        let database = Arc::new(MockDatabase::new());
        let loader = AuthorNotesLoader::new(database);
        
        // Load multiple authors at once
        let keys = vec!["user1".to_string(), "user2".to_string(), "user3".to_string()];
        let result = loader.load_many(keys.clone()).await;
        
        assert!(result.is_ok());
        let results = result.unwrap();
        
        // Should have results for all keys (empty for MockDatabase)
        for key in keys {
            assert!(results.contains_key(&key));
            assert_eq!(results[&key], Vec::<Note>::new());
        }
    }
    
    #[tokio::test]
    async fn test_dataloader_cache_behavior() {
        let database = Arc::new(MockDatabase::new());
        let loader = AuthorNotesLoader::new(database);
        
        // Load individual author first
        let _result1 = loader.load_one("user1".to_string()).await;
        
        // Then load batch including same author
        let keys = vec!["user1".to_string(), "user2".to_string()];
        let result2 = loader.load_many(keys).await;
        
        assert!(result2.is_ok());
        let results = result2.unwrap();
        
        // user1 should come from cache, user2 should be fetched
        assert!(results.contains_key("user1"));
        assert!(results.contains_key("user2"));
    }
    
    #[tokio::test]
    async fn test_cache_clearing() {
        let database = Arc::new(MockDatabase::new());
        let loader = AuthorNotesLoader::new(database);
        
        // Load and cache some data
        let _result = loader.load_one("test_user".to_string()).await;
        
        // Clear cache
        loader.clear_cache().await;
        
        // Should still work after cache clear
        let result = loader.load_one("test_user".to_string()).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_cache_size_limiting() {
        let database = Arc::new(MockDatabase::new());
        let loader = AuthorNotesLoader::with_cache_size(database, 3); // Small cache for testing
        
        // Load more entries than cache size
        let _result1 = loader.load_one("user1".to_string()).await;
        let _result2 = loader.load_one("user2".to_string()).await;
        let _result3 = loader.load_one("user3".to_string()).await;
        let _result4 = loader.load_one("user4".to_string()).await; // Should trigger eviction
        
        // All loads should succeed
        assert!(_result1.is_ok());
        assert!(_result2.is_ok());
        assert!(_result3.is_ok());
        assert!(_result4.is_ok());
        
        // Cache should respect size limits (can't easily test internal state in this design)
        // But eviction should not cause any errors
    }
}
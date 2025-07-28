/// Connection Pool Examples - Phase 2 Database Implementation
///
/// This file demonstrates connection pool patterns for reliable database connectivity.
/// Includes health monitoring, connection lifecycle, and metrics collection.

use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use async_trait::async_trait;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain
    pub min_connections: usize,
    /// Maximum number of connections allowed
    pub max_connections: usize,
    /// How long a connection can be idle before removal
    pub idle_timeout: Duration,
    /// How long to wait for a connection before timing out
    pub acquire_timeout: Duration,
    /// How often to run health checks
    pub health_check_interval: Duration,
    /// Maximum connection lifetime
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            idle_timeout: Duration::from_secs(300),      // 5 minutes
            acquire_timeout: Duration::from_secs(30),     // 30 seconds
            health_check_interval: Duration::from_secs(10), // 10 seconds
            max_lifetime: Duration::from_secs(3600),      // 1 hour
        }
    }
}

/// Represents a pooled connection
pub struct PooledConnection<C> {
    conn: Option<C>,
    pool: Arc<ConnectionPoolInner<C>>,
    created_at: Instant,
    last_used: Instant,
    id: u64,
}

impl<C> PooledConnection<C> {
    /// Get a reference to the underlying connection
    pub fn get(&self) -> &C {
        self.conn.as_ref().expect("Connection already returned to pool")
    }
    
    /// Get a mutable reference to the underlying connection
    pub fn get_mut(&mut self) -> &mut C {
        self.conn.as_mut().expect("Connection already returned to pool")
    }
}

impl<C> Drop for PooledConnection<C> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let pool = self.pool.clone();
            let id = self.id;
            
            // Return connection to pool asynchronously
            tokio::spawn(async move {
                pool.return_connection(conn, id).await;
            });
        }
    }
}

/// Trait for connections that can be pooled
#[async_trait]
pub trait Poolable: Send + Sync + 'static {
    /// Create a new connection
    async fn create() -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where Self: Sized;
    
    /// Check if the connection is healthy
    async fn is_healthy(&self) -> bool;
    
    /// Close the connection cleanly
    async fn close(self);
}

/// Connection pool implementation
pub struct ConnectionPool<C: Poolable> {
    inner: Arc<ConnectionPoolInner<C>>,
}

struct ConnectionPoolInner<C> {
    /// Available connections
    available: Mutex<VecDeque<(C, Instant, u64)>>,
    /// Semaphore to limit total connections
    semaphore: Semaphore,
    /// Configuration
    config: PoolConfig,
    /// Connection ID counter
    next_id: Mutex<u64>,
    /// Total connections (active + available)
    total_connections: Mutex<usize>,
}

impl<C: Poolable> ConnectionPool<C> {
    pub fn new(config: PoolConfig) -> Self {
        let inner = Arc::new(ConnectionPoolInner {
            available: Mutex::new(VecDeque::new()),
            semaphore: Semaphore::new(config.max_connections),
            config,
            next_id: Mutex::new(0),
            total_connections: Mutex::new(0),
        });
        
        let pool = Self { inner };
        
        // Start background tasks
        pool.start_maintenance_task();
        pool.start_health_check_task();
        
        // Pre-warm the pool
        let warming_pool = pool.clone();
        tokio::spawn(async move {
            warming_pool.warm_pool().await;
        });
        
        pool
    }
    
    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnection<C>, PoolError> {
        let start = Instant::now();
        
        // Try to get an existing connection first
        if let Some((conn, _, id)) = self.get_available_connection().await {
            return Ok(PooledConnection {
                conn: Some(conn),
                pool: self.inner.clone(),
                created_at: Instant::now(),
                last_used: Instant::now(),
                id,
            });
        }
        
        // Need to create a new connection
        match tokio::time::timeout(
            self.inner.config.acquire_timeout,
            self.create_new_connection()
        ).await {
            Ok(Ok((conn, id))) => Ok(PooledConnection {
                conn: Some(conn),
                pool: self.inner.clone(),
                created_at: Instant::now(),
                last_used: Instant::now(),
                id,
            }),
            Ok(Err(e)) => Err(PoolError::ConnectionError(e.to_string())),
            Err(_) => Err(PoolError::AcquireTimeout),
        }
    }
    
    /// Get pool health information
    pub async fn health(&self) -> PoolHealth {
        let available = self.inner.available.lock().await.len();
        let total = *self.inner.total_connections.lock().await;
        let active = total - available;
        
        PoolHealth {
            total_connections: total,
            active_connections: active,
            available_connections: available,
            max_connections: self.inner.config.max_connections,
        }
    }
    
    async fn get_available_connection(&self) -> Option<(C, Instant, u64)> {
        let mut available = self.inner.available.lock().await;
        
        while let Some((conn, created_at, id)) = available.pop_front() {
            // Check if connection is still valid
            if created_at.elapsed() > self.inner.config.max_lifetime {
                // Connection too old, close it
                drop(available); // Release lock
                self.close_connection(conn).await;
                available = self.inner.available.lock().await;
                continue;
            }
            
            // Found a good connection
            return Some((conn, created_at, id));
        }
        
        None
    }
    
    async fn create_new_connection(&self) -> Result<(C, u64), Box<dyn std::error::Error + Send + Sync>> {
        // Acquire permit to create new connection
        let _permit = self.inner.semaphore.acquire().await?;
        
        // Create the connection
        let conn = C::create().await?;
        
        // Get next ID
        let id = {
            let mut next_id = self.inner.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        // Update total count
        *self.inner.total_connections.lock().await += 1;
        
        Ok((conn, id))
    }
    
    async fn close_connection(&self, conn: C) {
        conn.close().await;
        *self.inner.total_connections.lock().await -= 1;
    }
    
    /// Pre-warm the pool with minimum connections
    async fn warm_pool(&self) {
        let target = self.inner.config.min_connections;
        
        for _ in 0..target {
            match self.create_new_connection().await {
                Ok((conn, id)) => {
                    self.inner.available.lock().await.push_back((conn, Instant::now(), id));
                }
                Err(e) => {
                    tracing::warn!("Failed to pre-warm connection: {}", e);
                    break;
                }
            }
        }
    }
    
    /// Start background task to maintain pool health
    fn start_maintenance_task(&self) {
        let pool = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                pool.perform_maintenance().await;
            }
        });
    }
    
    /// Start background task for health checks
    fn start_health_check_task(&self) {
        let pool = self.clone();
        let interval_duration = pool.inner.config.health_check_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval_duration);
            
            loop {
                interval.tick().await;
                pool.check_connection_health().await;
            }
        });
    }
    
    async fn perform_maintenance(&self) {
        let mut available = self.inner.available.lock().await;
        let mut to_close = Vec::new();
        
        // Remove idle connections beyond minimum
        while available.len() > self.inner.config.min_connections {
            if let Some((conn, created_at, _)) = available.back() {
                if created_at.elapsed() > self.inner.config.idle_timeout {
                    if let Some((conn, _, _)) = available.pop_back() {
                        to_close.push(conn);
                    }
                } else {
                    break; // Connections are ordered, so we can stop
                }
            } else {
                break;
            }
        }
        
        drop(available); // Release lock before closing connections
        
        for conn in to_close {
            self.close_connection(conn).await;
        }
    }
    
    async fn check_connection_health(&self) {
        let mut available = self.inner.available.lock().await;
        let mut healthy_connections = VecDeque::new();
        let mut to_close = Vec::new();
        
        // Check each available connection
        while let Some((conn, created_at, id)) = available.pop_front() {
            if conn.is_healthy().await {
                healthy_connections.push_back((conn, created_at, id));
            } else {
                to_close.push(conn);
            }
        }
        
        // Put healthy connections back
        *available = healthy_connections;
        
        drop(available); // Release lock
        
        // Close unhealthy connections
        for conn in to_close {
            self.close_connection(conn).await;
        }
        
        // Ensure minimum connections
        let current_total = *self.inner.total_connections.lock().await;
        if current_total < self.inner.config.min_connections {
            let warming_pool = self.clone();
            tokio::spawn(async move {
                warming_pool.warm_pool().await;
            });
        }
    }
}

impl<C: Poolable> Clone for ConnectionPool<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<C> ConnectionPoolInner<C> {
    async fn return_connection(&self, conn: C, id: u64) {
        // Check if we should keep this connection
        let total = *self.total_connections.lock().await;
        
        if total > self.config.max_connections {
            // Too many connections, close this one
            conn.close().await;
            *self.total_connections.lock().await -= 1;
        } else {
            // Return to available pool
            self.available.lock().await.push_back((conn, Instant::now(), id));
        }
        
        // Release the semaphore permit
        self.semaphore.add_permits(1);
    }
}

/// Pool health information
#[derive(Debug, Clone)]
pub struct PoolHealth {
    pub total_connections: usize,
    pub active_connections: usize,
    pub available_connections: usize,
    pub max_connections: usize,
}

impl PoolHealth {
    pub fn is_healthy(&self) -> bool {
        self.total_connections > 0 && self.total_connections <= self.max_connections
    }
    
    pub fn utilization(&self) -> f64 {
        if self.max_connections == 0 {
            0.0
        } else {
            self.active_connections as f64 / self.max_connections as f64
        }
    }
}

/// Pool errors
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Failed to acquire connection within timeout")]
    AcquireTimeout,
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Pool is shutting down")]
    PoolShuttingDown,
}

/// Example implementation for SurrealDB
pub mod surrealdb_example {
    use super::*;
    use surrealdb::{Surreal, engine::remote::ws::{Client, Ws}};
    
    pub struct SurrealConnection {
        client: Surreal<Client>,
        endpoint: String,
    }
    
    #[async_trait]
    impl Poolable for SurrealConnection {
        async fn create() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            let endpoint = std::env::var("SURREALDB_ENDPOINT")
                .unwrap_or_else(|_| "ws://localhost:8000".to_string());
            
            let client = Surreal::new::<Ws>(&endpoint).await?;
            
            // Authenticate and select namespace/database
            client.use_ns("production").use_db("api").await?;
            
            Ok(Self { client, endpoint })
        }
        
        async fn is_healthy(&self) -> bool {
            // Perform a simple query to check connection
            match self.client.query("SELECT 1").await {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        
        async fn close(self) {
            // SurrealDB client closes on drop
            drop(self.client);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock connection for testing
    struct MockConnection {
        id: u64,
        healthy: bool,
    }
    
    #[async_trait]
    impl Poolable for MockConnection {
        async fn create() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            Ok(Self {
                id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
                healthy: true,
            })
        }
        
        async fn is_healthy(&self) -> bool {
            self.healthy
        }
        
        async fn close(self) {
            // Mock cleanup
        }
    }
    
    #[tokio::test]
    async fn test_pool_basic_operations() {
        let config = PoolConfig {
            min_connections: 2,
            max_connections: 5,
            ..Default::default()
        };
        
        let pool = ConnectionPool::<MockConnection>::new(config);
        
        // Wait for pre-warming
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check health
        let health = pool.health().await;
        assert_eq!(health.total_connections, 2);
        assert_eq!(health.available_connections, 2);
        
        // Acquire a connection
        let conn = pool.acquire().await.unwrap();
        assert!(conn.get().is_healthy().await);
        
        // Check health again
        let health = pool.health().await;
        assert_eq!(health.active_connections, 1);
        assert_eq!(health.available_connections, 1);
        
        // Drop connection (returns to pool)
        drop(conn);
        
        // Give time for async return
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Should be back in pool
        let health = pool.health().await;
        assert_eq!(health.available_connections, 2);
    }
}
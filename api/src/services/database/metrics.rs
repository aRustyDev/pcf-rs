use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsSnapshot {
    pub pool_size: u64,
    pub active_connections: u64,
    pub idle_connections: u64,
    pub failed_connections: u64,
    pub query_count: u64,
    pub connection_errors: u64,
}

/// Simple metrics collector for database operations
/// This is a basic implementation that can be extended with Prometheus later
pub struct DatabaseMetrics {
    pub connection_pool_size: AtomicU64,
    pub active_connections: AtomicU64,
    pub idle_connections: AtomicU64,
    pub failed_connections: AtomicU64,
    pub query_count: AtomicU64,
    pub connection_errors: AtomicU64,
}

impl DatabaseMetrics {
    pub fn new() -> Self {
        Self {
            connection_pool_size: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            idle_connections: AtomicU64::new(0),
            failed_connections: AtomicU64::new(0),
            query_count: AtomicU64::new(0),
            connection_errors: AtomicU64::new(0),
        }
    }
    
    pub fn set_pool_size(&self, size: u64) {
        self.connection_pool_size.store(size, Ordering::Relaxed);
    }
    
    pub fn set_active_connections(&self, count: u64) {
        self.active_connections.store(count, Ordering::Relaxed);
    }
    
    pub fn set_idle_connections(&self, count: u64) {
        self.idle_connections.store(count, Ordering::Relaxed);
    }
    
    pub fn increment_failed_connections(&self) {
        self.failed_connections.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_query_count(&self) {
        self.query_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_connection_errors(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_pool_size(&self) -> u64 {
        self.connection_pool_size.load(Ordering::Relaxed)
    }
    
    pub fn get_active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }
    
    pub fn get_idle_connections(&self) -> u64 {
        self.idle_connections.load(Ordering::Relaxed)
    }
    
    pub fn get_failed_connections(&self) -> u64 {
        self.failed_connections.load(Ordering::Relaxed)
    }
    
    pub fn get_query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }
    
    pub fn get_connection_errors(&self) -> u64 {
        self.connection_errors.load(Ordering::Relaxed)
    }
}

impl Default for DatabaseMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Feature-flagged metrics collection
#[cfg(feature = "metrics-basic")]
pub mod feature_metrics {
    use lazy_static::lazy_static;
    use std::sync::Arc;
    use super::{DatabaseMetrics, MetricsSnapshot};
    
    lazy_static! {
        pub static ref DATABASE_METRICS: Arc<DatabaseMetrics> = Arc::new(DatabaseMetrics::new());
    }
    
    pub fn record_pool_size(size: u64) {
        DATABASE_METRICS.set_pool_size(size);
    }
    
    pub fn record_active_connections(count: u64) {
        DATABASE_METRICS.set_active_connections(count);
    }
    
    pub fn record_idle_connections(count: u64) {
        DATABASE_METRICS.set_idle_connections(count);
    }
    
    pub fn increment_failed_connections() {
        DATABASE_METRICS.increment_failed_connections();
    }
    
    pub fn increment_query_count() {
        DATABASE_METRICS.increment_query_count();
    }
    
    pub fn increment_connection_errors() {
        DATABASE_METRICS.increment_connection_errors();
    }
    
    pub fn increment_operations(_collection: &str, _operation: &str) {
        // Placeholder for operation counting
        // In a real implementation, this would increment counters by collection and operation type
        DATABASE_METRICS.increment_query_count();
    }
    
    /// Get a snapshot of current metrics
    pub fn get_metrics_snapshot() -> MetricsSnapshot {
        MetricsSnapshot {
            pool_size: DATABASE_METRICS.get_pool_size(),
            active_connections: DATABASE_METRICS.get_active_connections(),
            idle_connections: DATABASE_METRICS.get_idle_connections(),
            failed_connections: DATABASE_METRICS.get_failed_connections(),
            query_count: DATABASE_METRICS.get_query_count(),
            connection_errors: DATABASE_METRICS.get_connection_errors(),
        }
    }
}

/// No-op metrics collection when feature is disabled
#[cfg(not(feature = "metrics-basic"))]
pub mod feature_metrics {
    use super::MetricsSnapshot;
    
    pub fn record_pool_size(_size: u64) {}
    pub fn record_active_connections(_count: u64) {}
    pub fn record_idle_connections(_count: u64) {}
    pub fn increment_failed_connections() {}
    pub fn increment_query_count() {}
    pub fn increment_connection_errors() {}
    pub fn increment_operations(_collection: &str, _operation: &str) {}
    
    pub fn get_metrics_snapshot() -> MetricsSnapshot {
        MetricsSnapshot {
            pool_size: 0,
            active_connections: 0,
            idle_connections: 0,
            failed_connections: 0,
            query_count: 0,
            connection_errors: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_database_metrics_basic_operations() {
        let metrics = DatabaseMetrics::new();
        
        // Test initial values
        assert_eq!(metrics.get_pool_size(), 0);
        assert_eq!(metrics.get_active_connections(), 0);
        assert_eq!(metrics.get_connection_errors(), 0);
        
        // Test setters
        metrics.set_pool_size(10);
        metrics.set_active_connections(5);
        assert_eq!(metrics.get_pool_size(), 10);
        assert_eq!(metrics.get_active_connections(), 5);
        
        // Test increment operations
        metrics.increment_failed_connections();
        metrics.increment_query_count();
        metrics.increment_connection_errors();
        
        assert_eq!(metrics.get_failed_connections(), 1);
        assert_eq!(metrics.get_query_count(), 1);
        assert_eq!(metrics.get_connection_errors(), 1);
        
        // Test multiple increments
        metrics.increment_query_count();
        metrics.increment_query_count();
        assert_eq!(metrics.get_query_count(), 3);
    }
    
    #[test]
    fn test_metrics_thread_safety() {
        let metrics = DatabaseMetrics::new();
        let metrics_ref = &metrics;
        
        // Simulate concurrent access
        std::thread::scope(|s| {
            for _ in 0..5 {
                s.spawn(|| {
                    for _ in 0..10 {
                        metrics_ref.increment_query_count();
                        metrics_ref.increment_connection_errors();
                    }
                });
            }
        });
        
        // Should have 50 queries and 50 errors from 5 threads * 10 iterations each
        assert_eq!(metrics.get_query_count(), 50);
        assert_eq!(metrics.get_connection_errors(), 50);
    }
    
    #[cfg(feature = "metrics-basic")]
    #[test]
    fn test_feature_metrics_enabled() {
        use feature_metrics::*;
        
        // Reset metrics for test
        record_pool_size(0);
        record_active_connections(0);
        
        record_pool_size(15);
        record_active_connections(8);
        increment_query_count();
        
        let snapshot = get_metrics_snapshot();
        assert_eq!(snapshot.pool_size, 15);
        assert_eq!(snapshot.active_connections, 8);
        assert!(snapshot.query_count > 0);
    }
    
    #[cfg(not(feature = "metrics-basic"))]
    #[test]
    fn test_feature_metrics_disabled() {
        use feature_metrics::*;
        
        // When metrics are disabled, all operations should be no-ops
        record_pool_size(15);
        record_active_connections(8);
        increment_query_count();
        
        let snapshot = get_metrics_snapshot();
        assert_eq!(snapshot.pool_size, 0);
        assert_eq!(snapshot.active_connections, 0);
        assert_eq!(snapshot.query_count, 0);
    }
}
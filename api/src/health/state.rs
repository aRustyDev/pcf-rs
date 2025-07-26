use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

/// Health status for individual services
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is fully operational
    Healthy,
    /// Service is operational but with reduced capacity
    Degraded,
    /// Service is not operational
    Unhealthy,
    /// Service is still starting up
    Starting,
}

/// Information about a service's health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Current health status
    pub status: HealthStatus,
    /// Human-readable description
    pub message: String,
    /// Last time this service was checked
    pub last_checked: Option<std::time::SystemTime>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Overall health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status (worst of all services)
    pub status: HealthStatus,
    /// Map of service name to health info
    pub services: HashMap<String, ServiceHealth>,
    /// Timestamp of this health check
    pub timestamp: std::time::SystemTime,
    /// Total time this service has been running
    pub uptime_seconds: u64,
}

/// Cached health check result with TTL
#[derive(Debug, Clone)]
struct CachedHealth {
    response: HealthResponse,
    cached_at: Instant,
    ttl: Duration,
}

impl CachedHealth {
    fn new(response: HealthResponse, ttl: Duration) -> Self {
        Self {
            response,
            cached_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }

    fn is_stale(&self, max_stale: Duration) -> bool {
        self.cached_at.elapsed() > (self.ttl + max_stale)
    }
}

/// Health check manager with caching and state tracking
#[derive(Debug)]
pub struct HealthManager {
    services: Arc<RwLock<HashMap<String, ServiceHealth>>>,
    cache: Arc<RwLock<Option<CachedHealth>>>,
    startup_time: Instant,
    cache_ttl: Duration,
    max_stale_duration: Duration,
    startup_grace_period: Duration,
}

impl HealthManager {
    /// Create a new health manager
    pub fn new() -> Self {
        let mut services = HashMap::new();
        
        // Initialize core services
        services.insert("api".to_string(), ServiceHealth {
            status: HealthStatus::Starting,
            message: "API server starting up".to_string(),
            last_checked: Some(std::time::SystemTime::now()),
            metadata: HashMap::new(),
        });

        Self {
            services: Arc::new(RwLock::new(services)),
            cache: Arc::new(RwLock::new(None)),
            startup_time: Instant::now(),
            cache_ttl: Duration::from_secs(5),
            max_stale_duration: Duration::from_secs(30),
            startup_grace_period: Duration::from_secs(30),
        }
    }

    /// Mark the API service as ready (called after server starts)
    pub async fn mark_ready(&self) {
        let mut services = self.services.write().await;
        if let Some(api_service) = services.get_mut("api") {
            api_service.status = HealthStatus::Healthy;
            api_service.message = "API server is ready".to_string();
            api_service.last_checked = Some(std::time::SystemTime::now());
        }
        
        // Clear cache when status changes
        *self.cache.write().await = None;
        info!("API service marked as ready");
    }

    /// Update the health status of a service
    pub async fn update_service_health(&self, service_name: &str, status: HealthStatus, message: String) {
        let mut services = self.services.write().await;
        let mut metadata = HashMap::new();
        metadata.insert("updated_at".to_string(), chrono::Utc::now().to_rfc3339());

        services.insert(service_name.to_string(), ServiceHealth {
            status: status.clone(),
            message,
            last_checked: Some(std::time::SystemTime::now()),
            metadata,
        });

        // Clear cache when status changes
        *self.cache.write().await = None;
        info!("Updated health status for service '{}': {:?}", service_name, status);
    }

    /// Get the current health status with caching
    pub async fn get_health(&self) -> HealthResponse {
        // Check if we have valid cached data
        let cache_guard = self.cache.read().await;
        if let Some(cached) = &*cache_guard {
            if !cached.is_expired() {
                return cached.response.clone();
            }
            // If expired but not stale, we can still use it while updating in background
            if !cached.is_stale(self.max_stale_duration) {
                let stale_response = cached.response.clone();
                drop(cache_guard);
                
                // Update cache in background
                tokio::spawn({
                    let health_manager = self.clone();
                    async move {
                        let _ = health_manager.refresh_health().await;
                    }
                });
                
                return stale_response;
            }
        }
        drop(cache_guard);

        // Need to refresh the health status
        self.refresh_health().await
    }

    /// Refresh health status and update cache
    async fn refresh_health(&self) -> HealthResponse {
        let services = self.services.read().await;
        let services_clone = services.clone();
        drop(services);

        // Determine overall status (worst of all services)
        let overall_status = services_clone.values()
            .map(|s| &s.status)
            .min_by(|a, b| self.status_priority(a).cmp(&self.status_priority(b)))
            .cloned()
            .unwrap_or(HealthStatus::Healthy);

        let response = HealthResponse {
            status: overall_status,
            services: services_clone,
            timestamp: std::time::SystemTime::now(),
            uptime_seconds: self.startup_time.elapsed().as_secs(),
        };

        // Update cache
        let cached = CachedHealth::new(response.clone(), self.cache_ttl);
        *self.cache.write().await = Some(cached);

        response
    }

    /// Get priority for status comparison (lower = worse)
    fn status_priority(&self, status: &HealthStatus) -> u8 {
        match status {
            HealthStatus::Unhealthy => 0,
            HealthStatus::Starting => 1,
            HealthStatus::Degraded => 2,
            HealthStatus::Healthy => 3,
        }
    }

    /// Check if we're still in startup grace period
    pub fn is_in_startup_period(&self) -> bool {
        self.startup_time.elapsed() < self.startup_grace_period
    }
}

impl Clone for HealthManager {
    fn clone(&self) -> Self {
        Self {
            services: Arc::clone(&self.services),
            cache: Arc::clone(&self.cache),
            startup_time: self.startup_time,
            cache_ttl: self.cache_ttl,
            max_stale_duration: self.max_stale_duration,
            startup_grace_period: self.startup_grace_period,
        }
    }
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Tests might use tokio::time later

    #[tokio::test]
    async fn test_health_manager_initialization() {
        let manager = HealthManager::new();
        let health = manager.get_health().await;
        
        assert_eq!(health.status, HealthStatus::Starting);
        assert!(health.services.contains_key("api"));
        assert_eq!(health.services["api"].status, HealthStatus::Starting);
    }

    #[tokio::test]
    async fn test_mark_ready() {
        let manager = HealthManager::new();
        manager.mark_ready().await;
        
        let health = manager.get_health().await;
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.services["api"].status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_service_health_update() {
        let manager = HealthManager::new();
        
        manager.update_service_health("database", HealthStatus::Healthy, "Connected".to_string()).await;
        
        let health = manager.get_health().await;
        assert!(health.services.contains_key("database"));
        assert_eq!(health.services["database"].status, HealthStatus::Healthy);
        assert_eq!(health.services["database"].message, "Connected");
    }

    #[tokio::test]
    async fn test_overall_status_calculation() {
        let manager = HealthManager::new();
        manager.mark_ready().await;
        
        // Add a degraded service
        manager.update_service_health("cache", HealthStatus::Degraded, "High latency".to_string()).await;
        
        let health = manager.get_health().await;
        // Overall status should be degraded (worst of healthy + degraded)
        assert_eq!(health.status, HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_caching_mechanism() {
        let manager = HealthManager::new();
        manager.mark_ready().await;
        
        // First call should compute health
        let health1 = manager.get_health().await;
        let timestamp1 = health1.timestamp;
        
        // Second call within TTL should return cached result
        let health2 = manager.get_health().await;
        let timestamp2 = health2.timestamp;
        
        assert_eq!(timestamp1, timestamp2); // Same timestamp means cached
    }

    #[tokio::test]
    async fn test_startup_grace_period() {
        let manager = HealthManager::new();
        assert!(manager.is_in_startup_period());
        
        // After marking ready, still in grace period
        manager.mark_ready().await;
        assert!(manager.is_in_startup_period());
    }

    #[tokio::test]
    async fn test_status_priority() {
        let manager = HealthManager::new();
        
        assert!(manager.status_priority(&HealthStatus::Unhealthy) < manager.status_priority(&HealthStatus::Starting));
        assert!(manager.status_priority(&HealthStatus::Starting) < manager.status_priority(&HealthStatus::Degraded));
        assert!(manager.status_priority(&HealthStatus::Degraded) < manager.status_priority(&HealthStatus::Healthy));
    }
}
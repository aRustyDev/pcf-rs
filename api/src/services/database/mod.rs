use async_trait::async_trait;
use lazy_static::lazy_static;
use serde_json::Value;
use semver::{Version, VersionReq};
use std::collections::HashMap;
use thiserror::Error;

pub mod adapter;
pub mod connection;
pub mod health;
pub mod metrics;
pub mod models;
pub mod write_queue;

/// Database health status
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
    Starting,
}

/// Database version compatibility result
#[derive(Debug, Clone, PartialEq)]
pub enum VersionCompatibility {
    Compatible,
    Incompatible(String),
    Untested(String),
    Unknown,
}

/// Database version information
#[derive(Debug, Clone)]
pub struct DatabaseVersion {
    pub version: String,
    pub build: Option<String>,
    pub features: Vec<String>,
}

/// Query structure for database operations
#[derive(Debug, Clone)]
pub struct Query {
    pub filter: HashMap<String, Value>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort: Option<HashMap<String, SortOrder>>,
}

#[derive(Debug, Clone)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Database errors
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query execution failed: {0}")]
    QueryFailed(String),
    
    #[error("Timeout occurred: {0}")]
    Timeout(String),
    
    #[error("Version incompatible: {0}")]
    VersionIncompatible(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Data validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Record not found: {0}")]
    NotFound(String),
    
    #[error("Internal database error: {0}")]
    Internal(String),
    
    #[error("Database service unavailable - retry after {retry_after} seconds")]
    ServiceUnavailable { retry_after: u64 },
}

lazy_static! {
    static ref SUPPORTED_VERSIONS: VersionReq = VersionReq::parse(">=1.0.0, <3.0.0")
        .expect("Valid version requirement - compile time constant");
    static ref TESTED_VERSIONS: Vec<Version> = vec![
        Version::parse("1.0.0").expect("Valid version - compile time constant"),
        Version::parse("1.1.0").expect("Valid version - compile time constant"),
        Version::parse("1.2.0").expect("Valid version - compile time constant"),
        Version::parse("2.0.0").expect("Valid version - compile time constant"),
    ];
}

/// Version compatibility checker
pub struct VersionChecker;

impl VersionChecker {
    pub fn new() -> Self {
        Self
    }
    
    pub fn check_version(&self, version: &str) -> VersionCompatibility {
        let ver = match Version::parse(version) {
            Ok(v) => v,
            Err(_) => return VersionCompatibility::Unknown,
        };
        
        if !SUPPORTED_VERSIONS.matches(&ver) {
            return VersionCompatibility::Incompatible(
                format!("Version {} is not supported. Supported range: {}", version, *SUPPORTED_VERSIONS)
            );
        }
        
        if TESTED_VERSIONS.contains(&ver) {
            VersionCompatibility::Compatible
        } else {
            VersionCompatibility::Untested(
                format!("Version {} is in supported range but not tested", version)
            )
        }
    }
    
    pub fn is_compatible(&self, version: &str) -> Result<(), DatabaseError> {
        match self.check_version(version) {
            VersionCompatibility::Compatible => Ok(()),
            VersionCompatibility::Untested(_) => Ok(()), // Allow untested but supported versions
            VersionCompatibility::Incompatible(msg) => Err(DatabaseError::VersionIncompatible(msg)),
            VersionCompatibility::Unknown => Err(DatabaseError::VersionIncompatible(
                format!("Unable to parse version: {}", version)
            )),
        }
    }
}

impl Default for VersionChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Database service trait that all implementations must follow
/// 
/// # Example
/// ```no_run
/// # use async_trait::async_trait;
/// # use pcf_api::services::database::{DatabaseService, MockDatabase};
/// # use serde_json::json;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = MockDatabase::new();
/// db.connect().await?;
/// 
/// // Create a record
/// let id = db.create("notes", json!({"title": "Test"})).await?;
/// 
/// // Read it back
/// let record = db.read("notes", &id).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait DatabaseService: Send + Sync {
    /// Connect to the database with retry logic
    async fn connect(&self) -> Result<(), DatabaseError>;
    
    /// Check database health
    async fn health_check(&self) -> DatabaseHealth;
    
    /// Get database version information
    async fn version(&self) -> Result<DatabaseVersion, DatabaseError>;
    
    /// Create a record
    async fn create(&self, collection: &str, data: Value) -> Result<String, DatabaseError>;
    
    /// Read a record by ID
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError>;
    
    /// Update a record
    async fn update(&self, collection: &str, id: &str, data: Value) -> Result<(), DatabaseError>;
    
    /// Delete a record
    async fn delete(&self, collection: &str, id: &str) -> Result<(), DatabaseError>;
    
    /// Query records with timeout
    async fn query(&self, collection: &str, query: Query) -> Result<Vec<Value>, DatabaseError>;
}

/// Mock database implementation for testing
pub struct MockDatabase {
    health: DatabaseHealth,
    version: DatabaseVersion,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            health: DatabaseHealth::Starting,
            version: DatabaseVersion {
                version: "1.0.0".to_string(),
                build: Some("test-build".to_string()),
                features: vec!["test".to_string()],
            },
        }
    }
    
    pub fn with_health(mut self, health: DatabaseHealth) -> Self {
        self.health = health;
        self
    }
    
    pub fn with_version(mut self, version: String) -> Self {
        self.version.version = version;
        self
    }
}

impl Default for MockDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseService for MockDatabase {
    async fn connect(&self) -> Result<(), DatabaseError> {
        Ok(())
    }
    
    async fn health_check(&self) -> DatabaseHealth {
        self.health.clone()
    }
    
    async fn version(&self) -> Result<DatabaseVersion, DatabaseError> {
        Ok(self.version.clone())
    }
    
    async fn create(&self, _collection: &str, _data: Value) -> Result<String, DatabaseError> {
        Ok("test-id".to_string())
    }
    
    async fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, DatabaseError> {
        if collection == "notes" && id.starts_with("notes:") {
            // Return a mock note for testing
            let note_data = serde_json::json!({
                "id": id,
                "title": "Test Note",
                "content": "Test content",
                "author": "test_user",
                "created_at": "2023-01-01T00:00:00Z",
                "updated_at": "2023-01-01T00:00:00Z",
                "tags": ["test"]
            });
            Ok(Some(note_data))
        } else {
            Ok(None)
        }
    }
    
    async fn update(&self, _collection: &str, _id: &str, _data: Value) -> Result<(), DatabaseError> {
        Ok(())
    }
    
    async fn delete(&self, _collection: &str, _id: &str) -> Result<(), DatabaseError> {
        Ok(())
    }
    
    async fn query(&self, _collection: &str, _query: Query) -> Result<Vec<Value>, DatabaseError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_database_trait_connect() {
        let db = MockDatabase::new();
        let result = db.connect().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_version_compatibility() {
        let checker = VersionChecker::new();
        assert!(checker.is_compatible("1.0.0").is_ok());
        assert!(checker.is_compatible("0.1.0").is_err());
    }
    
    #[tokio::test]
    async fn test_database_health_check() {
        let db = MockDatabase::new().with_health(DatabaseHealth::Healthy);
        let health = db.health_check().await;
        assert_eq!(health, DatabaseHealth::Healthy);
    }
    
    #[tokio::test]
    async fn test_version_checker_compatible_versions() {
        let checker = VersionChecker::new();
        
        // Test compatible versions
        assert_eq!(checker.check_version("1.0.0"), VersionCompatibility::Compatible);
        assert_eq!(checker.check_version("1.1.0"), VersionCompatibility::Compatible);
        assert_eq!(checker.check_version("1.2.0"), VersionCompatibility::Compatible);
    }
    
    #[tokio::test]
    async fn test_version_checker_incompatible_versions() {
        let checker = VersionChecker::new();
        
        // Test incompatible versions
        assert!(matches!(checker.check_version("0.9.0"), VersionCompatibility::Incompatible(_)));
        assert!(matches!(checker.check_version("3.0.0"), VersionCompatibility::Incompatible(_)));
    }
    
    #[tokio::test]
    async fn test_version_checker_untested_versions() {
        let checker = VersionChecker::new();
        
        // Test untested but supported versions
        assert!(matches!(checker.check_version("1.3.0"), VersionCompatibility::Untested(_)));
    }
    
    #[tokio::test]
    async fn test_version_checker_unknown_versions() {
        let checker = VersionChecker::new();
        
        // Test invalid version strings
        assert_eq!(checker.check_version("invalid"), VersionCompatibility::Unknown);
        assert_eq!(checker.check_version(""), VersionCompatibility::Unknown);
    }
    
    #[tokio::test]
    async fn test_database_crud_operations() {
        let db = MockDatabase::new();
        
        // Test create
        let id = db.create("test", Value::Object(serde_json::Map::new())).await;
        assert!(id.is_ok());
        
        // Test read
        let result = db.read("test", "test-id").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
        
        // Test update
        let result = db.update("test", "test-id", Value::Object(serde_json::Map::new())).await;
        assert!(result.is_ok());
        
        // Test delete
        let result = db.delete("test", "test-id").await;
        assert!(result.is_ok());
        
        // Test query
        let query = Query {
            filter: HashMap::new(),
            limit: None,
            offset: None,
            sort: None,
        };
        let result = db.query("test", query).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_mock_database_with_custom_health() {
        let db = MockDatabase::new().with_health(DatabaseHealth::Degraded("Test degradation".to_string()));
        let health = db.health_check().await;
        assert_eq!(health, DatabaseHealth::Degraded("Test degradation".to_string()));
    }
    
    #[tokio::test]
    async fn test_mock_database_with_custom_version() {
        let db = MockDatabase::new().with_version("1.5.0".to_string());
        let version = db.version().await;
        assert!(version.is_ok());
        assert_eq!(version.unwrap().version, "1.5.0");
    }
    
    #[tokio::test]
    async fn test_database_error_types() {
        let error = DatabaseError::ConnectionFailed("test".to_string());
        assert!(error.to_string().contains("Connection failed"));
        
        let error = DatabaseError::QueryFailed("test".to_string());
        assert!(error.to_string().contains("Query execution failed"));
        
        let error = DatabaseError::Timeout("test".to_string());
        assert!(error.to_string().contains("Timeout occurred"));
        
        let error = DatabaseError::VersionIncompatible("test".to_string());
        assert!(error.to_string().contains("Version incompatible"));
    }
}
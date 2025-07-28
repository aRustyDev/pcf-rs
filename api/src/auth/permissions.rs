//! Permission type definitions for the authorization system
//! 
//! This module defines the core types used throughout the authorization framework
//! to represent permissions, resources, and authorization decisions. These types
//! provide a type-safe way to work with permissions and ensure consistency across
//! the system.
//! 
//! The permission system is designed to be:
//! - Type-safe: Compile-time guarantees about permission validity
//! - Extensible: Easy to add new resource types and actions
//! - SpiceDB-compatible: Maps cleanly to SpiceDB's permission model
//! - Auditable: Rich context for authorization decisions

use std::fmt;
use serde::{Deserialize, Serialize};

/// Represents an action that can be performed on a resource
/// 
/// Actions define what operations a user might want to perform.
/// The authorization system checks if a user has permission to 
/// perform a specific action on a specific resource.
/// 
/// # Examples
/// 
/// ```rust
/// use pcf_api::auth::permissions::Action;
/// 
/// let read_action = Action::Read;
/// let write_action = Action::Write;
/// 
/// assert_eq!(read_action.to_string(), "read");
/// assert_eq!(write_action.to_string(), "write");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Read access - view or retrieve data
    Read,
    /// Write access - create or modify data  
    Write,
    /// Delete access - remove data
    Delete,
    /// Administrative access - manage permissions and system settings
    Admin,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Action::Read => "read",
            Action::Write => "write", 
            Action::Delete => "delete",
            Action::Admin => "admin",
        };
        write!(f, "{}", s)
    }
}

impl Action {
    /// Get all available actions
    pub fn all() -> Vec<Action> {
        vec![Action::Read, Action::Write, Action::Delete, Action::Admin]
    }
    
    /// Check if this action implies another action
    /// 
    /// Some actions provide broader permissions than others.
    /// For example, Admin implies all other actions.
    /// 
    /// # Arguments
    /// 
    /// * `other` - The action to check if this action implies
    /// 
    /// # Returns
    /// 
    /// `true` if this action provides the permissions of the other action
    pub fn implies(&self, other: &Action) -> bool {
        match self {
            Action::Admin => true, // Admin can do everything
            Action::Write => matches!(other, Action::Read), // Write implies Read
            Action::Delete => matches!(other, Action::Read), // Delete implies Read
            Action::Read => matches!(other, Action::Read), // Read only implies Read
        }
    }
}

/// Represents a type of resource in the system
/// 
/// Resources are the entities that users interact with and that 
/// need access control. Each resource type may support different
/// actions and have different permission semantics.
/// 
/// # Examples
/// 
/// ```rust
/// use pcf_api::auth::permissions::ResourceType;
/// 
/// let note_resource = ResourceType::Note;
/// let user_resource = ResourceType::User;
/// 
/// assert_eq!(note_resource.to_string(), "note");
/// assert_eq!(user_resource.to_string(), "user");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    /// Note resources - user-created content
    Note,
    /// User resources - user accounts and profiles
    User,
    /// System resources - administrative functions
    System,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ResourceType::Note => "note",
            ResourceType::User => "user",
            ResourceType::System => "system",
        };
        write!(f, "{}", s)
    }
}

impl ResourceType {
    /// Get all available resource types
    pub fn all() -> Vec<ResourceType> {
        vec![ResourceType::Note, ResourceType::User, ResourceType::System]
    }
    
    /// Get valid actions for this resource type
    /// 
    /// Different resource types may support different sets of actions.
    /// This method returns the actions that make sense for each resource type.
    pub fn valid_actions(&self) -> Vec<Action> {
        match self {
            ResourceType::Note => vec![Action::Read, Action::Write, Action::Delete],
            ResourceType::User => vec![Action::Read, Action::Write, Action::Admin],
            ResourceType::System => vec![Action::Read, Action::Admin],
        }
    }
}

/// Represents a permission check request
/// 
/// This struct encapsulates all the information needed to make
/// an authorization decision. It includes the user, the resource
/// they want to access, and the action they want to perform.
/// 
/// # Examples
/// 
/// ```rust
/// use pcf_api::auth::permissions::{PermissionCheck, Action, ResourceType};
/// 
/// let check = PermissionCheck::new(
///     "user123",
///     "note",
///     "notes:abc123",
///     Action::Read
/// );
/// 
/// assert_eq!(check.user_id, "user123");
/// assert_eq!(check.resource_id, "notes:abc123");
/// assert_eq!(check.action, Action::Read);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermissionCheck {
    /// ID of the user requesting access
    pub user_id: String,
    /// Type of resource being accessed
    pub resource_type: String,
    /// Specific ID of the resource instance
    pub resource_id: String,
    /// Action the user wants to perform
    pub action: Action,
}

impl PermissionCheck {
    /// Create a new permission check
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - ID of the user requesting access
    /// * `resource_type` - Type of resource (e.g., "note", "user")
    /// * `resource_id` - Specific resource instance ID
    /// * `action` - Action to be performed
    pub fn new(user_id: &str, resource_type: &str, resource_id: &str, action: Action) -> Self {
        Self {
            user_id: user_id.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            action,
        }
    }
    
    /// Create permission check from typed resource
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - ID of the user requesting access
    /// * `resource_type` - Resource type enum
    /// * `resource_id` - Specific resource instance ID
    /// * `action` - Action to be performed
    pub fn from_typed(user_id: &str, resource_type: ResourceType, resource_id: &str, action: Action) -> Self {
        Self {
            user_id: user_id.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            action,
        }
    }
    
    /// Generate cache key for this permission check
    /// 
    /// Creates a consistent cache key that can be used to store
    /// and retrieve authorization results for this specific permission.
    pub fn cache_key(&self) -> String {
        format!("{}:{}:{}:{}", self.user_id, self.resource_type, self.resource_id, self.action)
    }
    
    /// Generate SpiceDB relationship tuple
    /// 
    /// Creates the relationship tuple format used by SpiceDB for permission checks.
    /// Format: "resource_type:resource_id#action@user:user_id"
    pub fn to_spicedb_tuple(&self) -> String {
        format!("{}:{}#{}@user:{}", self.resource_type, self.resource_id, self.action, self.user_id)
    }
}

/// Represents the result of a permission check
/// 
/// Contains not just the allow/deny decision but also context
/// about how the decision was made, which is valuable for
/// auditing, debugging, and monitoring.
/// 
/// # Examples
/// 
/// ```rust
/// use pcf_api::auth::permissions::{PermissionResult, PermissionSource};
/// 
/// let result = PermissionResult {
///     allowed: true,
///     reason: "User owns the resource".to_string(),
///     source: PermissionSource::Cache,
///     cached: true,
/// };
/// 
/// assert!(result.allowed);
/// assert!(result.cached);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionResult {
    /// Whether the action is allowed
    pub allowed: bool,
    /// Human-readable explanation of the decision
    pub reason: String,
    /// Where the decision came from
    pub source: PermissionSource,
    /// Whether this result was served from cache
    pub cached: bool,
}

impl PermissionResult {
    /// Create a new allowed result
    /// 
    /// # Arguments
    /// 
    /// * `reason` - Explanation for why access was granted
    /// * `source` - Source of the authorization decision
    /// * `cached` - Whether this result came from cache
    pub fn allowed(reason: &str, source: PermissionSource, cached: bool) -> Self {
        Self {
            allowed: true,
            reason: reason.to_string(),
            source,
            cached,
        }
    }
    
    /// Create a new denied result
    /// 
    /// # Arguments
    /// 
    /// * `reason` - Explanation for why access was denied
    /// * `source` - Source of the authorization decision
    /// * `cached` - Whether this result came from cache
    pub fn denied(reason: &str, source: PermissionSource, cached: bool) -> Self {
        Self {
            allowed: false,
            reason: reason.to_string(),
            source,
            cached,
        }
    }
    
    /// Create a result for cache miss (default deny)
    pub fn cache_miss() -> Self {
        Self::denied("No cached permission found", PermissionSource::Cache, false)
    }
    
    /// Create a result for system errors (default deny)
    pub fn system_error(error: &str) -> Self {
        Self::denied(&format!("System error: {}", error), PermissionSource::Error, false)
    }
}

/// Indicates where a permission decision came from
/// 
/// This helps with debugging, monitoring, and understanding
/// the authorization system's behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionSource {
    /// Decision came from cache
    Cache,
    /// Decision came from SpiceDB
    SpiceDB,
    /// Decision came from local business logic
    Local,
    /// Decision came from demo/test mode
    Demo,
    /// Error occurred during authorization
    Error,
}

impl fmt::Display for PermissionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PermissionSource::Cache => "cache",
            PermissionSource::SpiceDB => "spicedb",
            PermissionSource::Local => "local",
            PermissionSource::Demo => "demo",
            PermissionSource::Error => "error",
        };
        write!(f, "{}", s)
    }
}

/// Role definitions for the system
/// 
/// Roles provide a way to group permissions and assign them to users.
/// They map to SpiceDB relations and provide a higher-level abstraction
/// over individual permissions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System administrator with full access
    Admin,
    /// Regular user with standard permissions
    User,
    /// Read-only access
    Viewer,
    /// Service account for automated access
    Service,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Role::Admin => "admin",
            Role::User => "user",
            Role::Viewer => "viewer",
            Role::Service => "service",
        };
        write!(f, "{}", s)
    }
}

impl Role {
    /// Get all available roles
    pub fn all() -> Vec<Role> {
        vec![Role::Admin, Role::User, Role::Viewer, Role::Service]
    }
    
    /// Get the actions this role can perform on a resource type
    /// 
    /// # Arguments
    /// 
    /// * `resource_type` - The type of resource
    /// 
    /// # Returns
    /// 
    /// Vector of actions this role is allowed to perform
    pub fn allowed_actions(&self, resource_type: &ResourceType) -> Vec<Action> {
        match (self, resource_type) {
            (Role::Admin, _) => vec![Action::Read, Action::Write, Action::Delete, Action::Admin],
            (Role::User, ResourceType::Note) => vec![Action::Read, Action::Write, Action::Delete],
            (Role::User, ResourceType::User) => vec![Action::Read, Action::Write],
            (Role::User, ResourceType::System) => vec![Action::Read],
            (Role::Viewer, _) => vec![Action::Read],
            (Role::Service, ResourceType::Note) => vec![Action::Read, Action::Write],
            (Role::Service, ResourceType::User) => vec![Action::Read],
            (Role::Service, ResourceType::System) => vec![Action::Read],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_action_display() {
        assert_eq!(Action::Read.to_string(), "read");
        assert_eq!(Action::Write.to_string(), "write");
        assert_eq!(Action::Delete.to_string(), "delete");
        assert_eq!(Action::Admin.to_string(), "admin");
    }
    
    #[test]
    fn test_action_all() {
        let actions = Action::all();
        assert_eq!(actions.len(), 4);
        assert!(actions.contains(&Action::Read));
        assert!(actions.contains(&Action::Write));
        assert!(actions.contains(&Action::Delete));
        assert!(actions.contains(&Action::Admin));
    }
    
    #[test]
    fn test_action_implies() {
        // Admin implies everything
        assert!(Action::Admin.implies(&Action::Read));
        assert!(Action::Admin.implies(&Action::Write));
        assert!(Action::Admin.implies(&Action::Delete));
        assert!(Action::Admin.implies(&Action::Admin));
        
        // Write implies Read
        assert!(Action::Write.implies(&Action::Read));
        assert!(!Action::Write.implies(&Action::Delete));
        assert!(!Action::Write.implies(&Action::Admin));
        
        // Delete implies Read
        assert!(Action::Delete.implies(&Action::Read));
        assert!(!Action::Delete.implies(&Action::Write));
        assert!(!Action::Delete.implies(&Action::Admin));
        
        // Read only implies Read
        assert!(Action::Read.implies(&Action::Read));
        assert!(!Action::Read.implies(&Action::Write));
        assert!(!Action::Read.implies(&Action::Delete));
        assert!(!Action::Read.implies(&Action::Admin));
    }
    
    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::Note.to_string(), "note");
        assert_eq!(ResourceType::User.to_string(), "user");
        assert_eq!(ResourceType::System.to_string(), "system");
    }
    
    #[test]
    fn test_resource_type_all() {
        let types = ResourceType::all();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&ResourceType::Note));
        assert!(types.contains(&ResourceType::User));
        assert!(types.contains(&ResourceType::System));
    }
    
    #[test]
    fn test_resource_type_valid_actions() {
        let note_actions = ResourceType::Note.valid_actions();
        assert!(note_actions.contains(&Action::Read));
        assert!(note_actions.contains(&Action::Write));
        assert!(note_actions.contains(&Action::Delete));
        assert!(!note_actions.contains(&Action::Admin));
        
        let user_actions = ResourceType::User.valid_actions();
        assert!(user_actions.contains(&Action::Read));
        assert!(user_actions.contains(&Action::Write));
        assert!(user_actions.contains(&Action::Admin));
        assert!(!user_actions.contains(&Action::Delete));
        
        let system_actions = ResourceType::System.valid_actions();
        assert!(system_actions.contains(&Action::Read));
        assert!(system_actions.contains(&Action::Admin));
        assert!(!system_actions.contains(&Action::Write));
        assert!(!system_actions.contains(&Action::Delete));
    }
    
    #[test]
    fn test_permission_check_new() {
        let check = PermissionCheck::new("user123", "note", "notes:abc", Action::Read);
        
        assert_eq!(check.user_id, "user123");
        assert_eq!(check.resource_type, "note");
        assert_eq!(check.resource_id, "notes:abc");
        assert_eq!(check.action, Action::Read);
    }
    
    #[test]
    fn test_permission_check_from_typed() {
        let check = PermissionCheck::from_typed("user123", ResourceType::Note, "notes:abc", Action::Write);
        
        assert_eq!(check.user_id, "user123");
        assert_eq!(check.resource_type, "note");
        assert_eq!(check.resource_id, "notes:abc");
        assert_eq!(check.action, Action::Write);
    }
    
    #[test]
    fn test_permission_check_cache_key() {
        let check = PermissionCheck::new("user123", "note", "notes:abc", Action::Read);
        let key = check.cache_key();
        
        assert_eq!(key, "user123:note:notes:abc:read");
    }
    
    #[test]
    fn test_permission_check_spicedb_tuple() {
        let check = PermissionCheck::new("user123", "note", "notes:abc", Action::Read);
        let tuple = check.to_spicedb_tuple();
        
        assert_eq!(tuple, "note:notes:abc#read@user:user123");
    }
    
    #[test]
    fn test_permission_result_allowed() {
        let result = PermissionResult::allowed("User owns resource", PermissionSource::SpiceDB, false);
        
        assert!(result.allowed);
        assert_eq!(result.reason, "User owns resource");
        assert_eq!(result.source, PermissionSource::SpiceDB);
        assert!(!result.cached);
    }
    
    #[test]
    fn test_permission_result_denied() {
        let result = PermissionResult::denied("Insufficient permissions", PermissionSource::Local, true);
        
        assert!(!result.allowed);
        assert_eq!(result.reason, "Insufficient permissions");
        assert_eq!(result.source, PermissionSource::Local);
        assert!(result.cached);
    }
    
    #[test]
    fn test_permission_result_cache_miss() {
        let result = PermissionResult::cache_miss();
        
        assert!(!result.allowed);
        assert_eq!(result.reason, "No cached permission found");
        assert_eq!(result.source, PermissionSource::Cache);
        assert!(!result.cached);
    }
    
    #[test]
    fn test_permission_result_system_error() {
        let result = PermissionResult::system_error("Database connection failed");
        
        assert!(!result.allowed);
        assert_eq!(result.reason, "System error: Database connection failed");
        assert_eq!(result.source, PermissionSource::Error);
        assert!(!result.cached);
    }
    
    #[test]
    fn test_permission_source_display() {
        assert_eq!(PermissionSource::Cache.to_string(), "cache");
        assert_eq!(PermissionSource::SpiceDB.to_string(), "spicedb");
        assert_eq!(PermissionSource::Local.to_string(), "local");
        assert_eq!(PermissionSource::Demo.to_string(), "demo");
        assert_eq!(PermissionSource::Error.to_string(), "error");
    }
    
    #[test]
    fn test_role_display() {
        assert_eq!(Role::Admin.to_string(), "admin");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Viewer.to_string(), "viewer");
        assert_eq!(Role::Service.to_string(), "service");
    }
    
    #[test]
    fn test_role_all() {
        let roles = Role::all();
        assert_eq!(roles.len(), 4);
        assert!(roles.contains(&Role::Admin));
        assert!(roles.contains(&Role::User));
        assert!(roles.contains(&Role::Viewer));
        assert!(roles.contains(&Role::Service));
    }
    
    #[test]
    fn test_role_allowed_actions() {
        // Admin can do everything
        let admin_note_actions = Role::Admin.allowed_actions(&ResourceType::Note);
        assert!(admin_note_actions.contains(&Action::Read));
        assert!(admin_note_actions.contains(&Action::Write));
        assert!(admin_note_actions.contains(&Action::Delete));
        assert!(admin_note_actions.contains(&Action::Admin));
        
        // User permissions vary by resource
        let user_note_actions = Role::User.allowed_actions(&ResourceType::Note);
        assert!(user_note_actions.contains(&Action::Read));
        assert!(user_note_actions.contains(&Action::Write));
        assert!(user_note_actions.contains(&Action::Delete));
        assert!(!user_note_actions.contains(&Action::Admin));
        
        let user_system_actions = Role::User.allowed_actions(&ResourceType::System);
        assert!(user_system_actions.contains(&Action::Read));
        assert!(!user_system_actions.contains(&Action::Write));
        assert!(!user_system_actions.contains(&Action::Delete));
        assert!(!user_system_actions.contains(&Action::Admin));
        
        // Viewer can only read
        let viewer_actions = Role::Viewer.allowed_actions(&ResourceType::Note);
        assert!(viewer_actions.contains(&Action::Read));
        assert!(!viewer_actions.contains(&Action::Write));
        assert!(!viewer_actions.contains(&Action::Delete));
        assert!(!viewer_actions.contains(&Action::Admin));
        
        // Service has limited write access
        let service_note_actions = Role::Service.allowed_actions(&ResourceType::Note);
        assert!(service_note_actions.contains(&Action::Read));
        assert!(service_note_actions.contains(&Action::Write));
        assert!(!service_note_actions.contains(&Action::Delete));
        assert!(!service_note_actions.contains(&Action::Admin));
    }
    
    #[test]
    fn test_serialization() {
        let check = PermissionCheck::new("user123", "note", "notes:abc", Action::Read);
        let json = serde_json::to_string(&check).unwrap();
        let deserialized: PermissionCheck = serde_json::from_str(&json).unwrap();
        
        assert_eq!(check, deserialized);
    }
    
    #[test]
    fn test_permission_result_serialization() {
        let result = PermissionResult::allowed("Test reason", PermissionSource::Cache, true);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PermissionResult = serde_json::from_str(&json).unwrap();
        
        assert_eq!(result, deserialized);
    }
    
    // Integration test showing typical permission workflow
    #[test]
    fn test_permission_workflow() {
        // 1. Create a permission check
        let check = PermissionCheck::from_typed("alice", ResourceType::Note, "notes:123", Action::Read);
        
        // 2. Generate cache key
        let cache_key = check.cache_key();
        assert_eq!(cache_key, "alice:note:notes:123:read");
        
        // 3. Check if action is valid for resource type
        let valid_actions = ResourceType::Note.valid_actions();
        assert!(valid_actions.contains(&check.action));
        
        // 4. Create permission result
        let result = PermissionResult::allowed("User can read their own notes", PermissionSource::Local, false);
        assert!(result.allowed);
        
        // 5. Generate SpiceDB tuple for external verification
        let tuple = check.to_spicedb_tuple();
        assert_eq!(tuple, "note:notes:123#read@user:alice");
    }
}
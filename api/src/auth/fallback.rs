//! Fallback authorization rules for when SpiceDB is unavailable
//!
//! This module provides conservative fallback authorization rules that are
//! applied when the primary authorization backend (SpiceDB) is unavailable
//! due to network issues, circuit breaker activation, or other failures.
//!
//! # Security Philosophy
//!
//! The fallback system follows a "fail-safe" approach:
//! - **Conservative by default**: When in doubt, deny access
//! - **Minimal permissions**: Only allow essential operations
//! - **No write operations**: All write/modify operations are denied
//! - **Resource ownership**: Users can only access resources they own
//! - **System health**: Health checks are always allowed
//!
//! # Fallback Rules
//!
//! The fallback authorization allows:
//! 1. System health checks (any user, any resource starting with "system:health")
//! 2. Users reading their own resources (format: "resource_type:user_id:resource_id")
//! 3. Public resources for read operations only
//!
//! All other operations are denied, including:
//! - All write, create, update, delete operations
//! - Cross-user resource access
//! - Administrative operations
//! - Unknown resource types
//!
//! # Usage
//!
//! ```rust
//! use crate::auth::fallback::FallbackAuthorizer;
//!
//! let fallback = FallbackAuthorizer::new();
//!
//! // Check if user can read their own resource
//! let allowed = fallback.is_authorized(
//!     "user:alice",
//!     "notes:alice:123",
//!     "read"
//! );
//! assert!(allowed);
//!
//! // Cross-user access denied
//! let denied = fallback.is_authorized(
//!     "user:alice",
//!     "notes:bob:456",
//!     "read"
//! );
//! assert!(!denied);
//!
//! // All writes denied
//! let write_denied = fallback.is_authorized(
//!     "user:alice",
//!     "notes:alice:123",
//!     "write"
//! );
//! assert!(!write_denied);
//! ```

use tracing::{debug, warn, info};

/// Fallback authorizer for when SpiceDB is unavailable
///
/// This struct provides conservative authorization rules that prioritize
/// security over functionality. It's designed to keep the system operational
/// with minimal permissions when the primary authorization backend fails.
#[derive(Debug, Clone)]
pub struct FallbackAuthorizer {
    /// Whether to log all authorization decisions for debugging
    debug_logging: bool,
}

/// Supported resource types in fallback mode
#[derive(Debug, Clone, PartialEq)]
enum ResourceType {
    /// User notes - allows owner read access only
    Notes,
    /// Public resources - allows read access for all users
    Public,
    /// System health - allows all operations for monitoring
    SystemHealth,
    /// Unknown resource type - denied by default
    Unknown(String),
}

/// Parsed resource information
#[derive(Debug, Clone)]
struct ParsedResource {
    /// Resource type
    resource_type: ResourceType,
    /// Resource namespace/owner (if applicable)
    namespace: Option<String>,
    /// Resource identifier
    #[allow(dead_code)]
    resource_id: String,
    /// Original resource string
    #[allow(dead_code)]
    original: String,
}

/// Parsed subject information
#[derive(Debug, Clone)]
struct ParsedSubject {
    /// Subject type (e.g., "user", "service")
    subject_type: String,
    /// Subject identifier
    subject_id: String,
    /// Original subject string
    #[allow(dead_code)]
    original: String,
}

/// Action categories for authorization decisions
#[derive(Debug, Clone, PartialEq)]
enum ActionCategory {
    /// Read operations (read, list, view)
    Read,
    /// Write operations (write, create, update, delete)
    Write,
    /// Administrative operations (admin, manage, configure)
    Admin,
    /// Health check operations
    Health,
    /// Unknown operation type
    Unknown(String),
}

impl FallbackAuthorizer {
    /// Create a new fallback authorizer
    ///
    /// # Arguments
    ///
    /// * `debug_logging` - Whether to enable debug logging for all decisions
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::auth::fallback::FallbackAuthorizer;
    ///
    /// let fallback = FallbackAuthorizer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            debug_logging: cfg!(debug_assertions), // Debug logging in debug builds
        }
    }

    /// Create a new fallback authorizer with debug logging enabled
    ///
    /// This is useful for development and troubleshooting scenarios.
    pub fn with_debug_logging() -> Self {
        Self {
            debug_logging: true,
        }
    }

    /// Check if a subject is authorized to perform an action on a resource
    ///
    /// This is the main authorization function that implements the conservative
    /// fallback rules. It should be used when SpiceDB is unavailable or when
    /// the circuit breaker is open.
    ///
    /// # Arguments
    ///
    /// * `subject` - Subject requesting permission (format: "type:id")
    /// * `resource` - Resource being accessed (format: "type:namespace:id" or "type:id")
    /// * `action` - Action being performed (e.g., "read", "write", "delete")
    ///
    /// # Returns
    ///
    /// * `true` - Permission granted under fallback rules
    /// * `false` - Permission denied
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::auth::fallback::FallbackAuthorizer;
    ///
    /// let fallback = FallbackAuthorizer::new();
    ///
    /// // User reading their own resource
    /// assert!(fallback.is_authorized("user:alice", "notes:alice:123", "read"));
    ///
    /// // User accessing another user's resource
    /// assert!(!fallback.is_authorized("user:alice", "notes:bob:456", "read"));
    ///
    /// // Write operations denied
    /// assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "write"));
    /// ```
    pub fn is_authorized(&self, subject: &str, resource: &str, action: &str) -> bool {
        let start_time = std::time::Instant::now();

        // Parse input parameters
        let parsed_subject = match self.parse_subject(subject) {
            Ok(subject) => subject,
            Err(reason) => {
                self.log_decision(
                    subject,
                    resource,
                    action,
                    false,
                    &format!("Invalid subject format: {}", reason),
                    start_time.elapsed(),
                );
                return false;
            }
        };

        let parsed_resource = match self.parse_resource(resource) {
            Ok(resource) => resource,
            Err(reason) => {
                self.log_decision(
                    subject,
                    resource,
                    action,
                    false,
                    &format!("Invalid resource format: {}", reason),
                    start_time.elapsed(),
                );
                return false;
            }
        };

        let action_category = self.categorize_action(action);

        // Apply authorization rules
        let allowed = self.apply_authorization_rules(&parsed_subject, &parsed_resource, &action_category);

        // Log the decision
        let reason = self.get_decision_reason(&parsed_subject, &parsed_resource, &action_category, allowed);
        self.log_decision(subject, resource, action, allowed, &reason, start_time.elapsed());

        allowed
    }

    /// Parse subject string into components
    fn parse_subject(&self, subject: &str) -> Result<ParsedSubject, String> {
        let parts: Vec<&str> = subject.split(':').collect();
        
        if parts.len() != 2 {
            return Err(format!(
                "Subject must be in format 'type:id', got {} parts",
                parts.len()
            ));
        }

        let subject_type = parts[0].to_string();
        let subject_id = parts[1].to_string();

        if subject_type.is_empty() || subject_id.is_empty() {
            return Err("Subject type and ID cannot be empty".to_string());
        }

        Ok(ParsedSubject {
            subject_type,
            subject_id,
            original: subject.to_string(),
        })
    }

    /// Parse resource string into components
    fn parse_resource(&self, resource: &str) -> Result<ParsedResource, String> {
        let parts: Vec<&str> = resource.split(':').collect();
        
        if parts.is_empty() {
            return Err("Resource cannot be empty".to_string());
        }

        let resource_type_str = parts[0];
        if resource_type_str.is_empty() {
            return Err("Resource type cannot be empty".to_string());
        }

        let resource_type = match resource_type_str {
            "notes" => ResourceType::Notes,
            "public" => ResourceType::Public,
            "system" if parts.len() > 1 && parts[1] == "health" => ResourceType::SystemHealth,
            _ => ResourceType::Unknown(resource_type_str.to_string()),
        };

        let (namespace, resource_id) = match parts.len() {
            1 => (None, "".to_string()),
            2 => (None, parts[1].to_string()),
            3 => (Some(parts[1].to_string()), parts[2].to_string()),
            _ => {
                // For resources with more than 3 parts, use the second part as namespace
                // and join the rest as resource_id
                (Some(parts[1].to_string()), parts[2..].join(":"))
            }
        };

        // For system health, resource_id can be empty
        if resource_type != ResourceType::SystemHealth && resource_id.is_empty() && parts.len() > 1 {
            return Err("Resource ID cannot be empty".to_string());
        }

        Ok(ParsedResource {
            resource_type,
            namespace,
            resource_id,
            original: resource.to_string(),
        })
    }

    /// Categorize action into broad categories
    fn categorize_action(&self, action: &str) -> ActionCategory {
        match action.to_lowercase().as_str() {
            "read" | "get" | "list" | "view" | "show" => ActionCategory::Read,
            "write" | "create" | "update" | "delete" | "modify" | "edit" | "post" | "put" | "patch" => ActionCategory::Write,
            "admin" | "manage" | "configure" | "administrate" => ActionCategory::Admin,
            "health" | "check" | "ping" | "status" => ActionCategory::Health,
            _ => ActionCategory::Unknown(action.to_string()),
        }
    }

    /// Apply the core authorization rules
    fn apply_authorization_rules(
        &self,
        subject: &ParsedSubject,
        resource: &ParsedResource,
        action: &ActionCategory,
    ) -> bool {
        // Rule 1: System health checks are always allowed
        if matches!(resource.resource_type, ResourceType::SystemHealth) {
            return true;
        }

        // Rule 2: All write and admin operations are denied in fallback mode
        if matches!(action, ActionCategory::Write | ActionCategory::Admin) {
            return false;
        }

        // Rule 3: Unknown actions are denied for security
        if matches!(action, ActionCategory::Unknown(_)) {
            return false;
        }

        // Rule 4: Only handle read operations from here on
        match (&resource.resource_type, action) {
            (ResourceType::Notes, ActionCategory::Read) => {
                // Users can only read their own notes
                // Expected format: "notes:user_id:note_id"
                if let Some(ref namespace) = resource.namespace {
                    namespace == &subject.subject_id && subject.subject_type == "user"
                } else {
                    false // Notes without namespace are denied
                }
            }
            (ResourceType::Public, ActionCategory::Read) => {
                // Public resources are readable by all authenticated users
                subject.subject_type == "user"
            }
            (ResourceType::Unknown(_), _) => {
                // Unknown resource types are denied for security
                false
            }
            _ => {
                // All other combinations are denied
                false
            }
        }
    }

    /// Generate human-readable reason for the authorization decision
    fn get_decision_reason(
        &self,
        subject: &ParsedSubject,
        resource: &ParsedResource,
        action: &ActionCategory,
        allowed: bool,
    ) -> String {
        if !allowed {
            match (&resource.resource_type, action) {
                (ResourceType::SystemHealth, _) => "System health checks allowed".to_string(),
                (_, ActionCategory::Write) => "Write operations denied in fallback mode".to_string(),
                (_, ActionCategory::Admin) => "Admin operations denied in fallback mode".to_string(),
                (_, ActionCategory::Unknown(action_name)) => {
                    format!("Unknown action '{}' denied", action_name)
                }
                (ResourceType::Notes, ActionCategory::Read) => {
                    if resource.namespace.as_ref() != Some(&subject.subject_id) {
                        "Cross-user access denied".to_string()
                    } else if subject.subject_type != "user" {
                        "Non-user subjects denied".to_string()
                    } else {
                        "Notes without proper namespace denied".to_string()
                    }
                }
                (ResourceType::Public, ActionCategory::Read) => {
                    if subject.subject_type != "user" {
                        "Non-user subjects denied for public resources".to_string()
                    } else {
                        "Public resource access denied for unknown reason".to_string()
                    }
                }
                (ResourceType::Unknown(resource_type), _) => {
                    format!("Unknown resource type '{}' denied", resource_type)
                }
                _ => "Default deny rule applied".to_string(),
            }
        } else {
            match (&resource.resource_type, action) {
                (ResourceType::SystemHealth, _) => "System health check allowed".to_string(),
                (ResourceType::Notes, ActionCategory::Read) => "Owner reading own resource".to_string(),
                (ResourceType::Public, ActionCategory::Read) => "Public resource read access".to_string(),
                _ => "Fallback rule allowed access".to_string(),
            }
        }
    }

    /// Log authorization decision with structured information
    fn log_decision(
        &self,
        subject: &str,
        resource: &str,
        action: &str,
        allowed: bool,
        reason: &str,
        duration: std::time::Duration,
    ) {
        if self.debug_logging {
            debug!(
                subject = %subject,
                resource = %resource,
                action = %action,
                allowed = %allowed,
                reason = %reason,
                duration_us = %duration.as_micros(),
                source = "fallback",
                "Fallback authorization decision"
            );
        }

        // Always log denials for security monitoring
        if !allowed {
            info!(
                subject = %subject,
                resource = %resource,
                action = %action,
                reason = %reason,
                source = "fallback",
                "Fallback authorization denied"
            );
        }

        // Log write attempts as warnings (they should all be denied)
        let action_category = self.categorize_action(action);
        if matches!(action_category, ActionCategory::Write | ActionCategory::Admin) {
            warn!(
                subject = %subject,
                resource = %resource,
                action = %action,
                allowed = %allowed,
                source = "fallback",
                "Write/admin operation attempted in fallback mode"
            );
        }
    }

    /// Get statistics about fallback authorizer usage
    ///
    /// Note: This is a simple implementation that doesn't maintain state.
    /// In a production system, you might want to add metrics collection.
    pub fn stats(&self) -> FallbackStats {
        FallbackStats {
            total_decisions: 0, // Would need state tracking to implement
            allowed_decisions: 0,
            denied_decisions: 0,
            read_operations: 0,
            write_operations: 0,
            admin_operations: 0,
            health_operations: 0,
            unknown_operations: 0,
            cross_user_attempts: 0,
            unknown_resource_attempts: 0,
        }
    }

    /// Check if the fallback authorizer is operating in debug mode
    pub fn is_debug_mode(&self) -> bool {
        self.debug_logging
    }
}

impl Default for FallbackAuthorizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for fallback authorizer monitoring
#[derive(Debug, Clone)]
pub struct FallbackStats {
    /// Total authorization decisions made
    pub total_decisions: u64,
    /// Number of allowed decisions
    pub allowed_decisions: u64,
    /// Number of denied decisions
    pub denied_decisions: u64,
    /// Number of read operations
    pub read_operations: u64,
    /// Number of write operations (should all be denied)
    pub write_operations: u64,
    /// Number of admin operations (should all be denied)
    pub admin_operations: u64,
    /// Number of health check operations
    pub health_operations: u64,
    /// Number of unknown operations
    pub unknown_operations: u64,
    /// Number of cross-user access attempts
    pub cross_user_attempts: u64,
    /// Number of unknown resource type attempts
    pub unknown_resource_attempts: u64,
}

impl Default for FallbackStats {
    fn default() -> Self {
        Self {
            total_decisions: 0,
            allowed_decisions: 0,
            denied_decisions: 0,
            read_operations: 0,
            write_operations: 0,
            admin_operations: 0,
            health_operations: 0,
            unknown_operations: 0,
            cross_user_attempts: 0,
            unknown_resource_attempts: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_authorizer_creation() {
        let fallback = FallbackAuthorizer::new();
        assert!(!fallback.is_debug_mode() || cfg!(debug_assertions));
        
        let fallback_debug = FallbackAuthorizer::with_debug_logging();
        assert!(fallback_debug.is_debug_mode());
    }

    #[test]
    fn test_parse_subject_valid() {
        let fallback = FallbackAuthorizer::new();
        
        let subject = fallback.parse_subject("user:alice").unwrap();
        assert_eq!(subject.subject_type, "user");
        assert_eq!(subject.subject_id, "alice");
        assert_eq!(subject.original, "user:alice");
    }

    #[test]
    fn test_parse_subject_invalid() {
        let fallback = FallbackAuthorizer::new();
        
        assert!(fallback.parse_subject("invalid").is_err());
        assert!(fallback.parse_subject("user:").is_err());
        assert!(fallback.parse_subject(":alice").is_err());
        assert!(fallback.parse_subject("user:alice:extra").is_err());
    }

    #[test]
    fn test_parse_resource_notes() {
        let fallback = FallbackAuthorizer::new();
        
        let resource = fallback.parse_resource("notes:alice:123").unwrap();
        assert!(matches!(resource.resource_type, ResourceType::Notes));
        assert_eq!(resource.namespace, Some("alice".to_string()));
        assert_eq!(resource.resource_id, "123");
    }

    #[test]
    fn test_parse_resource_public() {
        let fallback = FallbackAuthorizer::new();
        
        let resource = fallback.parse_resource("public:docs").unwrap();
        assert!(matches!(resource.resource_type, ResourceType::Public));
        assert_eq!(resource.namespace, None);
        assert_eq!(resource.resource_id, "docs");
    }

    #[test]
    fn test_parse_resource_system_health() {
        let fallback = FallbackAuthorizer::new();
        
        let resource = fallback.parse_resource("system:health:check").unwrap();
        assert!(matches!(resource.resource_type, ResourceType::SystemHealth));
        assert_eq!(resource.resource_id, "check");
    }

    #[test]
    fn test_categorize_action() {
        let fallback = FallbackAuthorizer::new();
        
        assert!(matches!(fallback.categorize_action("read"), ActionCategory::Read));
        assert!(matches!(fallback.categorize_action("list"), ActionCategory::Read));
        assert!(matches!(fallback.categorize_action("write"), ActionCategory::Write));
        assert!(matches!(fallback.categorize_action("delete"), ActionCategory::Write));
        assert!(matches!(fallback.categorize_action("admin"), ActionCategory::Admin));
        assert!(matches!(fallback.categorize_action("health"), ActionCategory::Health));
        assert!(matches!(fallback.categorize_action("unknown"), ActionCategory::Unknown(_)));
    }

    #[test]
    fn test_system_health_always_allowed() {
        let fallback = FallbackAuthorizer::new();
        
        assert!(fallback.is_authorized("user:alice", "system:health:check", "read"));
        assert!(fallback.is_authorized("user:bob", "system:health:status", "admin"));
        assert!(fallback.is_authorized("service:monitor", "system:health", "ping"));
    }

    #[test]
    fn test_owner_read_allowed() {
        let fallback = FallbackAuthorizer::new();
        
        // Users can read their own resources
        assert!(fallback.is_authorized("user:alice", "notes:alice:123", "read"));
        assert!(fallback.is_authorized("user:bob", "notes:bob:456", "list"));
    }

    #[test]
    fn test_cross_user_read_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Users cannot read other users' resources
        assert!(!fallback.is_authorized("user:alice", "notes:bob:123", "read"));
        assert!(!fallback.is_authorized("user:bob", "notes:alice:456", "list"));
    }

    #[test]
    fn test_all_writes_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // All write operations denied, even for own resources
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "write"));
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "delete"));
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "update"));
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "create"));
    }

    #[test]
    fn test_public_read_allowed() {
        let fallback = FallbackAuthorizer::new();
        
        // Public resources readable by authenticated users
        assert!(fallback.is_authorized("user:alice", "public:docs", "read"));
        assert!(fallback.is_authorized("user:bob", "public:announcements", "list"));
    }

    #[test]
    fn test_public_write_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Even public resources deny writes
        assert!(!fallback.is_authorized("user:alice", "public:docs", "write"));
        assert!(!fallback.is_authorized("user:admin", "public:announcements", "create"));
    }

    #[test]
    fn test_unknown_resources_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Unknown resource types denied
        assert!(!fallback.is_authorized("user:alice", "secrets:key", "read"));
        assert!(!fallback.is_authorized("user:alice", "admin:panel", "read"));
        assert!(!fallback.is_authorized("user:alice", "unknown:resource", "read"));
    }

    #[test]
    fn test_admin_operations_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Admin operations always denied
        assert!(!fallback.is_authorized("user:admin", "notes:admin:123", "admin"));
        assert!(!fallback.is_authorized("user:root", "system:config", "manage"));
    }

    #[test]
    fn test_unknown_actions_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Unknown actions denied for security
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "unknown"));
        assert!(!fallback.is_authorized("user:alice", "notes:alice:123", "custom"));
    }

    #[test]
    fn test_non_user_subjects_denied() {
        let fallback = FallbackAuthorizer::new();
        
        // Non-user subjects denied for most resources (except health)
        assert!(!fallback.is_authorized("service:api", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("bot:crawler", "public:docs", "read"));
        
        // But health checks still work
        assert!(fallback.is_authorized("service:monitor", "system:health", "check"));
    }

    #[test]
    fn test_malformed_inputs() {
        let fallback = FallbackAuthorizer::new();
        
        // Malformed subjects
        assert!(!fallback.is_authorized("alice", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("user", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("", "notes:alice:123", "read"));
        
        // Malformed resources
        assert!(!fallback.is_authorized("user:alice", "", "read"));
        assert!(!fallback.is_authorized("user:alice", "notes", "read"));
        assert!(!fallback.is_authorized("user:alice", ":alice:123", "read"));
    }

    #[test]
    fn test_case_sensitivity() {
        let fallback = FallbackAuthorizer::new();
        
        // Actions should be case-insensitive
        assert!(fallback.is_authorized("user:alice", "notes:alice:123", "READ"));
        assert!(fallback.is_authorized("user:alice", "notes:alice:123", "Read"));
        assert!(fallback.is_authorized("user:alice", "notes:alice:123", "LIST"));
        
        // But resource types and subjects are case-sensitive
        assert!(!fallback.is_authorized("User:alice", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("user:Alice", "notes:alice:123", "read"));
        assert!(!fallback.is_authorized("user:alice", "Notes:alice:123", "read"));
    }

    #[test]
    fn test_stats() {
        let fallback = FallbackAuthorizer::new();
        let stats = fallback.stats();
        
        // Stats start at zero (no state tracking in simple implementation)
        assert_eq!(stats.total_decisions, 0);
        assert_eq!(stats.allowed_decisions, 0);
        assert_eq!(stats.denied_decisions, 0);
    }
}
//! Demo mode configuration
//!
//! This module provides configuration and utilities for demo mode,
//! which allows bypassing certain security restrictions for
//! demonstration and testing purposes.
//!
//! # Security Warning
//!
//! Demo mode should NEVER be enabled in production environments.
//! It bypasses critical security checks and should only be used
//! for development, testing, and demonstration purposes.

use serde::{Deserialize, Serialize};
use std::env;
use tracing::{warn, info};

/// Demo mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoConfig {
    /// Whether demo mode is enabled
    pub enabled: bool,
    /// Demo user configuration
    pub demo_user: DemoUser,
    /// Authorization bypass settings
    pub authorization: DemoAuthConfig,
    /// Rate limiting bypass
    pub bypass_rate_limits: bool,
    /// Database settings for demo mode
    pub database: DemoDatabaseConfig,
}

/// Demo user configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoUser {
    /// Demo user ID
    pub user_id: String,
    /// Whether demo user has admin privileges
    pub is_admin: bool,
    /// Display name for the demo user
    pub display_name: String,
    /// Email address for the demo user
    pub email: String,
}

/// Demo authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoAuthConfig {
    /// Whether to bypass SpiceDB authorization checks
    pub bypass_spicedb: bool,
    /// Whether to allow all operations for demo user
    pub allow_all_operations: bool,
    /// Whether to bypass ownership checks
    pub bypass_ownership_checks: bool,
    /// Whether to enable auto-authentication (no tokens required)
    pub auto_authenticate: bool,
}

/// Demo database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoDatabaseConfig {
    /// Whether to use in-memory database for demo
    pub use_in_memory: bool,
    /// Whether to pre-populate with sample data
    pub populate_sample_data: bool,
    /// Number of sample notes to create
    pub sample_notes_count: usize,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            demo_user: DemoUser::default(),
            authorization: DemoAuthConfig::default(),
            bypass_rate_limits: false,
            database: DemoDatabaseConfig::default(),
        }
    }
}

impl Default for DemoUser {
    fn default() -> Self {
        Self {
            user_id: "demo_user".to_string(),
            is_admin: false,
            display_name: "Demo User".to_string(),
            email: "demo@example.com".to_string(),
        }
    }
}

impl Default for DemoAuthConfig {
    fn default() -> Self {
        Self {
            bypass_spicedb: true,
            allow_all_operations: true,
            bypass_ownership_checks: true,
            auto_authenticate: true,
        }
    }
}

impl Default for DemoDatabaseConfig {
    fn default() -> Self {
        Self {
            use_in_memory: true,
            populate_sample_data: true,
            sample_notes_count: 10,
        }
    }
}

impl DemoConfig {
    /// Load demo configuration from environment variables
    pub fn from_env() -> Self {
        let enabled = env::var("DEMO_MODE_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        if enabled {
            warn!("🚨 DEMO MODE ENABLED 🚨");
            warn!("This should NEVER be used in production!");
            warn!("Security checks will be bypassed!");
        }

        Self {
            enabled,
            demo_user: DemoUser::from_env(),
            authorization: DemoAuthConfig::from_env(),
            bypass_rate_limits: env::var("DEMO_BYPASS_RATE_LIMITS")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
            database: DemoDatabaseConfig::from_env(),
        }
    }

    /// Check if demo mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the demo user session if demo mode is enabled
    pub fn get_demo_session(&self) -> Option<crate::graphql::context::Session> {
        if self.enabled {
            Some(crate::graphql::context::Session {
                user_id: self.demo_user.user_id.clone(),
                is_admin: self.demo_user.is_admin,
            })
        } else {
            None
        }
    }

    /// Check if authorization should be bypassed for demo mode
    pub fn should_bypass_authorization(&self) -> bool {
        self.enabled && self.authorization.bypass_spicedb
    }

    /// Check if all operations should be allowed for demo user
    pub fn should_allow_all_operations(&self) -> bool {
        self.enabled && self.authorization.allow_all_operations
    }

    /// Check if ownership checks should be bypassed
    pub fn should_bypass_ownership_checks(&self) -> bool {
        self.enabled && self.authorization.bypass_ownership_checks
    }

    /// Check if auto-authentication is enabled
    pub fn should_auto_authenticate(&self) -> bool {
        self.enabled && self.authorization.auto_authenticate
    }

    /// Log demo mode status
    pub fn log_status(&self) {
        if self.enabled {
            warn!("🚨 Demo Mode Active 🚨");
            warn!("  User: {}", self.demo_user.user_id);
            warn!("  Admin: {}", self.demo_user.is_admin);
            warn!("  Bypass Auth: {}", self.authorization.bypass_spicedb);
            warn!("  Allow All Ops: {}", self.authorization.allow_all_operations);
            warn!("  Auto Auth: {}", self.authorization.auto_authenticate);
        } else {
            info!("Demo mode is disabled (production mode)");
        }
    }
}

impl DemoUser {
    fn from_env() -> Self {
        Self {
            user_id: env::var("DEMO_USER_ID").unwrap_or_else(|_| "demo_user".to_string()),
            is_admin: env::var("DEMO_USER_IS_ADMIN")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
            display_name: env::var("DEMO_USER_DISPLAY_NAME")
                .unwrap_or_else(|_| "Demo User".to_string()),
            email: env::var("DEMO_USER_EMAIL")
                .unwrap_or_else(|_| "demo@example.com".to_string()),
        }
    }
}

impl DemoAuthConfig {
    fn from_env() -> Self {
        Self {
            bypass_spicedb: env::var("DEMO_BYPASS_SPICEDB")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
            allow_all_operations: env::var("DEMO_ALLOW_ALL_OPERATIONS")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
            bypass_ownership_checks: env::var("DEMO_BYPASS_OWNERSHIP_CHECKS")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
            auto_authenticate: env::var("DEMO_AUTO_AUTHENTICATE")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
        }
    }
}

impl DemoDatabaseConfig {
    fn from_env() -> Self {
        Self {
            use_in_memory: env::var("DEMO_USE_IN_MEMORY_DB")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
            populate_sample_data: env::var("DEMO_POPULATE_SAMPLE_DATA")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
            sample_notes_count: env::var("DEMO_SAMPLE_NOTES_COUNT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}

/// Feature flag for demo mode functionality
pub struct DemoFeatures;

impl DemoFeatures {
    /// Check if a feature is enabled in demo mode
    pub fn is_feature_enabled(feature: &str, demo_config: &DemoConfig) -> bool {
        if !demo_config.is_enabled() {
            return false;
        }

        match feature {
            "bypass_auth" => demo_config.should_bypass_authorization(),
            "allow_all_ops" => demo_config.should_allow_all_operations(),
            "bypass_ownership" => demo_config.should_bypass_ownership_checks(),
            "auto_auth" => demo_config.should_auto_authenticate(),
            "bypass_rate_limits" => demo_config.bypass_rate_limits,
            "in_memory_db" => demo_config.database.use_in_memory,
            "sample_data" => demo_config.database.populate_sample_data,
            _ => false,
        }
    }

    /// Get list of enabled demo features
    pub fn get_enabled_features(demo_config: &DemoConfig) -> Vec<String> {
        if !demo_config.is_enabled() {
            return vec![];
        }

        let mut features = vec![];
        
        if demo_config.should_bypass_authorization() {
            features.push("bypass_auth".to_string());
        }
        if demo_config.should_allow_all_operations() {
            features.push("allow_all_ops".to_string());
        }
        if demo_config.should_bypass_ownership_checks() {
            features.push("bypass_ownership".to_string());
        }
        if demo_config.should_auto_authenticate() {
            features.push("auto_auth".to_string());
        }
        if demo_config.bypass_rate_limits {
            features.push("bypass_rate_limits".to_string());
        }
        if demo_config.database.use_in_memory {
            features.push("in_memory_db".to_string());
        }
        if demo_config.database.populate_sample_data {
            features.push("sample_data".to_string());
        }

        features
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_demo_config_default() {
        let config = DemoConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.demo_user.user_id, "demo_user");
        assert!(!config.demo_user.is_admin);
    }

    #[test]
    fn test_demo_config_from_env_disabled() {
        // Ensure demo mode is disabled by default
        unsafe { env::remove_var("DEMO_MODE_ENABLED"); }
        
        let config = DemoConfig::from_env();
        assert!(!config.enabled);
        assert!(!config.should_bypass_authorization());
    }

    #[test]
    fn test_demo_config_from_env_enabled() {
        // Set environment variables for enabled demo mode
        unsafe {
            env::set_var("DEMO_MODE_ENABLED", "true");
            env::set_var("DEMO_USER_ID", "test_demo_user");
            env::set_var("DEMO_USER_IS_ADMIN", "true");
        }
        
        let config = DemoConfig::from_env();
        assert!(config.enabled);
        assert_eq!(config.demo_user.user_id, "test_demo_user");
        assert!(config.demo_user.is_admin);
        assert!(config.should_bypass_authorization());
        
        // Cleanup
        unsafe {
            env::remove_var("DEMO_MODE_ENABLED");
            env::remove_var("DEMO_USER_ID");
            env::remove_var("DEMO_USER_IS_ADMIN");
        }
    }

    #[test]
    fn test_demo_session_creation() {
        let mut config = DemoConfig::default();
        
        // When disabled, should return None
        assert!(config.get_demo_session().is_none());
        
        // When enabled, should return demo session
        config.enabled = true;
        let session = config.get_demo_session().unwrap();
        assert_eq!(session.user_id, "demo_user");
        assert!(!session.is_admin);
    }

    #[test]
    fn test_demo_features() {
        let mut config = DemoConfig::default();
        
        // When disabled, no features should be enabled
        assert!(!DemoFeatures::is_feature_enabled("bypass_auth", &config));
        assert!(DemoFeatures::get_enabled_features(&config).is_empty());
        
        // When enabled, features should be available
        config.enabled = true;
        assert!(DemoFeatures::is_feature_enabled("bypass_auth", &config));
        assert!(DemoFeatures::is_feature_enabled("allow_all_ops", &config));
        
        let features = DemoFeatures::get_enabled_features(&config);
        assert!(features.contains(&"bypass_auth".to_string()));
        assert!(features.contains(&"allow_all_ops".to_string()));
    }
}
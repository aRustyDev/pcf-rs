use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::info;

use super::AuthContext;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub trace_id: String,
    pub user_id: String,
    pub resource: String,
    pub action: String,
    pub allowed: bool,
    pub source: String, // "cache", "spicedb", "fallback"
    pub duration_ms: u64,
}

pub async fn audit_authorization_decision(
    auth: &AuthContext,
    resource: &str,
    action: &str,
    allowed: bool,
    source: &str,
) {
    let entry = AuditEntry {
        timestamp: Utc::now(),
        trace_id: auth.trace_id.clone(),
        user_id: auth.user_id.clone().unwrap_or_default(),
        resource: resource.to_string(),
        action: action.to_string(),
        allowed,
        source: source.to_string(),
        duration_ms: 0, // Will be calculated with timing in actual implementation
    };
    
    // Log as structured JSON
    info!(
        target: "audit",
        audit_type = "authorization",
        trace_id = %entry.trace_id,
        user_id = %entry.user_id,
        resource = %entry.resource,
        action = %entry.action,
        allowed = %entry.allowed,
        source = %entry.source,
        "Authorization decision"
    );
    
    // Future: Send to audit service
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthContext;
    
    #[tokio::test]
    async fn test_audit_authorization_decision() {
        let auth = AuthContext {
            user_id: Some("user123".to_string()),
            trace_id: "trace456".to_string(),
            is_admin: false,
            session_token: None,
        };
        
        // This should not panic
        audit_authorization_decision(&auth, "notes:123", "read", true, "spicedb").await;
        
        // Test with no user_id
        let auth_no_user = AuthContext {
            user_id: None,
            trace_id: "trace789".to_string(),
            is_admin: false,
            session_token: None,
        };
        
        audit_authorization_decision(&auth_no_user, "notes:456", "write", false, "fallback").await;
    }
}
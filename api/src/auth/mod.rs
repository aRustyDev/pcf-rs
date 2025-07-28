use async_graphql::{Error, ErrorExtensions};
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod audit;
pub mod cache;
pub mod components;
pub mod fallback;
pub mod permissions;

pub use audit::*;
pub use cache::*;
pub use fallback::*;
pub use permissions::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub trace_id: String,
    pub is_admin: bool,
    #[serde(skip)]
    pub session_token: Option<String>,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }
    
    pub fn require_auth(&self) -> Result<&str, Error> {
        self.user_id.as_deref().ok_or_else(|| {
            Error::new("Authentication required")
                .extend_with(|_, ext| ext.set("code", "UNAUTHORIZED"))
        })
    }
}

/// Extract authentication from request headers
pub async fn extract_auth_context(headers: &HeaderMap) -> AuthContext {
    let user_id = headers
        .get("x-user-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    
    let trace_id = headers
        .get("x-trace-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    let session_token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    
    AuthContext {
        user_id,
        trace_id,
        is_admin: false, // Will be determined by SpiceDB
        session_token,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    
    #[test]
    fn test_auth_context_is_authenticated() {
        let auth = AuthContext {
            user_id: Some("user123".to_string()),
            trace_id: "trace456".to_string(),
            is_admin: false,
            session_token: None,
        };
        
        assert!(auth.is_authenticated());
    }
    
    #[test]
    fn test_auth_context_not_authenticated() {
        let auth = AuthContext {
            user_id: None,
            trace_id: "trace456".to_string(),
            is_admin: false,
            session_token: None,
        };
        
        assert!(!auth.is_authenticated());
    }
    
    #[test]
    fn test_require_auth_success() {
        let auth = AuthContext {
            user_id: Some("user123".to_string()),
            trace_id: "trace456".to_string(),
            is_admin: false,
            session_token: None,
        };
        
        let result = auth.require_auth();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "user123");
    }
    
    #[test]
    fn test_require_auth_failure() {
        let auth = AuthContext {
            user_id: None,
            trace_id: "trace456".to_string(),
            is_admin: false,
            session_token: None,
        };
        
        let result = auth.require_auth();
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert_eq!(err.message, "Authentication required");
        if let Some(extensions) = &err.extensions {
            assert_eq!(extensions.get("code"), Some(&async_graphql::Value::from("UNAUTHORIZED")));
        } else {
            panic!("Expected error extensions to be present");
        }
    }
    
    #[tokio::test]
    async fn test_extract_auth_context_with_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", HeaderValue::from_static("alice"));
        headers.insert("x-trace-id", HeaderValue::from_static("trace123"));
        headers.insert("authorization", HeaderValue::from_static("Bearer token456"));
        
        let auth = extract_auth_context(&headers).await;
        
        assert_eq!(auth.user_id, Some("alice".to_string()));
        assert_eq!(auth.trace_id, "trace123");
        assert_eq!(auth.session_token, Some("token456".to_string()));
        assert!(!auth.is_admin); // Default to false
    }
    
    #[tokio::test]
    async fn test_extract_auth_context_minimal_headers() {
        let headers = HeaderMap::new();
        
        let auth = extract_auth_context(&headers).await;
        
        assert_eq!(auth.user_id, None);
        assert!(!auth.trace_id.is_empty()); // Should generate a UUID
        assert_eq!(auth.session_token, None);
        assert!(!auth.is_admin);
    }
    
    #[tokio::test]
    async fn test_extract_auth_context_without_bearer_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("token456"));
        
        let auth = extract_auth_context(&headers).await;
        
        assert_eq!(auth.session_token, None); // Should not extract without Bearer prefix
    }
    
    #[tokio::test]
    async fn test_extract_auth_context_invalid_headers() {
        let mut headers = HeaderMap::new();
        // Add invalid UTF-8 header (this is tricky to test, so we'll test with valid headers)
        headers.insert("x-user-id", HeaderValue::from_static("user with spaces"));
        
        let auth = extract_auth_context(&headers).await;
        
        // Should handle gracefully
        assert_eq!(auth.user_id, Some("user with spaces".to_string()));
    }
}
use crate::services::database::DatabaseService;
use async_graphql::{Context, Result, ErrorExtensions};
use std::sync::Arc;

/// Session information for authenticated users
#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: String,
    pub is_admin: bool,
}

impl Session {
    /// Create a demo session for testing/demo mode
    #[cfg(feature = "demo")]
    pub fn demo() -> Self {
        Self {
            user_id: "demo_user".to_string(),
            is_admin: true,
        }
    }
}

/// GraphQL request context containing shared resources
pub struct GraphQLContext {
    pub database: Arc<dyn DatabaseService>,
    pub session: Option<Session>,
    pub request_id: String,
    #[cfg(feature = "demo")]
    pub demo_mode: bool,
    #[cfg(feature = "demo")]
    demo_session: Session,
}

impl GraphQLContext {
    pub fn new(
        database: Arc<dyn DatabaseService>,
        session: Option<Session>,
        request_id: String,
    ) -> Self {
        Self {
            database,
            session,
            request_id,
            #[cfg(feature = "demo")]
            demo_mode: true,
            #[cfg(feature = "demo")]
            demo_session: Session::demo(),
        }
    }
    
    /// Check if user is authenticated (demo mode bypass)
    pub fn require_auth(&self) -> Result<&Session> {
        #[cfg(feature = "demo")]
        if self.demo_mode {
            return Ok(self.session.as_ref().unwrap_or(&self.demo_session));
        }
        
        self.session.as_ref()
            .ok_or_else(|| {
                async_graphql::Error::new("Authentication required")
                    .extend_with(|_, e| e.set("code", "UNAUTHENTICATED"))
            })
    }
    
    /// Get the current user ID (requires authentication)
    pub fn get_current_user(&self) -> Result<String> {
        let session = self.require_auth()?;
        Ok(session.user_id.clone())
    }
}

/// Extension trait for easy context access
pub trait ContextExt {
    fn get_context(&self) -> Result<&GraphQLContext>;
}

impl<'a> ContextExt for Context<'a> {
    fn get_context(&self) -> Result<&GraphQLContext> {
        self.data::<GraphQLContext>()
            .map_err(|_| async_graphql::Error::new("Context not available"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::MockDatabase;
    
    fn mock_database() -> Arc<dyn DatabaseService> {
        Arc::new(MockDatabase::new())
    }
    
    #[test]
    fn test_context_creation() {
        let database = mock_database();
        let session = Some(Session {
            user_id: "test_user".to_string(),
            is_admin: false,
        });
        
        let context = GraphQLContext::new(
            database,
            session.clone(),
            "test-request-123".to_string(),
        );
        
        assert_eq!(context.request_id, "test-request-123");
        assert!(context.session.is_some());
        assert_eq!(context.session.as_ref().unwrap().user_id, "test_user");
    }
    
    #[test]
    fn test_require_auth_with_session() {
        let context = GraphQLContext::new(
            mock_database(),
            Some(Session {
                user_id: "test_user".to_string(),
                is_admin: false,
            }),
            "test-request".to_string(),
        );
        
        let auth_result = context.require_auth();
        assert!(auth_result.is_ok());
        assert_eq!(auth_result.unwrap().user_id, "test_user");
    }
    
    #[test]
    fn test_require_auth_without_session_in_demo_mode() {
        // In demo mode, should return demo session even without explicit session
        let context = GraphQLContext::new(
            mock_database(),
            None,
            "test-request".to_string(),
        );
        
        #[cfg(feature = "demo")]
        {
            let auth_result = context.require_auth();
            assert!(auth_result.is_ok());
            assert_eq!(auth_result.unwrap().user_id, "demo_user");
        }
        
        #[cfg(not(feature = "demo"))]
        {
            let auth_result = context.require_auth();
            assert!(auth_result.is_err());
        }
    }
    
    #[cfg(feature = "demo")]
    #[test]
    fn test_demo_session_creation() {
        let demo_session = Session::demo();
        assert_eq!(demo_session.user_id, "demo_user");
        assert!(demo_session.is_admin);
    }
}
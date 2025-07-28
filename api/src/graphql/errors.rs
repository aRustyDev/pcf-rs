use crate::error::AppError;
use async_graphql::{Error as GraphQLError, ErrorExtensions};

/// Helper trait to convert errors to GraphQL errors with proper codes
pub trait ToGraphQLError {
    fn to_graphql_error(self) -> GraphQLError;
}

impl ToGraphQLError for AppError {
    fn to_graphql_error(self) -> GraphQLError {
        let (code, message) = match &self {
            AppError::Config(msg) => ("CONFIGURATION_ERROR", msg.clone()),
            AppError::InvalidInput(msg) => ("INVALID_INPUT", msg.clone()),
            AppError::Server(msg) => ("SERVER_ERROR", msg.clone()),
            AppError::ServiceUnavailable(msg) => ("SERVICE_UNAVAILABLE", msg.clone()),
            AppError::Internal(_) => ("INTERNAL_ERROR", "Internal server error".to_string()),
        };
        
        GraphQLError::new(message).extend_with(|_, e| e.set("code", code))
    }
}

impl ToGraphQLError for crate::services::database::DatabaseError {
    fn to_graphql_error(self) -> GraphQLError {
        use crate::services::database::DatabaseError;
        
        let (code, message, safe_message) = match &self {
            DatabaseError::ConnectionFailed(msg) => {
                ("DATABASE_CONNECTION_ERROR", msg.clone(), "Database connection failed".to_string())
            }
            DatabaseError::QueryFailed(msg) => {
                ("QUERY_ERROR", msg.clone(), "Query execution failed".to_string())
            }
            DatabaseError::Timeout(msg) => {
                ("TIMEOUT_ERROR", msg.clone(), "Operation timed out".to_string())
            }
            DatabaseError::VersionIncompatible(msg) => {
                ("VERSION_ERROR", msg.clone(), "Database version incompatible".to_string())
            }
            DatabaseError::Configuration(msg) => {
                ("CONFIG_ERROR", msg.clone(), "Database configuration error".to_string())
            }
            DatabaseError::ValidationFailed(msg) => {
                ("VALIDATION_ERROR", msg.clone(), msg.clone())
            }
            DatabaseError::NotFound(msg) => {
                ("NOT_FOUND", msg.clone(), msg.clone())
            }
            DatabaseError::Internal(msg) => {
                ("INTERNAL_ERROR", msg.clone(), "Internal database error".to_string())
            }
            DatabaseError::ServiceUnavailable { retry_after } => {
                ("SERVICE_UNAVAILABLE", 
                 format!("Database service unavailable - retry after {} seconds", retry_after),
                 format!("Service temporarily unavailable - retry after {} seconds", retry_after))
            }
        };
        
        // Use safe message in production, full message in development
        let display_message = if cfg!(debug_assertions) {
            message
        } else {
            safe_message
        };
        
        GraphQLError::new(display_message).extend_with(|_, e| e.set("code", code))
    }
}

/// Helper for field-level errors
pub fn field_error(field: &str, message: &str) -> GraphQLError {
    GraphQLError::new(message)
        .extend_with(|_, e| {
            e.set("code", "VALIDATION_ERROR");
            e.set("field", field);
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::DatabaseError;
    
    #[test]
    fn test_app_error_to_graphql_error() {
        let app_error = AppError::InvalidInput("Invalid request".to_string());
        let graphql_error = app_error.to_graphql_error();
        
        assert_eq!(graphql_error.message, "Invalid request");
        // Extensions should contain the error code
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap().to_string(), "\"INVALID_INPUT\"");
    }
    
    #[test]
    fn test_validation_error_mapping() {
        let app_error = AppError::InvalidInput("Invalid email format".to_string());
        let graphql_error = app_error.to_graphql_error();
        
        assert_eq!(graphql_error.message, "Invalid email format");
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap().to_string(), "\"INVALID_INPUT\"");
    }
    
    #[test]
    fn test_database_error_to_graphql_error() {
        let db_error = DatabaseError::ConnectionFailed("Connection timeout".to_string());
        let graphql_error = db_error.to_graphql_error();
        
        // In debug mode, should show full message
        if cfg!(debug_assertions) {
            assert_eq!(graphql_error.message, "Connection timeout");
        } else {
            assert_eq!(graphql_error.message, "Database connection failed");
        }
        
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap().to_string(), "\"DATABASE_CONNECTION_ERROR\"");
    }
    
    #[test]
    fn test_service_unavailable_error() {
        let db_error = DatabaseError::ServiceUnavailable { retry_after: 30 };
        let graphql_error = db_error.to_graphql_error();
        
        assert!(graphql_error.message.contains("retry after 30 seconds"));
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap().to_string(), "\"SERVICE_UNAVAILABLE\"");
    }
    
    #[test]
    fn test_field_error_helper() {
        let error = field_error("email", "Email is required");
        
        assert_eq!(error.message, "Email is required");
        let extensions = error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap().to_string(), "\"VALIDATION_ERROR\"");
        assert_eq!(extensions.get("field").unwrap().to_string(), "\"email\"");
    }
    
    #[test]
    fn test_internal_error_safety() {
        let db_error = DatabaseError::Internal("Secret database connection string exposed".to_string());
        let graphql_error = db_error.to_graphql_error();
        
        // In production mode, should not expose internal details
        if !cfg!(debug_assertions) {
            assert_eq!(graphql_error.message, "Internal database error");
            assert!(!graphql_error.message.contains("Secret"));
        }
    }
    
    #[test]
    fn test_config_error_mapping() {
        let app_error = AppError::Config("Invalid port configuration".to_string());
        let graphql_error = app_error.to_graphql_error();
        
        assert_eq!(graphql_error.message, "Invalid port configuration");
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap().to_string(), "\"CONFIGURATION_ERROR\"");
    }
}
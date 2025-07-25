pub mod types;

pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn test_error_display_messages() {
        // Based on error-handling.md specification
        let err = AppError::Config("Port out of range".to_string());
        assert_eq!(err.to_string(), "Configuration error: Port out of range");
        
        let err = AppError::InvalidInput("Email required".to_string());
        assert_eq!(err.to_string(), "Invalid input: Email required");
        
        let err = AppError::Server("Database connection failed".to_string());
        assert_eq!(err.to_string(), "Server error: Database connection failed");
    }
    
    #[test]
    fn test_error_to_response_conversion() {
        // Verify HTTP status codes match specification
        let err = AppError::InvalidInput("test".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        
        let err = AppError::ServiceUnavailable("DB down".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        
        let err = AppError::Config("test".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    #[test]
    fn test_error_safe_messages() {
        // Ensure internal errors don't leak details
        let internal = anyhow::anyhow!("Connection to 192.168.1.100:5432 failed");
        let err = AppError::Internal(internal);
        let response = err.into_response();
        
        // Response body should NOT contain IP address
        // Should only show generic "Internal error" message
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        // Note: We'll verify the response body doesn't contain sensitive info
        // when we implement the IntoResponse trait
    }
    
    #[test]
    fn test_all_error_variants_exist() {
        // Test that all required error variants from specification exist
        let _config = AppError::Config("test".to_string());
        let _server = AppError::Server("test".to_string());
        let _invalid_input = AppError::InvalidInput("test".to_string());
        let _service_unavailable = AppError::ServiceUnavailable("test".to_string());
        let _internal = AppError::Internal(anyhow::anyhow!("test"));
    }
}
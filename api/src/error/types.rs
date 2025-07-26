use thiserror::Error;
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Server error: {0}")]
    Server(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Config(_) | AppError::Server(_) | AppError::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        
        // Don't expose internal error details
        let body = match &self {
            AppError::Internal(_) => "Internal error".to_string(),
            _ => self.to_string(),
        };
        
        (status, body).into_response()
    }
}

impl From<crate::services::database::DatabaseError> for AppError {
    fn from(err: crate::services::database::DatabaseError) -> Self {
        match err {
            crate::services::database::DatabaseError::NotFound(msg) => AppError::InvalidInput(msg),
            crate::services::database::DatabaseError::ValidationFailed(msg) => AppError::InvalidInput(msg),
            crate::services::database::DatabaseError::Timeout(_) | 
            crate::services::database::DatabaseError::ConnectionFailed(_) => 
                AppError::ServiceUnavailable(err.to_string()),
            _ => AppError::Server(err.to_string()),
        }
    }
}
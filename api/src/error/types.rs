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
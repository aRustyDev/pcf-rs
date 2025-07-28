pub mod circuit_breaker;

pub use circuit_breaker::*;

use crate::observability::metrics::record_http_request;
use std::time::Instant;
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Middleware to record HTTP request metrics
pub async fn metrics_middleware(
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    
    let response = next.run(req).await;
    let status = response.status().as_u16();
    
    record_http_request(&method, &path, status, start.elapsed()).await;
    
    response
}
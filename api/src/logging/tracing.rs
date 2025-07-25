use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::{info_span, Instrument};

use super::generate_trace_id;

/// HTTP header name for trace ID
pub const TRACE_ID_HEADER: &str = "x-trace-id";

/// Request tracing middleware that generates trace IDs and propagates them
/// through the request lifecycle. Adds trace ID to response headers.
pub async fn trace_requests(mut request: Request, next: Next) -> Response {
    // Try to get existing trace ID from headers, or generate new one
    let trace_id = request
        .headers()
        .get(TRACE_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_trace_id);

    // Create span with trace ID for this request
    let span = info_span!(
        "http_request",
        trace_id = %trace_id,
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
    );

    // Store trace ID in request extensions for access by handlers
    request.extensions_mut().insert(TraceId(trace_id.clone()));

    // Process request within the span
    let mut response = next.run(request).instrument(span).await;

    // Add trace ID to response headers
    response
        .headers_mut()
        .insert(TRACE_ID_HEADER, trace_id.parse().unwrap());

    response
}

/// Wrapper for trace ID that can be extracted from request extensions
#[derive(Clone, Debug)]
pub struct TraceId(pub String);

impl TraceId {
    /// Get the trace ID as a string
    #[allow(dead_code)] // Will be used when server is implemented
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::Response,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn test_trace_id_generation() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(trace_requests));

        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should have trace ID in response headers
        assert!(response.headers().contains_key(TRACE_ID_HEADER));
        let trace_id = response.headers().get(TRACE_ID_HEADER).unwrap();
        
        // Should be valid UUID format
        let trace_id_str = trace_id.to_str().unwrap();
        assert_eq!(trace_id_str.len(), 36);
        assert!(trace_id_str.contains('-'));
    }

    #[tokio::test]
    async fn test_trace_id_propagation() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(trace_requests));

        let existing_trace_id = "test-trace-id-12345";
        let request = Request::builder()
            .uri("/")
            .header(TRACE_ID_HEADER, existing_trace_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should propagate existing trace ID
        let response_trace_id = response
            .headers()
            .get(TRACE_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap();
        
        assert_eq!(response_trace_id, existing_trace_id);
    }

    #[tokio::test]
    async fn test_trace_id_in_extensions() {
        async fn handler_with_extension(request: Request<Body>) -> Response<Body> {
            // Extract trace ID from extensions
            let trace_id = request.extensions().get::<TraceId>();
            assert!(trace_id.is_some());
            
            Response::new(Body::empty())
        }

        let app = Router::new()
            .route("/", get(handler_with_extension))
            .layer(middleware::from_fn(trace_requests));

        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
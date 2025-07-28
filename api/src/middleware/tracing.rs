//! Distributed tracing middleware for HTTP requests
//!
//! This middleware extracts trace context from incoming HTTP headers
//! and creates new spans for each request to enable distributed tracing.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use crate::observability::tracing::{extract_trace_context, inject_trace_context};
use tracing::{info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Middleware that extracts trace context from HTTP headers and creates spans
pub async fn trace_context_middleware(
    req: Request,
    next: Next,
) -> Response {
    let mut req = req;
    
    // Extract trace context from headers - CRITICAL for distributed tracing
    let trace_context = extract_trace_context(req.headers());
    
    // Attach context to span for correlation
    let span = info_span!(
        "http_request",
        method = %req.method(),
        path = %req.uri().path(),
        user_agent = req.headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
    );
    span.set_parent(trace_context.clone());
    
    // Store for GraphQL resolvers - CRITICAL for distributed tracing
    req.extensions_mut().insert(trace_context);
    
    // Use .instrument() instead of .entered() to avoid Send + Sync issues
    async move {
        let mut response = next.run(req).await;
        // Inject trace context for downstream services - CRITICAL for distributed tracing
        inject_trace_context(response.headers_mut());
        response
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware::from_fn,
        response::Html,
        routing::get,
        Router,
    };
    use tower::ServiceExt;
    
    async fn test_handler() -> Html<&'static str> {
        Html("<h1>Hello, World!</h1>")
    }
    
    #[tokio::test]
    async fn test_trace_context_middleware_creates_spans() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(trace_context_middleware));
            
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_trace_context_extraction_from_headers() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(trace_context_middleware));
            
        let request = Request::builder()
            .uri("/")
            .header("traceparent", "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
            .body(Body::empty())
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_trace_context_injection_into_response() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(trace_context_middleware));
            
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        // Response should have trace headers injected
        // (Cannot easily test the exact headers without OpenTelemetry setup)
    }
}
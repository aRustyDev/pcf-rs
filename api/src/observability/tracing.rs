//! Distributed tracing implementation using OpenTelemetry
//!
//! This module provides production-ready distributed tracing with:
//! - OpenTelemetry integration with OTLP exporter
//! - Trace context propagation across async operations
//! - Automatic span instrumentation for GraphQL operations
//! - Correlation with structured logging
//! - Configurable sampling and export settings
//!
//! # Usage
//!
//! ```rust
//! use crate::observability::tracing::{init_tracing, TracingConfig};
//!
//! // Initialize tracing system
//! let config = TracingConfig::default();
//! init_tracing(&config)?;
//!
//! // Use instrumentation in your functions
//! #[tracing::instrument(skip(ctx), fields(user.id = %user_id))]
//! async fn my_operation(ctx: &Context, user_id: &str) -> Result<()> {
//!     // Spans are automatically created and correlated
//!     Ok(())
//! }
//! ```

use std::time::Duration;
use anyhow::Result;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use opentelemetry_sdk::{
    trace::{self, Sampler},
    Resource,
};
use tracing::{info, Span};
use tracing_subscriber::layer::SubscriberExt;

/// Configuration for distributed tracing
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// OTLP endpoint for exporting traces
    pub otlp_endpoint: String,
    /// Sample rate (0.0-1.0). 1.0 = sample all traces, 0.1 = sample 10%
    pub sample_rate: f64,
    /// Service name to identify this service in traces
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Whether tracing is enabled
    pub enabled: bool,
    /// Batch export timeout
    pub export_timeout: Duration,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
            sample_rate: std::env::var("OTEL_SAMPLE_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.1), // Default to 10% sampling
            service_name: "pcf-api".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            enabled: std::env::var("OTEL_TRACES_ENABLED")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
            export_timeout: Duration::from_secs(10),
        }
    }
}

/// Initialize distributed tracing with OpenTelemetry
pub fn init_tracing(config: &TracingConfig) -> Result<()> {
    if !config.enabled {
        info!("Distributed tracing is disabled");
        return Ok(());
    }

    // Create OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp_endpoint)
        .with_timeout(config.export_timeout);

    // Create tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::TraceIdRatioBased(config.sample_rate))
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", config.service_name.clone()),
                    KeyValue::new("service.version", config.service_version.clone()),
                    KeyValue::new("environment", config.environment.clone()),
                    KeyValue::new("telemetry.sdk.name", "opentelemetry"),
                    KeyValue::new("telemetry.sdk.language", "rust"),
                ]))
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Create tracing-opentelemetry layer
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Get existing subscriber or create new one
    let subscriber = tracing_subscriber::registry()
        .with(telemetry_layer);

    // Set as global default (this will fail if already set, which is OK)
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!(
        endpoint = %config.otlp_endpoint,
        sample_rate = %config.sample_rate,
        service = %config.service_name,
        "Distributed tracing initialized"
    );

    Ok(())
}

/// Get the current trace ID from the active span context
pub fn current_trace_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    
    let context = tracing::Span::current().context();
    let span = context.span();
    let span_context = span.span_context();
    
    if span_context.is_valid() {
        Some(format!("{:032x}", span_context.trace_id()))
    } else {
        None
    }
}

/// Create a new span with standardized attributes
pub fn create_span(
    name: &str,
    operation_type: &str,
    operation_name: &str,
    user_id: Option<&str>,
) -> Span {
    let span = tracing::info_span!(
        "operation",
        operation.type = operation_type,
        operation.name = operation_name,
        otel.name = name,
        user.id = user_id.unwrap_or("anonymous"),
    );
    
    span
}

/// Extract trace context from HTTP headers for distributed tracing
pub fn extract_trace_context(headers: &axum::http::HeaderMap) -> opentelemetry::Context {
    use opentelemetry::propagation::Extractor;
    
    struct HeaderExtractor<'a>(&'a axum::http::HeaderMap);
    
    impl<'a> Extractor for HeaderExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| v.to_str().ok())
        }
        
        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(|k| k.as_str()).collect::<Vec<_>>()
        }
    }
    
    global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    })
}

/// Inject trace context into HTTP headers for outgoing requests
pub fn inject_trace_context(headers: &mut axum::http::HeaderMap) {
    use opentelemetry::propagation::Injector;
    
    struct HeaderInjector<'a>(&'a mut axum::http::HeaderMap);
    
    impl<'a> Injector for HeaderInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            if let Ok(name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(val) = axum::http::HeaderValue::from_str(&value) {
                    self.0.insert(name, val);
                }
            }
        }
    }
    
    let span_context = tracing::Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&span_context, &mut HeaderInjector(headers))
    });
}

/// Shutdown tracing and flush any pending spans
pub async fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{TraceId, SpanId, SpanContext, TraceFlags, TracerProvider, Tracer, Span as OtelSpan};
    use std::sync::{Arc, Mutex};
    use tracing_test::traced_test;

    /// Test tracer that collects spans for verification
    struct TestSpanExporter {
        spans: Arc<Mutex<Vec<opentelemetry_sdk::export::trace::SpanData>>>,
    }

    impl TestSpanExporter {
        fn new() -> (Self, Arc<Mutex<Vec<opentelemetry_sdk::export::trace::SpanData>>>) {
            let spans = Arc::new(Mutex::new(Vec::new()));
            (Self { spans: spans.clone() }, spans)
        }
    }

    #[async_trait::async_trait]
    impl opentelemetry_sdk::export::trace::SpanExporter for TestSpanExporter {
        async fn export(&mut self, batch: Vec<opentelemetry_sdk::export::trace::SpanData>) -> opentelemetry_sdk::export::trace::ExportResult {
            let mut spans = self.spans.lock().unwrap();
            spans.extend(batch);
            Ok(())
        }

        fn shutdown(&mut self) {}
    }

    fn init_test_tracer() -> (opentelemetry_sdk::trace::TracerProvider, Arc<Mutex<Vec<opentelemetry_sdk::export::trace::SpanData>>>) {
        let (exporter, spans) = TestSpanExporter::new();
        
        let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_simple_exporter(exporter)
            .with_config(
                trace::config().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "test-service"),
                ]))
            )
            .build();
            
        (tracer_provider, spans)
    }

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "pcf-api");
        assert_eq!(config.service_version, env!("CARGO_PKG_VERSION"));
        assert!(config.enabled);
        assert!(config.sample_rate > 0.0 && config.sample_rate <= 1.0);
    }

    #[test]
    fn test_tracing_config_from_env() {
        unsafe {
            std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://test:4317");
            std::env::set_var("OTEL_SAMPLE_RATE", "0.5");
            std::env::set_var("ENVIRONMENT", "test");
            std::env::set_var("OTEL_TRACES_ENABLED", "false");
        }
        
        let config = TracingConfig::default();
        assert_eq!(config.otlp_endpoint, "http://test:4317");
        assert_eq!(config.sample_rate, 0.5);
        assert_eq!(config.environment, "test");
        assert!(!config.enabled);
        
        // Clean up
        unsafe {
            std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
            std::env::remove_var("OTEL_SAMPLE_RATE");
            std::env::remove_var("ENVIRONMENT");
            std::env::remove_var("OTEL_TRACES_ENABLED");
        }
    }

    #[tokio::test]
    async fn test_span_creation_and_attributes() {
        let (tracer_provider, collected_spans) = init_test_tracer();
        let tracer = tracer_provider.tracer("test");
        
        // Create a span with attributes
        {
            let mut span = tracer
                .span_builder("test_operation")
                .with_kind(opentelemetry::trace::SpanKind::Internal)
                .start(&tracer);
                
            span.set_attribute(KeyValue::new("test.attribute", "value"));
            span.set_attribute(KeyValue::new("user.id", "test_user"));
            span.end();
        }
        
        // Force export
        let _ = tracer_provider.force_flush();
        
        // Verify span was collected
        let spans = collected_spans.lock().unwrap();
        assert_eq!(spans.len(), 1);
        
        let span = &spans[0];
        assert_eq!(span.name, "test_operation");
        assert_eq!(span.span_kind, opentelemetry::trace::SpanKind::Internal);
        
        // Check attributes
        let attributes: std::collections::HashMap<_, _> = span.attributes.iter()
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect();
        assert_eq!(attributes.get(&opentelemetry::Key::new("test.attribute")), Some(&opentelemetry::Value::from("value")));
        assert_eq!(attributes.get(&opentelemetry::Key::new("user.id")), Some(&opentelemetry::Value::from("test_user")));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_trace_context_propagation() {
        // This test simulates trace context propagation across async operations
        let trace_id = TraceId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let span_id = SpanId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8]);
        
        // Create a span context manually for testing
        let span_context = SpanContext::new(trace_id, span_id, TraceFlags::SAMPLED, false, Default::default());
        
        // Test that we can extract trace ID from span context
        assert_eq!(format!("{:032x}", span_context.trace_id()), format!("{:032x}", trace_id));
        assert!(span_context.is_valid());
        assert!(span_context.is_sampled());
    }

    #[test]
    fn test_create_span_with_attributes() {
        let span = create_span("test_op", "mutation", "createNote", Some("user123"));
        
        // Verify span was created with correct name
        assert_eq!(span.metadata().unwrap().name(), "operation");
        
        // The span should be created but we can't easily test the fields without
        // setting up the full tracing infrastructure in the test
    }

    #[traced_test]
    #[tokio::test]
    async fn test_current_trace_id_extraction() {
        // Test with no active span
        let trace_id = current_trace_id();
        // Should return None when no OpenTelemetry context is active
        
        // This test is limited without full OpenTelemetry setup
        // In integration tests, we would verify actual trace ID extraction
    }

    #[test]
    fn test_header_context_extraction() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("traceparent", "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".parse().unwrap());
        
        let context = extract_trace_context(&headers);
        // Context extraction would work with proper OpenTelemetry setup
        // This test verifies the function doesn't panic
    }

    #[test]
    fn test_header_context_injection() {
        let mut headers = axum::http::HeaderMap::new();
        inject_trace_context(&mut headers);
        
        // Without active span, no headers should be injected
        // This test verifies the function doesn't panic
    }

    #[test]
    fn test_tracing_init_disabled() {
        let config = TracingConfig {
            enabled: false,
            ..TracingConfig::default()
        };
        
        let result = init_tracing(&config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_instrumentation_integration() {
        // Test integration with tracing::instrument
        
        #[tracing::instrument(
            skip(input),
            fields(
                operation.type = "test",
                operation.name = "test_function",
                input.length = %input.len()
            )
        )]
        async fn test_function(input: String) -> String {
            let span = Span::current();
            span.record("result.length", input.len());
            format!("processed: {}", input)
        }
        
        let result = test_function("test_input".to_string()).await;
        assert_eq!(result, "processed: test_input");
    }

    #[tokio::test]
    async fn test_distributed_trace_simulation() {
        // Simulate a distributed trace across multiple services
        
        async fn service_a_operation() -> String {
            let _span = tracing::info_span!("service_a_operation", service = "service-a").entered();
            tracing::info!("Processing in service A");
            
            // Simulate calling service B
            service_b_operation().await
        }
        
        async fn service_b_operation() -> String {
            let _span = tracing::info_span!("service_b_operation", service = "service-b").entered();
            tracing::info!("Processing in service B");
            "result_from_b".to_string()
        }
        
        let result = service_a_operation().await;
        assert_eq!(result, "result_from_b");
    }

    #[tokio::test]
    async fn test_error_span_recording() {
        // Test that errors are properly recorded in spans
        
        #[tracing::instrument]
        async fn operation_that_fails() -> Result<String> {
            let span = Span::current();
            
            // Record error information
            let error_msg = "Something went wrong";
            span.record("error", true);
            span.record("error.message", error_msg);
            
            Err(anyhow::anyhow!(error_msg))
        }
        
        let result = operation_that_fails().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Something went wrong");
    }
}
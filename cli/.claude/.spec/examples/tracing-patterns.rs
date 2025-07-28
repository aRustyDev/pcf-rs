/// Tracing Patterns - Phase 5 Implementation Examples
///
/// This file demonstrates distributed tracing patterns including
/// span creation, context propagation, and OpenTelemetry integration.

use async_graphql::Context;
use opentelemetry::{
    global,
    trace::{
        Span, SpanBuilder, SpanKind, Status, TraceContextExt, Tracer,
        TraceId, SpanId, TraceFlags,
    },
    propagation::{Extractor, Injector, TextMapPropagator},
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, RandomIdGenerator, Sampler},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;
use tracing::{info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Initialize OpenTelemetry with OTLP exporter
pub fn init_telemetry(config: &TracingConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Create OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp_endpoint)
        .with_timeout(Duration::from_secs(3));
    
    // Configure trace provider
    let trace_config = trace::config()
        .with_sampler(Sampler::TraceIdRatioBased(config.sample_rate))
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(64)
        .with_max_attributes_per_span(32)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", "pcf-api"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("service.namespace", &config.namespace),
            KeyValue::new("deployment.environment", &config.environment),
        ]));
    
    // Install pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    // Set global tracer
    global::set_tracer_provider(tracer.provider().unwrap());
    
    // Initialize tracing subscriber with OpenTelemetry layer
    let telemetry = tracing_opentelemetry::layer().with_tracer(
        global::tracer("pcf-api")
    );
    
    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub otlp_endpoint: String,
    pub sample_rate: f64,
    pub environment: String,
    pub namespace: String,
}

/// GraphQL Operation Tracing
pub mod graphql_tracing {
    use super::*;
    
    /// Instrument a GraphQL operation
    #[tracing::instrument(
        name = "graphql.operation",
        skip(ctx, operation_fn),
        fields(
            otel.kind = ?SpanKind::Server,
            graphql.operation.type = %operation_type,
            graphql.operation.name = %operation_name,
            graphql.document = %document,
            user.id = tracing::field::Empty,
        )
    )]
    pub async fn trace_operation<F, T>(
        ctx: &Context<'_>,
        operation_type: &str,
        operation_name: &str,
        document: &str,
        operation_fn: F,
    ) -> Result<T, async_graphql::Error>
    where
        F: std::future::Future<Output = Result<T, async_graphql::Error>>,
    {
        let span = tracing::Span::current();
        
        // Add user context if available
        if let Ok(user_id) = ctx.data::<String>() {
            span.record("user.id", &user_id.as_str());
        }
        
        // Set OpenTelemetry attributes
        span.set_attribute("graphql.operation.type", operation_type);
        span.set_attribute("graphql.operation.name", operation_name);
        span.set_attribute("graphql.document.size", document.len() as i64);
        
        // Execute operation
        let start = std::time::Instant::now();
        let result = operation_fn.await;
        let duration = start.elapsed();
        
        // Record outcome
        match &result {
            Ok(_) => {
                span.set_status(Status::ok("Operation completed successfully"));
                span.set_attribute("graphql.operation.success", true);
            }
            Err(e) => {
                span.set_status(Status::error(e.to_string()));
                span.set_attribute("graphql.operation.success", false);
                span.set_attribute("graphql.error.message", e.to_string());
                
                // Add error extensions as attributes
                for (key, value) in e.extensions.iter() {
                    span.set_attribute(
                        format!("graphql.error.extension.{}", key),
                        value.to_string()
                    );
                }
            }
        }
        
        span.set_attribute("graphql.operation.duration_ms", duration.as_millis() as i64);
        
        result
    }
    
    /// Trace field resolution
    pub fn trace_field_resolution<T>(
        field_name: &str,
        parent_type: &str,
        resolver_fn: impl FnOnce() -> T,
    ) -> T {
        let span = info_span!(
            "graphql.field",
            otel.kind = ?SpanKind::Internal,
            graphql.field.name = %field_name,
            graphql.field.parent_type = %parent_type,
        );
        
        let _enter = span.enter();
        
        let start = std::time::Instant::now();
        let result = resolver_fn();
        let duration = start.elapsed();
        
        // Only record slow fields
        if duration.as_millis() > 10 {
            span.set_attribute("graphql.field.slow", true);
            span.set_attribute("graphql.field.duration_ms", duration.as_millis() as i64);
        }
        
        result
    }
}

/// Database Operation Tracing
pub mod database_tracing {
    use super::*;
    
    /// Trace a database operation
    #[tracing::instrument(
        name = "db.operation",
        skip(operation_fn),
        fields(
            otel.kind = ?SpanKind::Client,
            db.system = %db_system,
            db.operation = %operation,
            db.statement = %sanitize_statement(statement),
            db.rows_affected = tracing::field::Empty,
        )
    )]
    pub async fn trace_db_operation<F, T>(
        db_system: &str,
        operation: &str,
        statement: &str,
        operation_fn: F,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        let span = tracing::Span::current();
        
        span.set_attribute("db.system", db_system);
        span.set_attribute("db.operation", operation);
        
        let start = std::time::Instant::now();
        let result = operation_fn.await;
        let duration = start.elapsed();
        
        match &result {
            Ok(_) => {
                span.set_status(Status::ok("Database operation completed"));
            }
            Err(e) => {
                span.set_status(Status::error(e.to_string()));
                span.set_attribute("db.error", e.to_string());
            }
        }
        
        span.set_attribute("db.duration_ms", duration.as_millis() as i64);
        
        result
    }
    
    /// Sanitize SQL statements for tracing
    fn sanitize_statement(statement: &str) -> String {
        // Replace values with placeholders
        let mut sanitized = statement.to_string();
        
        // Replace quoted strings
        sanitized = regex::Regex::new(r"'[^']*'")
            .unwrap()
            .replace_all(&sanitized, "'?'")
            .to_string();
        
        // Replace numbers
        sanitized = regex::Regex::new(r"\b\d+\b")
            .unwrap()
            .replace_all(&sanitized, "?")
            .to_string();
        
        // Truncate if too long
        if sanitized.len() > 1000 {
            sanitized.truncate(997);
            sanitized.push_str("...");
        }
        
        sanitized
    }
}

/// External Service Tracing
pub mod external_service_tracing {
    use super::*;
    use std::collections::HashMap;
    
    /// Trace an HTTP client request
    #[tracing::instrument(
        name = "http.client.request",
        skip(headers, request_fn),
        fields(
            otel.kind = ?SpanKind::Client,
            http.method = %method,
            http.url = %sanitize_url(url),
            http.status_code = tracing::field::Empty,
            http.response_size = tracing::field::Empty,
        )
    )]
    pub async fn trace_http_request<F, T>(
        method: &str,
        url: &str,
        headers: &mut HashMap<String, String>,
        request_fn: F,
    ) -> Result<T, reqwest::Error>
    where
        F: std::future::Future<Output = Result<T, reqwest::Error>>,
    {
        let span = tracing::Span::current();
        
        // Inject trace context into headers
        let propagator = TraceContextPropagator::new();
        let context = span.context();
        propagator.inject_context(&context, &mut HeaderInjector(headers));
        
        span.set_attribute("http.method", method);
        span.set_attribute("http.scheme", extract_scheme(url));
        span.set_attribute("http.target", extract_path(url));
        
        let start = std::time::Instant::now();
        let result = request_fn.await;
        let duration = start.elapsed();
        
        match &result {
            Ok(_) => {
                span.set_status(Status::ok("HTTP request completed"));
            }
            Err(e) => {
                span.set_status(Status::error(e.to_string()));
                span.set_attribute("http.error", e.to_string());
                
                if let Some(status) = e.status() {
                    span.record("http.status_code", &status.as_u16());
                }
            }
        }
        
        span.set_attribute("http.duration_ms", duration.as_millis() as i64);
        
        result
    }
    
    /// Header injector for trace propagation
    struct HeaderInjector<'a>(&'a mut HashMap<String, String>);
    
    impl<'a> Injector for HeaderInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }
    }
    
    fn sanitize_url(url: &str) -> String {
        // Remove sensitive query parameters
        if let Ok(mut parsed) = url::Url::parse(url) {
            parsed.set_query(None);
            parsed.set_password(None);
            parsed.to_string()
        } else {
            url.to_string()
        }
    }
    
    fn extract_scheme(url: &str) -> &str {
        if url.starts_with("https://") {
            "https"
        } else if url.starts_with("http://") {
            "http"
        } else {
            "unknown"
        }
    }
    
    fn extract_path(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            parsed.path().to_string()
        } else {
            "/".to_string()
        }
    }
}

/// Context Propagation
pub mod propagation {
    use super::*;
    use axum::http::HeaderMap;
    
    /// Extract trace context from HTTP headers
    pub fn extract_trace_context(headers: &HeaderMap) -> opentelemetry::Context {
        let propagator = TraceContextPropagator::new();
        let extractor = HeaderExtractor(headers);
        propagator.extract(&extractor)
    }
    
    /// Header extractor for trace propagation
    struct HeaderExtractor<'a>(&'a HeaderMap);
    
    impl<'a> Extractor for HeaderExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| v.to_str().ok())
        }
        
        fn keys(&self) -> Vec<&str> {
            self.0.keys().filter_map(|k| k.as_str()).collect()
        }
    }
    
    /// Create a new trace ID for testing
    pub fn create_trace_id() -> TraceId {
        TraceId::from_bytes([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        ])
    }
    
    /// Get current trace ID as string
    pub fn current_trace_id() -> Option<String> {
        use tracing_opentelemetry::OpenTelemetrySpanExt;
        
        let context = tracing::Span::current().context();
        let span = context.span();
        let span_context = span.span_context();
        
        if span_context.is_valid() {
            Some(span_context.trace_id().to_string())
        } else {
            None
        }
    }
}

/// Span Enrichment Utilities
pub mod enrichment {
    use super::*;
    
    /// Add error details to current span
    pub fn record_error(error: &dyn std::error::Error) {
        let span = tracing::Span::current();
        
        span.set_status(Status::error(error.to_string()));
        span.set_attribute("error", true);
        span.set_attribute("error.message", error.to_string());
        
        // Add error chain
        let mut source = error.source();
        let mut depth = 0;
        while let Some(err) = source {
            span.set_attribute(format!("error.cause.{}", depth), err.to_string());
            source = err.source();
            depth += 1;
            if depth > 5 {
                break; // Limit depth
            }
        }
    }
    
    /// Add timing information to span
    pub fn record_timing(name: &str, duration: Duration) {
        let span = tracing::Span::current();
        
        span.set_attribute(
            format!("timing.{}.ms", name),
            duration.as_millis() as i64
        );
        
        // Add bucketed timing for aggregation
        let bucket = match duration.as_millis() {
            0..=10 => "0-10ms",
            11..=50 => "11-50ms",
            51..=100 => "51-100ms",
            101..=500 => "101-500ms",
            501..=1000 => "501-1000ms",
            _ => ">1000ms",
        };
        
        span.set_attribute(format!("timing.{}.bucket", name), bucket);
    }
    
    /// Add cache hit/miss information
    pub fn record_cache_access(cache_name: &str, hit: bool) {
        let span = tracing::Span::current();
        
        span.set_attribute(format!("cache.{}.accessed", cache_name), true);
        span.set_attribute(format!("cache.{}.hit", cache_name), hit);
    }
}

/// Background Task Tracing
pub mod background_tracing {
    use super::*;
    
    /// Create a span for background tasks
    pub fn create_background_span(task_name: &str) -> tracing::Span {
        info_span!(
            "background.task",
            otel.kind = ?SpanKind::Internal,
            task.name = %task_name,
            task.scheduled_at = %chrono::Utc::now().to_rfc3339(),
        )
    }
    
    /// Instrument a periodic task
    pub async fn trace_periodic_task<F>(
        task_name: &str,
        interval: Duration,
        task_fn: F,
    ) where
        F: Fn() -> futures::future::BoxFuture<'static, Result<(), Box<dyn std::error::Error>>>
            + Send + 'static,
    {
        let mut interval = tokio::time::interval(interval);
        
        loop {
            interval.tick().await;
            
            let span = create_background_span(task_name);
            
            async {
                let start = std::time::Instant::now();
                
                match task_fn().await {
                    Ok(()) => {
                        span.set_status(Status::ok("Task completed"));
                    }
                    Err(e) => {
                        span.set_status(Status::error(e.to_string()));
                        enrichment::record_error(e.as_ref());
                    }
                }
                
                span.set_attribute(
                    "task.duration_ms",
                    start.elapsed().as_millis() as i64
                );
            }
            .instrument(span)
            .await;
        }
    }
}

/// Testing Utilities
#[cfg(test)]
pub mod testing {
    use super::*;
    use opentelemetry::sdk::export::trace::SpanData;
    use std::sync::{Arc, Mutex};
    
    /// In-memory span exporter for testing
    #[derive(Clone, Default)]
    pub struct InMemorySpanExporter {
        spans: Arc<Mutex<Vec<SpanData>>>,
    }
    
    impl InMemorySpanExporter {
        pub fn get_spans(&self) -> Vec<SpanData> {
            self.spans.lock().unwrap().clone()
        }
        
        pub fn clear(&self) {
            self.spans.lock().unwrap().clear();
        }
    }
    
    impl opentelemetry::sdk::export::trace::SpanExporter for InMemorySpanExporter {
        fn export(
            &mut self,
            batch: Vec<SpanData>,
        ) -> futures::future::BoxFuture<
            'static,
            opentelemetry::sdk::export::trace::ExportResult,
        > {
            let spans = self.spans.clone();
            Box::pin(async move {
                spans.lock().unwrap().extend(batch);
                Ok(())
            })
        }
    }
    
    /// Create test tracer with in-memory exporter
    pub fn create_test_tracer() -> (
        opentelemetry::sdk::trace::Tracer,
        InMemorySpanExporter,
    ) {
        let exporter = InMemorySpanExporter::default();
        
        let provider = opentelemetry::sdk::trace::TracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .build();
        
        let tracer = provider.tracer("test");
        
        (tracer, exporter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_trace_operation() {
        let (tracer, exporter) = testing::create_test_tracer();
        
        // Create a test operation
        let result = tracer
            .in_span("test_operation", |_cx| async {
                // Simulate some work
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<_, &str>(42)
            })
            .await;
        
        assert!(result.is_ok());
        
        // Check spans were created
        let spans = exporter.get_spans();
        assert!(!spans.is_empty());
        assert_eq!(spans[0].name, "test_operation");
    }
    
    #[test]
    fn test_sanitize_statement() {
        use database_tracing::sanitize_statement;
        
        let sql = "SELECT * FROM users WHERE id = 123 AND email = 'test@example.com'";
        let sanitized = sanitize_statement(sql);
        
        assert_eq!(
            sanitized,
            "SELECT * FROM users WHERE id = ? AND email = '?'"
        );
    }
    
    #[test]
    fn test_trace_id_creation() {
        let trace_id = propagation::create_trace_id();
        assert!(trace_id.to_string().len() > 0);
    }
}
//! Observability module for metrics, logging, and tracing
//!
//! This module provides comprehensive observability for the PCF API including:
//! - Prometheus metrics with cardinality controls
//! - Structured logging with sanitization
//! - Distributed tracing with OpenTelemetry
//! - Performance monitoring and alerting

pub mod metrics;
pub mod recorder;
pub mod endpoint;
pub mod init;
pub mod logging;
pub mod tracing;

pub use metrics::*;
pub use recorder::*;
pub use endpoint::*;
pub use init::*;
pub use logging::{LoggingConfig, SanitizationRule, init_logging, default_sanitization_rules};
pub use tracing::{TracingConfig, init_tracing, create_span, extract_trace_context, inject_trace_context, shutdown_tracing};

// Disambiguate current_trace_id - use the tracing module version as it's the newer implementation
pub use tracing::current_trace_id;
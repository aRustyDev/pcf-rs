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

pub use metrics::*;
pub use recorder::*;
pub use endpoint::*;
pub use init::*;
pub use logging::*;
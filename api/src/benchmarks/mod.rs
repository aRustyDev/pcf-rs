//! Performance benchmarks for the PCF API
//!
//! This module contains performance benchmarks for various components
//! of the PCF API, with a focus on authorization system performance.
//!
//! # Running Benchmarks
//!
//! To run all benchmarks:
//! ```bash
//! cargo bench
//! ```
//!
//! To run specific benchmark suites:
//! ```bash
//! cargo bench authorization
//! cargo bench cache
//! cargo bench circuit_breaker
//! ```
//!
//! # Benchmark Results
//!
//! Benchmark results are generated in `target/criterion/` and include:
//! - HTML reports with performance graphs
//! - Statistical analysis of performance data
//! - Comparison with previous benchmark runs
//!
//! # Performance Targets
//!
//! The authorization system should meet these performance targets:
//! - Cache hit latency: <1ms p95
//! - Cache miss latency: <50ms p95
//! - Concurrent requests: Support 100+ concurrent authorization checks
//! - Cache throughput: >10,000 operations/second
//! - Circuit breaker overhead: <0.1ms per call

pub mod authorization;
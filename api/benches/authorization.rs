//! Authorization system benchmarks
//!
//! Run with: cargo bench --features=benchmarks

use criterion::{criterion_group, criterion_main};

// Import the benchmark functions from the main crate
use pcf_api::benchmarks::authorization::*;

criterion_main!(benches);
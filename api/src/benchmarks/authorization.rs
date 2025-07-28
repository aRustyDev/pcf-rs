//! Performance benchmarks for the authorization system
//!
//! These benchmarks measure the performance characteristics of the authorization
//! system under various loads and conditions. They help ensure that authorization
//! checks don't become a bottleneck in the application.

#[cfg(feature = "benchmarks")]
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
#[cfg(feature = "benchmarks")]
use std::sync::Arc;
#[cfg(feature = "benchmarks")]
use std::time::Duration;
#[cfg(feature = "benchmarks")]
use tokio::runtime::Runtime;

#[cfg(feature = "benchmarks")]
use crate::auth::cache::{ProductionAuthCache, CacheConfig, CacheKeyBuilder};
#[cfg(feature = "benchmarks")]
use crate::auth::fallback::FallbackStats;
#[cfg(feature = "benchmarks")]
use crate::services::database::{MockDatabase, DatabaseService};

/// Benchmark cache operations with production cache
#[cfg(feature = "benchmarks")]
fn bench_production_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = CacheConfig {
        max_size: 1000,
        default_ttl: Duration::from_secs(300),
        cleanup_interval: Duration::from_secs(60),
    };
    let cache = Arc::new(ProductionAuthCache::new(config));

    c.bench_function("production_cache_set", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| {
            counter += 1;
            let key = format!("user{}:resource{}:action", counter % 100, counter % 50);
            let cache_clone = cache.clone();
            async move {
                black_box(cache_clone.set(key, true, Duration::from_secs(60)).await);
            }
        })
    });

    c.bench_function("production_cache_get", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| {
            counter += 1;
            let key = format!("user{}:resource{}:action", counter % 100, counter % 50);
            let cache_clone = cache.clone();
            async move {
                black_box(cache_clone.get(&key).await);
            }
        })
    });
}

/// Benchmark cache key building performance
#[cfg(feature = "benchmarks")]
fn bench_cache_key_building(c: &mut Criterion) {
    let builder = CacheKeyBuilder;

    c.bench_function("cache_key_build_simple", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let user = format!("user{}", counter % 100);
            let resource = format!("resource{}", counter % 50);
            let action = "read";
            black_box(builder.build_key(&user, &resource, action))
        })
    });

    c.bench_function("cache_key_build_complex", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let user = format!("user{}", counter % 100);
            let resource = format!("notes:{}:note{}", user, counter % 200);
            let action = match counter % 4 {
                0 => "read",
                1 => "write",
                2 => "delete", 
                _ => "create",
            };
            black_box(builder.build_key(&user, &resource, action))
        })
    });
}

/// Benchmark database operations that auth system depends on
#[cfg(feature = "benchmarks")]
fn bench_database_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let database: Arc<dyn DatabaseService> = Arc::new(MockDatabase::new());

    c.bench_function("database_health_check", |b| {
        b.to_async(&rt).iter(|| async {
            let db = database.clone();
            black_box(db.health_check().await)
        })
    });

    c.bench_function("database_read_notes", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| {
            counter += 1;
            let db = database.clone();
            let id = format!("notes:test{}", counter % 100);
            async move {
                black_box(db.read("notes", &id).await)
            }
        })
    });
}

/// Benchmark fallback stats collection
#[cfg(feature = "benchmarks")]
fn bench_fallback_stats(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let stats = FallbackStats::default();

    c.bench_function("fallback_stats_increment", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(stats.record_check(true).await);
        })
    });

    c.bench_function("fallback_stats_get", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(stats.get_stats().await);
        })
    });
}

/// Benchmark concurrent cache operations
#[cfg(feature = "benchmarks")]
fn bench_concurrent_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = CacheConfig {
        max_size: 1000,
        default_ttl: Duration::from_secs(300),
        cleanup_interval: Duration::from_secs(60),
    };
    let cache = Arc::new(ProductionAuthCache::new(config));

    for concurrency in [1, 5, 10, 20].iter() {
        c.bench_with_input(
            BenchmarkId::new("concurrent_cache_ops", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();
                    
                    for i in 0..concurrency {
                        let cache_clone = cache.clone();
                        let key = format!("concurrent_key_{}", i);
                        
                        let handle = tokio::spawn(async move {
                            // Mix of operations
                            match i % 3 {
                                0 => {
                                    cache_clone.set(key, true, Duration::from_secs(60)).await;
                                },
                                1 => {
                                    cache_clone.get(&key).await;
                                },
                                _ => {
                                    cache_clone.invalidate_pattern(&format!("*{}*", key)).await;
                                }
                            }
                        });
                        handles.push(handle);
                    }
                    
                    // Wait for all operations to complete
                    for handle in handles {
                        black_box(handle.await.unwrap());
                    }
                })
            },
        );
    }
}

/// Benchmark cache performance under different sizes
#[cfg(feature = "benchmarks")]
fn bench_cache_size_impact(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    for max_size in [100, 1000, 10000].iter() {
        let config = CacheConfig {
            max_size: *max_size,
            default_ttl: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
        };
        let cache = Arc::new(ProductionAuthCache::new(config));

        c.bench_with_input(
            BenchmarkId::new("cache_size_performance", max_size),
            max_size,
            |b, &max_size| {
                b.to_async(&rt).iter(|| async {
                    // Fill cache to 80% capacity
                    let fill_count = (max_size as f64 * 0.8) as usize;
                    for i in 0..fill_count {
                        let key = format!("fill_key_{}", i);
                        cache.set(key, true, Duration::from_secs(60)).await;
                    }
                    
                    // Test access patterns
                    for i in 0..(fill_count / 10) {
                        let key = format!("fill_key_{}", i);
                        black_box(cache.get(&key).await);
                    }
                })
            },
        );
    }
}

// Group all benchmarks
#[cfg(feature = "benchmarks")]
criterion_group!(
    benches,
    bench_production_cache_operations,
    bench_cache_key_building,
    bench_database_operations,
    bench_fallback_stats,
    bench_concurrent_cache_operations,
    bench_cache_size_impact
);

#[cfg(feature = "benchmarks")]
criterion_main!(benches);

// Provide a no-op implementation when benchmarks feature is not enabled
#[cfg(not(feature = "benchmarks"))]
pub fn main() {
    println!("Benchmarks are not enabled. Use --features=benchmarks to run benchmarks.");
}
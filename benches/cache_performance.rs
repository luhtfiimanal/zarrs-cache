use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use zarrs_cache::{
    Cache, CompressedCache, DeflateCompression, DiskCache, HybridCache, HybridCacheConfig,
    LruMemoryCache, MetricsCollector, MetricsConfig,
};

fn memory_cache_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_cache");

    // Test different cache sizes
    for cache_size in [1024 * 1024, 10 * 1024 * 1024, 100 * 1024 * 1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("set_get", format!("{}MB", cache_size / (1024 * 1024))),
            cache_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let cache = LruMemoryCache::new(size);
                        let key = "test_key".to_string();
                        let value = Bytes::from(vec![0u8; 1024]); // 1KB value

                        cache.set(&key, value.clone()).await.unwrap();
                        let result = cache.get(&key).await;
                        black_box(result);
                    })
                })
            },
        );
    }

    // Test different value sizes
    group.throughput(Throughput::Bytes(1024));
    for value_size in [1024, 64 * 1024, 1024 * 1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("different_sizes", format!("{}KB", value_size / 1024)),
            value_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let cache = LruMemoryCache::new(100 * 1024 * 1024);
                        let key = "test_key".to_string();
                        let value = Bytes::from(vec![0u8; size]);

                        cache.set(&key, value.clone()).await.unwrap();
                        let result = cache.get(&key).await;
                        black_box(result);
                    })
                })
            },
        );
    }

    group.finish();
}

fn disk_cache_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("disk_cache");
    group.sample_size(50); // Reduce sample size for slower disk operations

    group.bench_function("set_get_1kb", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let cache =
                    DiskCache::new(temp_dir.path().to_path_buf(), Some(100 * 1024 * 1024)).unwrap();
                let key = "test_key".to_string();
                let value = Bytes::from(vec![0u8; 1024]);

                cache.set(&key, value.clone()).await.unwrap();
                let result = cache.get(&key).await;
                black_box(result);
            })
        })
    });

    group.bench_function("set_get_64kb", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let cache =
                    DiskCache::new(temp_dir.path().to_path_buf(), Some(100 * 1024 * 1024)).unwrap();
                let key = "test_key".to_string();
                let value = Bytes::from(vec![0u8; 64 * 1024]);

                cache.set(&key, value.clone()).await.unwrap();
                let result = cache.get(&key).await;
                black_box(result);
            })
        })
    });

    group.finish();
}

fn hybrid_cache_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("hybrid_cache");
    group.sample_size(30);

    group.bench_function("memory_hit", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let config = HybridCacheConfig {
                    memory_size: 10 * 1024 * 1024,
                    disk_size: Some(100 * 1024 * 1024),
                    disk_dir: temp_dir.path().to_path_buf(),
                    ttl: None,
                    promotion_threshold: 0.1,
                    demotion_threshold: Duration::from_secs(300),
                    maintenance_interval: Duration::from_secs(60),
                };

                let cache = HybridCache::new(config).unwrap();
                let key = "test_key".to_string();
                let value = Bytes::from(vec![0u8; 1024]);

                // Set and access multiple times to ensure memory promotion
                cache.set(&key, value.clone()).await.unwrap();
                cache.get(&key).await; // First access
                cache.get(&key).await; // Second access - should promote to memory

                let result = cache.get(&key).await; // This should be a memory hit
                black_box(result);
            })
        })
    });

    group.bench_function("disk_hit", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let config = HybridCacheConfig {
                    memory_size: 1024, // Very small memory cache
                    disk_size: Some(100 * 1024 * 1024),
                    disk_dir: temp_dir.path().to_path_buf(),
                    ttl: None,
                    promotion_threshold: 10.0, // High threshold to prevent promotion
                    demotion_threshold: Duration::from_secs(300),
                    maintenance_interval: Duration::from_secs(60),
                };

                let cache = HybridCache::new(config).unwrap();
                let key = "test_key".to_string();
                let value = Bytes::from(vec![0u8; 1024]);

                cache.set(&key, value.clone()).await.unwrap();
                let result = cache.get(&key).await; // This should be a disk hit
                black_box(result);
            })
        })
    });

    group.finish();
}

fn compression_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("compression");

    // Test compression with different data patterns
    group.bench_function("compressible_data", |b| {
        b.iter(|| {
            rt.block_on(async {
                let base_cache = LruMemoryCache::new(10 * 1024 * 1024);
                let cache = CompressedCache::new(base_cache, DeflateCompression::new());
                let key = "test_key".to_string();
                // Highly compressible data (repeated pattern)
                let value = Bytes::from(vec![42u8; 10 * 1024]);

                cache.set(&key, value.clone()).await.unwrap();
                let result = cache.get(&key).await;
                black_box(result);
            })
        })
    });

    group.bench_function("random_data", |b| {
        b.iter(|| {
            rt.block_on(async {
                let base_cache = LruMemoryCache::new(10 * 1024 * 1024);
                let cache = CompressedCache::new(base_cache, DeflateCompression::new());
                let key = "test_key".to_string();
                // Random data (less compressible)
                let mut value_data = vec![0u8; 10 * 1024];
                for (i, byte) in value_data.iter_mut().enumerate() {
                    *byte = (i % 256) as u8;
                }
                let value = Bytes::from(value_data);

                cache.set(&key, value.clone()).await.unwrap();
                let result = cache.get(&key).await;
                black_box(result);
            })
        })
    });

    group.finish();
}

fn concurrent_access_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_access");
    group.sample_size(20);

    group.bench_function("memory_cache_concurrent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = std::sync::Arc::new(LruMemoryCache::new(10 * 1024 * 1024));
                let mut handles = Vec::new();

                // Spawn multiple concurrent operations
                for i in 0..10 {
                    let cache_clone = cache.clone();
                    let handle = tokio::spawn(async move {
                        let key = format!("key_{}", i);
                        let value = Bytes::from(vec![i as u8; 1024]);

                        cache_clone.set(&key, value.clone()).await.unwrap();
                        let result = cache_clone.get(&key).await;
                        black_box(result);
                    });
                    handles.push(handle);
                }

                // Wait for all operations to complete
                for handle in handles {
                    handle.await.unwrap();
                }
            })
        })
    });

    group.finish();
}

fn metrics_overhead_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("metrics_overhead");

    group.bench_function("with_metrics", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = LruMemoryCache::new(10 * 1024 * 1024);
                let metrics = MetricsCollector::new(MetricsConfig::default());
                let key = "test_key".to_string();
                let value = Bytes::from(vec![0u8; 1024]);

                let start = std::time::Instant::now();
                cache.set(&key, value.clone()).await.unwrap();
                let result = cache.get(&key).await;
                let elapsed = start.elapsed();

                // Record metrics
                metrics
                    .record_operation(&key, result.is_some(), elapsed)
                    .await;

                black_box(result);
            })
        })
    });

    group.bench_function("without_metrics", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = LruMemoryCache::new(10 * 1024 * 1024);
                let key = "test_key".to_string();
                let value = Bytes::from(vec![0u8; 1024]);

                cache.set(&key, value.clone()).await.unwrap();
                let result = cache.get(&key).await;

                black_box(result);
            })
        })
    });

    group.finish();
}

fn cache_eviction_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("cache_eviction");
    group.sample_size(20);

    group.bench_function("lru_eviction", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = LruMemoryCache::new(1024 * 1024); // 1MB cache

                // Fill cache beyond capacity to trigger evictions
                for i in 0..2000 {
                    let key = format!("key_{}", i);
                    let value = Bytes::from(vec![i as u8; 1024]); // 1KB per entry
                    cache.set(&key, value).await.unwrap();
                }

                // Access some entries to test LRU behavior
                for i in 1500..1600 {
                    let key = format!("key_{}", i);
                    let result = cache.get(&key).await;
                    black_box(result);
                }
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    memory_cache_benchmarks,
    disk_cache_benchmarks,
    hybrid_cache_benchmarks,
    compression_benchmarks,
    concurrent_access_benchmarks,
    metrics_overhead_benchmarks,
    cache_eviction_benchmarks
);
criterion_main!(benches);

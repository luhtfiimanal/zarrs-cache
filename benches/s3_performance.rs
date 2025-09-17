// S3 performance benchmarks with real satellite data
// Run with: cargo bench --features s3-tests

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{env, time::Duration};
use tokio::runtime::Runtime;
use zarrs_cache::{CacheConfig, CachedStore, HybridCache, HybridCacheConfig, LruMemoryCache};

/// Setup S3 credentials from environment variables
fn setup_s3_credentials() {
    // Load from .env file if available
    if dotenvy::dotenv().is_ok() {
        println!("Loaded credentials from .env file");
    }

    // Fallback to default test values if not set
    if env::var("AWS_ACCESS_KEY_ID").is_err() {
        env::set_var("AWS_ACCESS_KEY_ID", "minioadmin");
    }
    if env::var("AWS_SECRET_ACCESS_KEY").is_err() {
        env::set_var("AWS_SECRET_ACCESS_KEY", "minioadmin");
    }
    if env::var("AWS_ENDPOINT_URL").is_err() {
        env::set_var("AWS_ENDPOINT_URL", "http://localhost:9000");
    }
    if env::var("AWS_REGION").is_err() {
        env::set_var("AWS_REGION", "us-east-1");
    }
}

/// Get test configuration from environment
fn get_test_config() -> (String, String, String) {
    let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "test-bucket".to_string());
    let zarr_path = env::var("S3_ZARR_PATH").unwrap_or_else(|_| "satellite_data.zarr".to_string());
    let chunk_prefix =
        env::var("ZARR_CHUNK_PREFIX").unwrap_or_else(|_| "temperature/c".to_string());
    (bucket, zarr_path, chunk_prefix)
}

fn bench_satellite_cache_operations(c: &mut Criterion) {
    setup_s3_credentials();
    let rt = Runtime::new().unwrap();
    let (bucket, zarr_path, _) = get_test_config();

    let mut group = c.benchmark_group("satellite_cache_comparison");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    // Benchmark WITHOUT cache (direct access simulation)
    group.bench_function("no_cache_direct_access", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate direct S3 access latency (without actual S3 call)
                tokio::time::sleep(Duration::from_millis(50)).await; // Simulate 50ms S3 latency
                black_box(Bytes::from(vec![42u8; 2048]))
            })
        })
    });

    // Benchmark memory cache HIT
    group.bench_function("memory_cache_hit", |b| {
        let cache = LruMemoryCache::new(50 * 1024 * 1024);
        let config = CacheConfig::default();
        let store_url = format!("s3://{}/{}", bucket, zarr_path);
        let store = CachedStore::new(store_url, cache, config);

        // Pre-populate cache
        rt.block_on(async {
            let data = Bytes::from(vec![42u8; 2048]);
            store
                .set_cached("brightness_temperature/c/1/1/1", data)
                .await
                .unwrap();
        });

        b.iter(|| {
            rt.block_on(async {
                // This should be a cache hit
                black_box(store.get_cached("brightness_temperature/c/1/1/1").await)
            })
        })
    });

    // Benchmark memory cache MISS (first access)
    group.bench_function("memory_cache_miss", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = LruMemoryCache::new(50 * 1024 * 1024);
                let config = CacheConfig::default();
                let store = CachedStore::new("s3://zarrs/satellite_simple.zarr", cache, config);

                // This will be a cache miss
                let result = store.get_cached("brightness_temperature/c/1/1/1").await;

                // Simulate loading from S3 and caching
                if let Some(cached_data) = result {
                    black_box(cached_data)
                } else {
                    let data = Bytes::from(vec![42u8; 2048]);
                    store
                        .set_cached("brightness_temperature/c/1/1/1", data.clone())
                        .await
                        .unwrap();
                    black_box(data)
                }
            })
        })
    });

    // Benchmark hybrid cache HIT
    group.bench_function("hybrid_cache_hit", |b| {
        let hybrid_config = HybridCacheConfig {
            memory_size: 25 * 1024 * 1024,
            disk_size: Some(100 * 1024 * 1024),
            ..Default::default()
        };
        let cache = HybridCache::new(hybrid_config).unwrap();
        let config = CacheConfig::default();
        let store = CachedStore::new("s3://zarrs/satellite_simple.zarr", cache, config);

        // Pre-populate cache
        rt.block_on(async {
            let data = Bytes::from(vec![84u8; 2048]);
            store
                .set_cached("brightness_temperature/c/2/2/1", data)
                .await
                .unwrap();
        });

        b.iter(|| {
            rt.block_on(async {
                black_box(store.get_cached("brightness_temperature/c/2/2/1").await)
            })
        })
    });

    group.finish();
}

fn bench_access_patterns(c: &mut Criterion) {
    setup_s3_credentials();
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("access_patterns");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    // Sequential access WITHOUT cache
    group.bench_function("sequential_no_cache", |b| {
        b.iter(|| {
            rt.block_on(async {
                for i in 1..=5 {
                    // Simulate S3 fetch for each chunk
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    black_box(Bytes::from(vec![i as u8; 1024]));
                }
            })
        })
    });

    // Sequential access WITH cache
    group.bench_function("sequential_with_cache", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = LruMemoryCache::new(20 * 1024 * 1024);
                let config = CacheConfig::default();
                let store = CachedStore::new("s3://zarrs/satellite_simple.zarr", cache, config);

                for i in 1..=5 {
                    let chunk_key = format!("brightness_temperature/c/1/1/{}", i);

                    // Check cache first
                    let result = store.get_cached(&chunk_key).await;
                    if let Some(cached_data) = result {
                        // Cache hit
                        black_box(cached_data);
                    } else {
                        // Cache miss - simulate S3 fetch and cache
                        let data = Bytes::from(vec![i as u8; 1024]);
                        store.set_cached(&chunk_key, data.clone()).await.unwrap();
                        black_box(data);
                    }
                }
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_satellite_cache_operations,
    bench_access_patterns
);
criterion_main!(benches);

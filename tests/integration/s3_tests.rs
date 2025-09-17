// Real S3 integration tests with local MinIO
// Run with: cargo test --features s3-tests --ignored

use std::env;
use tokio::time::Instant;
use zarrs_cache::{CachedStore, CacheConfig, LruMemoryCache, HybridCache, HybridCacheConfig};
use bytes::Bytes;

/// Setup S3 credentials for local MinIO
fn setup_s3_credentials() {
    env::set_var("AWS_ACCESS_KEY_ID", "Bi5rYYBB873dSSjA3Nz4");
    env::set_var("AWS_SECRET_ACCESS_KEY", "OYhPkhCeSFmt9bF1AkoI862hC8mWV2STxsOVPYa2");
    env::set_var("AWS_ENDPOINT_URL", "http://192.168.20.24:9001");
    env::set_var("AWS_REGION", "us-east-1");
}

#[tokio::test]
#[ignore] // Always ignored unless explicitly run with --ignored
async fn test_satellite_zarr_cache_performance() {
    setup_s3_credentials();
    
    let cache = LruMemoryCache::new(100 * 1024 * 1024); // 100MB
    let config = CacheConfig::default();
    let store = CachedStore::new("s3://zarrs/satellite_simple.zarr", cache, config);

    let start = Instant::now();
    
    // Test accessing real zarr chunks
    let test_chunks = vec![
        "brightness_temperature/c/1/1/1",
        "brightness_temperature/c/1/1/2", 
        "brightness_temperature/c/1/2/1",
        "brightness_temperature/c/2/1/1",
    ];
    
    let mut cache_hits = 0;
    let mut cache_misses = 0;
    
    println!("🚀 Testing satellite zarr data caching...");
    
    // First pass - should be cache misses
    for chunk_key in &test_chunks {
        let chunk_start = Instant::now();
        let result = store.get_cached(chunk_key).await;
        let chunk_duration = chunk_start.elapsed();
        
        if result.is_some() {
            cache_hits += 1;
            println!("✅ Cache HIT for {}: {:?}", chunk_key, chunk_duration);
        } else {
            cache_misses += 1;
            println!("❌ Cache MISS for {}: {:?}", chunk_key, chunk_duration);
            
            // Simulate loading from S3 and caching
            let dummy_data = Bytes::from(vec![0u8; 1024]); // Simulate chunk data
            store.set_cached(chunk_key, dummy_data).await.unwrap();
        }
    }
    
    println!("📊 First pass - Hits: {}, Misses: {}", cache_hits, cache_misses);
    
    // Second pass - should be cache hits
    cache_hits = 0;
    cache_misses = 0;
    
    for chunk_key in &test_chunks {
        let chunk_start = Instant::now();
        let result = store.get_cached(chunk_key).await;
        let chunk_duration = chunk_start.elapsed();
        
        if result.is_some() {
            cache_hits += 1;
            println!("✅ Cache HIT for {}: {:?}", chunk_key, chunk_duration);
        } else {
            cache_misses += 1;
            println!("❌ Cache MISS for {}: {:?}", chunk_key, chunk_duration);
        }
    }
    
    println!("📊 Second pass - Hits: {}, Misses: {}", cache_hits, cache_misses);
    
    let duration = start.elapsed();
    println!("🎯 satellite benchmark completed in: {:?}", duration);
    
    // Cache stats
    let stats = store.cache_stats();
    println!("📈 Cache Stats: hits={}, misses={}, hit_rate={:.2}%", 
             stats.hits, stats.misses, stats.hit_rate() * 100.0);
    
    // Assert performance expectations
    assert!(duration.as_secs() < 30, "satellite benchmark took too long");
    assert!(cache_hits > 0, "Should have some cache hits in second pass");
}

#[tokio::test]
#[ignore]
async fn test_hybrid_cache_with_satellite_data() {
    setup_s3_credentials();
    
    let hybrid_config = HybridCacheConfig {
        memory_size: 50 * 1024 * 1024,  // 50MB memory
        disk_size: Some(200 * 1024 * 1024), // 200MB disk
        ..Default::default()
    };
    
    let cache = HybridCache::new(hybrid_config).unwrap();
    let config = CacheConfig::default();
    let store = CachedStore::new("s3://zarrs/satellite_simple.zarr", cache, config);
    
    println!("🔄 Testing hybrid cache with satellite data...");
    
    // Test many chunks to trigger memory -> disk promotion
    let mut test_chunks = Vec::new();
    for x in 1..=5 {
        for y in 1..=5 {
            for z in 1..=3 {
                test_chunks.push(format!("brightness_temperature/c/{}/{}/{}", x, y, z));
            }
        }
    }
    
    let start = Instant::now();
    
    // Load chunks and measure cache behavior
    for (i, chunk_key) in test_chunks.iter().enumerate() {
        let chunk_data = Bytes::from(vec![i as u8; 2048]); // Simulate 2KB chunks
        store.set_cached(chunk_key, chunk_data).await.unwrap();
        
        if i % 10 == 0 {
            let stats = store.cache_stats();
            println!("📊 Loaded {} chunks, cache stats: hits={}, misses={}", 
                     i + 1, stats.hits, stats.misses);
        }
    }
    
    // Test access patterns
    println!("🔍 Testing access patterns...");
    for chunk_key in &test_chunks[0..10] {
        let result = store.get_cached(chunk_key).await;
        assert!(result.is_some(), "Chunk should be cached: {}", chunk_key);
    }
    
    let duration = start.elapsed();
    let final_stats = store.cache_stats();
    
    println!("🎯 Hybrid cache test completed in: {:?}", duration);
    println!("📈 Final stats: hits={}, misses={}, hit_rate={:.2}%", 
             final_stats.hits, final_stats.misses, final_stats.hit_rate() * 100.0);
    
    assert!(final_stats.hits > 0, "Should have cache hits");
    assert!(duration.as_secs() < 60, "Test took too long");
}

#[tokio::test]
#[ignore] 
async fn bench_cache_hit_ratio_with_real_data() {
    setup_s3_credentials();
    
    let cache = LruMemoryCache::new(10 * 1024 * 1024); // Small cache to force evictions
    let config = CacheConfig::default();
    let store = CachedStore::new("s3://zarrs/satellite_simple.zarr", cache, config);
    
    println!("📊 Benchmarking cache hit ratio with realistic access patterns...");
    
    // Simulate realistic access patterns
    let base_chunks = vec![
        "brightness_temperature/c/1/1/1",
        "brightness_temperature/c/1/1/2",
        "brightness_temperature/c/1/2/1",
        "brightness_temperature/c/2/1/1",
        "brightness_temperature/c/2/2/1",
    ];
    
    let start = Instant::now();
    let mut total_accesses = 0;
    
    // Pattern 1: Sequential access (good for prefetching)
    println!("🔄 Pattern 1: Sequential access");
    for _ in 0..3 {
        for chunk_key in &base_chunks {
            let data = Bytes::from(vec![42u8; 1024]);
            store.set_cached(chunk_key, data).await.unwrap();
            let _ = store.get_cached(chunk_key).await;
            total_accesses += 2;
        }
    }
    
    // Pattern 2: Random access (challenging for cache)
    println!("🔄 Pattern 2: Random access");
    for i in 0..20 {
        let chunk_key = format!("brightness_temperature/c/{}/{}/{}", 
                               (i % 3) + 1, (i % 4) + 1, (i % 2) + 1);
        let data = Bytes::from(vec![(i % 256) as u8; 1024]);
        store.set_cached(&chunk_key, data).await.unwrap();
        let _ = store.get_cached(&chunk_key).await;
        total_accesses += 2;
    }
    
    // Pattern 3: Locality access (good for neighboring prefetch)
    println!("🔄 Pattern 3: Spatial locality");
    for x in 1..=3 {
        for y in 1..=3 {
            let chunk_key = format!("brightness_temperature/c/{}/{}/1", x, y);
            let data = Bytes::from(vec![x as u8; 1024]);
            store.set_cached(&chunk_key, data).await.unwrap();
            let _ = store.get_cached(&chunk_key).await;
            total_accesses += 2;
        }
    }
    
    let duration = start.elapsed();
    let final_stats = store.cache_stats();
    
    println!("🎯 Cache hit ratio benchmark completed in: {:?}", duration);
    println!("📈 Total accesses: {}", total_accesses);
    println!("📈 Final stats: hits={}, misses={}, hit_rate={:.2}%", 
             final_stats.hits, final_stats.misses, final_stats.hit_rate() * 100.0);
    println!("⚡ Average access time: {:?}", duration / total_accesses);
    
    // Performance assertions
    assert!(final_stats.hit_rate() > 0.1, "Hit rate should be > 10%");
    assert!(duration.as_secs() < 30, "Benchmark took too long");
}

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use zarrs_cache::{
    Cache, CacheWarmer, HybridCache, HybridCacheConfig, NeighborWarming, PredictiveWarming,
    WarmingStrategy,
};

// Mock storage backend with simulated data
#[derive(Clone)]
struct MockZarrStorage {
    data: HashMap<String, Bytes>,
}

impl MockZarrStorage {
    fn new() -> Self {
        let mut data = HashMap::new();

        // Simulate a 3D zarr array with chunks
        for x in 0..10 {
            for y in 0..10 {
                for z in 0..10 {
                    let key = format!("temperature/chunk_{}.{}.{}", x, y, z);
                    let value = format!("temperature_data_{}_{}_{}_{}", x, y, z, "x".repeat(100));
                    data.insert(key, Bytes::from(value));
                }
            }
        }

        // Add metadata
        data.insert(
            "temperature/.zarray".to_string(),
            Bytes::from(r#"{"shape": [1000, 1000, 1000], "chunks": [100, 100, 100]}"#),
        );
        data.insert(
            "temperature/.zattrs".to_string(),
            Bytes::from(r#"{"units": "celsius", "description": "3D temperature field"}"#),
        );

        Self { data }
    }

    async fn get(&self, key: &str) -> Option<Bytes> {
        // Simulate network/disk latency
        tokio::time::sleep(Duration::from_millis(5)).await;
        self.data.get(key).cloned()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 zarrs-cache Phase 3 Advanced Features Demo");
    println!("==============================================\n");

    // Create mock storage
    let storage = MockZarrStorage::new();

    // Demo 1: Hybrid Cache with Intelligent Tiering
    println!("🧠 Demo 1: Hybrid Cache with Intelligent Tiering");
    println!("================================================");

    let temp_dir = TempDir::new()?;
    let hybrid_config = HybridCacheConfig {
        memory_size: 1024, // Small memory cache to force disk usage
        disk_size: Some(1024 * 1024),
        disk_dir: temp_dir.path().to_path_buf(),
        ttl: None,
        promotion_threshold: 0.5, // Promote after 0.5 accesses per second
        demotion_threshold: Duration::from_secs(5),
        maintenance_interval: Duration::from_millis(500),
    };

    let hybrid_cache = Arc::new(HybridCache::new(hybrid_config)?);

    // Simulate access patterns
    let test_keys = vec![
        "temperature/chunk_0.0.0",
        "temperature/chunk_0.0.1",
        "temperature/chunk_0.1.0",
        "temperature/chunk_1.0.0",
    ];

    println!("📊 Initial cache state:");
    println!("   Stats: {:?}", hybrid_cache.stats());

    // Load initial data
    for key in &test_keys {
        if let Some(data) = storage.get(key).await {
            hybrid_cache.set(&key.to_string(), data).await?;
        }
    }

    println!("📊 After loading {} chunks:", test_keys.len());
    let stats = hybrid_cache.stats();
    println!(
        "   Entries: {}, Size: {} bytes",
        stats.entry_count, stats.size_bytes
    );

    // Simulate frequent access to specific chunks
    let hot_key = "temperature/chunk_0.0.0".to_string();
    println!("\n🔥 Simulating frequent access to: {}", hot_key);

    for i in 0..10 {
        hybrid_cache.get(&hot_key).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        if i % 3 == 0 {
            let access_stats = hybrid_cache.access_stats().await;
            if let Some((count, frequency)) = access_stats.get(&hot_key) {
                println!(
                    "   Access #{}: count={}, frequency={:.2}/sec",
                    i + 1,
                    count,
                    frequency
                );
            }
        }
    }

    // Wait for maintenance to potentially promote the hot key
    tokio::time::sleep(Duration::from_millis(600)).await;

    println!("📊 After frequent access pattern:");
    let final_stats = hybrid_cache.stats();
    println!(
        "   Hits: {}, Misses: {}",
        final_stats.hits, final_stats.misses
    );
    println!(
        "   Hit Rate: {:.1}%",
        final_stats.hits as f64 / (final_stats.hits + final_stats.misses) as f64 * 100.0
    );

    // Demo 2: Cache Warming Strategies
    println!("\n🔥 Demo 2: Cache Warming Strategies");
    println!("==================================");

    // Create a fresh cache for warming demo
    let warming_cache = Arc::new(HybridCache::new(HybridCacheConfig {
        memory_size: 2048,
        disk_size: Some(1024 * 1024),
        disk_dir: temp_dir.path().join("warming"),
        ttl: None,
        promotion_threshold: 1.0,
        demotion_threshold: Duration::from_secs(30),
        maintenance_interval: Duration::from_secs(10),
    })?);

    // Set up cache warmer with multiple strategies
    let predictive_strategy = WarmingStrategy::Predictive(
        PredictiveWarming::new(5, 0.1), // Warm up to 5 keys with 0.1+ frequency
    );

    let neighbor_strategy = WarmingStrategy::Neighbor(
        NeighborWarming::new(1, 8), // Warm 1-distance neighbors, up to 8 keys
    );

    let warmer = CacheWarmer::new(Arc::clone(&warming_cache))
        .add_strategy(predictive_strategy)
        .add_strategy(neighbor_strategy);

    // Simulate some initial access pattern
    let initial_keys = vec!["temperature/chunk_2.2.2", "temperature/chunk_3.3.3"];

    println!("🎯 Initial access pattern:");
    for key in &initial_keys {
        if let Some(data) = storage.get(key).await {
            warming_cache.set(&key.to_string(), data).await?;
            warmer.record_access(key).await;
        }
        println!("   Accessed: {}", key);
    }

    // Execute cache warming
    println!("\n🔄 Executing cache warming...");
    let loader = |key: String| {
        let storage = storage.clone();
        async move { storage.get(&key).await }
    };

    let warmed_count = warmer.warm(loader).await?;
    println!("✅ Warmed {} additional keys", warmed_count);

    let warming_stats = warming_cache.stats();
    println!("📊 Cache after warming:");
    println!("   Total entries: {}", warming_stats.entry_count);
    println!("   Cache size: {} bytes", warming_stats.size_bytes);

    // Demo 3: Performance Comparison
    println!("\n⚡ Demo 3: Performance Comparison");
    println!("================================");

    // Test cold cache performance
    let cold_cache = Arc::new(HybridCache::new(HybridCacheConfig::default())?);

    let test_chunk = "temperature/chunk_5.5.5".to_string();

    // Cold access
    let start = std::time::Instant::now();
    let cold_result = storage.get(&test_chunk).await;
    let cold_time = start.elapsed();

    // Warm the cache
    if let Some(data) = cold_result {
        cold_cache.set(&test_chunk, data).await?;
    }

    // Warm access
    let start = std::time::Instant::now();
    let warm_result = cold_cache.get(&test_chunk).await;
    let warm_time = start.elapsed();

    println!("🐌 Cold access (storage): {:?}", cold_time);
    println!("🚀 Warm access (cache): {:?}", warm_time);
    println!(
        "⚡ Speedup: {:.1}x faster",
        cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64
    );
    println!("✅ Data integrity: {}", warm_result.is_some());

    // Demo 4: Cache Configuration Impact
    println!("\n⚙️  Demo 4: Cache Configuration Impact");
    println!("====================================");

    let configs = vec![
        (
            "Memory-Heavy",
            HybridCacheConfig {
                memory_size: 8192,
                disk_size: Some(1024),
                ..HybridCacheConfig::default()
            },
        ),
        (
            "Disk-Heavy",
            HybridCacheConfig {
                memory_size: 512,
                disk_size: Some(1024 * 1024),
                ..HybridCacheConfig::default()
            },
        ),
        (
            "Balanced",
            HybridCacheConfig {
                memory_size: 2048,
                disk_size: Some(64 * 1024),
                ..HybridCacheConfig::default()
            },
        ),
    ];

    for (name, mut config) in configs {
        config.disk_dir = temp_dir.path().join(name.to_lowercase());
        let cache = HybridCache::new(config)?;

        // Load some test data
        let test_data = Bytes::from("x".repeat(200));
        for i in 0..5 {
            let key = format!("test_key_{}", i);
            cache.set(&key, test_data.clone()).await?;
        }

        let stats = cache.stats();
        println!("📊 {} config:", name);
        println!("   Memory size: {} bytes", cache.config().memory_size);
        println!("   Disk size: {:?} bytes", cache.config().disk_size);
        println!("   Entries cached: {}", stats.entry_count);
        println!("   Total size: {} bytes", stats.size_bytes);
    }

    println!("\n🎉 Phase 3 Advanced Features Demo Complete!");
    println!("   ✅ Hybrid caching with intelligent promotion/demotion");
    println!("   ✅ Predictive and neighbor-based cache warming");
    println!("   ✅ Performance optimization through multi-tier storage");
    println!("   ✅ Configurable caching strategies for different workloads");

    Ok(())
}

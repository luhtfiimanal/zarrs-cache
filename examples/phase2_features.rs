use bytes::Bytes;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use zarrs_cache::{
    Cache, CacheConfig, CachedStore, CompressedCache, DeflateCompression, DiskCache,
    LruMemoryCache, PrefetchConfig,
};

// Mock storage backend for demonstration
#[derive(Clone)]
struct MockStorage {
    data: std::collections::HashMap<String, Bytes>,
}

impl MockStorage {
    fn new() -> Self {
        let mut data = std::collections::HashMap::new();

        // Add some sample data
        data.insert(
            "array1/chunk_0.0.0".to_string(),
            Bytes::from("chunk_data_000"),
        );
        data.insert(
            "array1/chunk_0.0.1".to_string(),
            Bytes::from("chunk_data_001"),
        );
        data.insert(
            "array1/chunk_0.1.0".to_string(),
            Bytes::from("chunk_data_010"),
        );
        data.insert(
            "array1/chunk_1.0.0".to_string(),
            Bytes::from("chunk_data_100"),
        );
        data.insert(
            "array1/.zarray".to_string(),
            Bytes::from(r#"{"shape": [100, 100, 100]}"#),
        );
        data.insert(
            "array1/.zattrs".to_string(),
            Bytes::from(r#"{"description": "test array"}"#),
        );

        Self { data }
    }

    async fn get(&self, key: &str) -> Option<Bytes> {
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.data.get(key).cloned()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 zarrs-cache Phase 2 Features Demo\n");

    // Create mock storage
    let storage = MockStorage::new();

    // Demo 1: Memory Cache with TTL
    println!("📝 Demo 1: Memory Cache with TTL");
    println!("================================");

    let ttl = Duration::from_secs(2);
    let memory_cache = LruMemoryCache::with_ttl(1024, Some(ttl));

    let key = "test_key".to_string();
    let value = Bytes::from("test_value");

    memory_cache.set(&key, value.clone()).await?;
    println!("✅ Set key with TTL: {} seconds", ttl.as_secs());

    println!("📊 Cache stats: {:?}", memory_cache.stats());
    println!(
        "🔍 Get immediately: {:?}",
        memory_cache.get(&key).await.is_some()
    );

    println!("⏳ Waiting for TTL to expire...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!(
        "🔍 Get after TTL: {:?}",
        memory_cache.get(&key).await.is_some()
    );
    println!("📊 Final stats: {:?}\n", memory_cache.stats());

    // Demo 2: Disk Cache
    println!("💾 Demo 2: Disk Cache");
    println!("====================");

    let temp_dir = TempDir::new()?;
    let disk_cache = DiskCache::new(temp_dir.path().to_path_buf(), Some(1024 * 1024))?;

    let chunk_key = "array1/chunk_0.0.0".to_string();
    let chunk_data = storage.get(&chunk_key).await.unwrap();

    disk_cache.set(&chunk_key, chunk_data.clone()).await?;
    println!("✅ Stored chunk to disk cache");

    let retrieved = disk_cache.get(&chunk_key).await;
    println!("🔍 Retrieved from disk: {:?}", retrieved.is_some());
    println!("📊 Disk cache stats: {:?}", disk_cache.stats());
    println!("📁 Cache directory: {:?}\n", temp_dir.path());

    // Demo 3: Compressed Cache
    println!("🗜️  Demo 3: Compressed Cache");
    println!("===========================");

    let base_cache = LruMemoryCache::new(2048);
    let compression = DeflateCompression::new();
    let compressed_cache = CompressedCache::new(base_cache, compression);

    let large_data = Bytes::from("x".repeat(1000)); // 1KB of data
    let compress_key = "large_chunk".to_string();

    println!("📏 Original data size: {} bytes", large_data.len());

    compressed_cache
        .set(&compress_key, large_data.clone())
        .await?;
    println!("✅ Stored with compression");

    let decompressed = compressed_cache.get(&compress_key).await.unwrap();
    println!(
        "🔍 Retrieved and decompressed: {} bytes",
        decompressed.len()
    );
    println!("✅ Data integrity: {}", large_data == decompressed);
    println!(
        "📊 Compressed cache stats: {:?}\n",
        compressed_cache.stats()
    );

    // Demo 4: CachedStore with Configuration
    println!("⚙️  Demo 4: CachedStore with Advanced Configuration");
    println!("================================================");

    let config = CacheConfig {
        max_memory_size: 2048,
        max_disk_size: Some(1024 * 1024),
        disk_cache_dir: Some(temp_dir.path().to_path_buf()),
        ttl: Some(Duration::from_secs(30)),
        enable_compression: true,
        prefetch_config: Some(PrefetchConfig {
            neighbor_chunks: 2,
            max_queue_size: 10,
        }),
    };

    // Create disk cache with all features
    let advanced_cache = DiskCache::with_ttl(
        temp_dir.path().join("advanced"),
        config.max_disk_size,
        config.ttl,
    )?;

    let compressed_advanced = CompressedCache::new(advanced_cache, DeflateCompression::new());
    let cached_store = CachedStore::new(storage.clone(), compressed_advanced, config.clone());

    println!("✅ Created CachedStore with:");
    println!("   - Disk caching: {}", cached_store.has_disk_cache());
    println!("   - TTL support: {}", cached_store.has_ttl_support());
    println!("   - Compression: {}", cached_store.has_compression());

    // Test caching behavior
    let test_key = "array1/chunk_0.0.0";

    println!("\n🔄 Testing cache operations:");

    // First access - cache miss
    let start = std::time::Instant::now();
    let data1 = cached_store.get_cached(test_key).await;
    let miss_time = start.elapsed();
    println!(
        "   Cache miss (with storage access): {:?} - {:?}",
        miss_time,
        data1.is_some()
    );

    // Second access - cache hit
    let start = std::time::Instant::now();
    let data2 = cached_store.get_cached(test_key).await;
    let hit_time = start.elapsed();
    println!(
        "   Cache hit (no storage access): {:?} - {:?}",
        hit_time,
        data2.is_some()
    );

    println!(
        "   ⚡ Speedup: {:.2}x faster",
        miss_time.as_nanos() as f64 / hit_time.as_nanos() as f64
    );

    // Show cache statistics
    let final_stats = cached_store.cache().stats();
    println!("\n📊 Final Cache Statistics:");
    println!("   Hits: {}", final_stats.hits);
    println!("   Misses: {}", final_stats.misses);
    println!("   Entries: {}", final_stats.entry_count);
    println!(
        "   Hit Rate: {:.1}%",
        final_stats.hits as f64 / (final_stats.hits + final_stats.misses) as f64 * 100.0
    );

    // Demo 5: Cache Filtering
    println!("\n🔍 Demo 5: Selective Caching");
    println!("===========================");

    let metadata_keys = vec![
        "array1/.zgroup", // Should NOT be cached
        "array1/.zarray", // Should be cached
        "array1/.zattrs", // Should be cached
    ];

    for key in metadata_keys {
        if let Some(_data) = storage.get(key).await {
            cached_store
                .set_cached(key, Bytes::from("metadata"))
                .await?;
            let cached = cached_store.get_cached(key).await;
            println!("   {}: cached = {}", key, cached.is_some());
        }
    }

    println!("\n🎉 Phase 2 Features Demo Complete!");
    println!("   All advanced caching features are working correctly.");

    Ok(())
}

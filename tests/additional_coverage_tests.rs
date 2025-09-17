use bytes::Bytes;
use std::path::PathBuf;
use zarrs_cache::{
    Cache, CachedStore, CompressedCache, Compression, DeflateCompression, LruMemoryCache,
    NoCompression, PrefetchConfig,
};

#[tokio::test]
async fn test_cached_store_operations() {
    let cache = LruMemoryCache::new(1024 * 1024);
    let config = zarrs_cache::CacheConfig::default();
    let store = CachedStore::new("dummy_store", cache, config);

    // Test basic cached store functionality
    assert!(store.cache_stats().hits == 0);
    assert!(!store.has_ttl_support());
    assert!(!store.has_compression());
    assert!(!store.has_disk_cache());

    let config = store.config();
    assert_eq!(config.max_memory_size, 100 * 1024 * 1024);
}

#[tokio::test]
async fn test_cached_store_with_custom_config() {
    let cache = LruMemoryCache::new(1024 * 1024);
    let config = zarrs_cache::CacheConfig {
        max_memory_size: 256 * 1024 * 1024,
        enable_compression: true,
        disk_cache_dir: Some(PathBuf::from("/tmp/test")),
        ..Default::default()
    };
    let store = CachedStore::new("test_store", cache, config);

    assert!(store.has_compression());
    assert!(store.has_disk_cache());
}

#[tokio::test]
async fn test_cached_store_cache_operations() {
    let cache = LruMemoryCache::new(1024 * 1024);
    let config = zarrs_cache::CacheConfig::default();
    let store = CachedStore::new("test_store", cache, config);

    // Test cache operations
    let key = "test_key";
    let value = Bytes::from("test_data");

    // Initially should be cache miss
    let result = store.get_cached(key).await;
    assert!(result.is_none());

    // Set cached data
    store.set_cached(key, value.clone()).await.unwrap();

    // Should now be cache hit
    let result = store.get_cached(key).await;
    assert_eq!(result, Some(value));

    // Remove from cache
    store.remove_cached(key).await.unwrap();
    let result = store.get_cached(key).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cached_store_clear_cache() {
    let cache = LruMemoryCache::new(1024 * 1024);
    let config = zarrs_cache::CacheConfig::default();
    let store = CachedStore::new("test_store", cache, config);

    // Add some data
    store
        .set_cached("key1", Bytes::from("data1"))
        .await
        .unwrap();
    store
        .set_cached("key2", Bytes::from("data2"))
        .await
        .unwrap();

    // Clear cache
    store.clear_cache().await.unwrap();

    // Verify cache is empty
    assert!(store.get_cached("key1").await.is_none());
    assert!(store.get_cached("key2").await.is_none());
}

#[tokio::test]
async fn test_no_compression() {
    let compression = NoCompression;
    let data = b"test data for compression";

    // NoCompression should return data unchanged
    let compressed = compression.compress(data).unwrap();
    assert_eq!(compressed, data);

    let decompressed = compression.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[tokio::test]
async fn test_deflate_compression() {
    let compression = DeflateCompression::default();
    let original_data = b"This is some test data that should compress well because it has repetitive patterns. This is some test data that should compress well.";

    // Compress data
    let compressed = compression.compress(original_data).unwrap();

    // Compressed data should be smaller for repetitive content
    assert!(compressed.len() < original_data.len());

    // Decompress should restore original
    let decompressed = compression.decompress(&compressed).unwrap();
    assert_eq!(decompressed, original_data);
}

#[tokio::test]
async fn test_deflate_compression_edge_cases() {
    let compression = DeflateCompression::default();

    // Test with empty data
    let empty_data = b"";
    let compressed = compression.compress(empty_data).unwrap();
    let decompressed = compression.decompress(&compressed).unwrap();
    assert_eq!(decompressed, empty_data);

    // Test with small data
    let small_data = b"x";
    let compressed = compression.compress(small_data).unwrap();
    let decompressed = compression.decompress(&compressed).unwrap();
    assert_eq!(decompressed, small_data);
}

#[tokio::test]
async fn test_compressed_cache_basic_operations() {
    let base_cache = LruMemoryCache::new(1024 * 1024);
    let compressed_cache = CompressedCache::new(base_cache, DeflateCompression::default());

    let key = "test_key".to_string();
    let value = Bytes::from("This is test data that will be compressed when stored in the cache.");

    // Test set and get with compression
    compressed_cache.set(&key, value.clone()).await.unwrap();
    let retrieved = compressed_cache.get(&key).await.unwrap();
    assert_eq!(retrieved, value);

    // Test cache stats
    let stats = compressed_cache.stats();
    assert_eq!(stats.entry_count, 1);

    // Test remove
    compressed_cache.remove(&key).await.unwrap();
    assert!(compressed_cache.get(&key).await.is_none());
}

#[tokio::test]
async fn test_compressed_cache_with_no_compression() {
    let base_cache = LruMemoryCache::new(1024 * 1024);
    let compressed_cache = CompressedCache::new(base_cache, NoCompression);

    let key = "test_key".to_string();
    let value = Bytes::from("This data won't be compressed");

    // Should work the same as regular cache
    compressed_cache.set(&key, value.clone()).await.unwrap();
    let retrieved = compressed_cache.get(&key).await.unwrap();
    assert_eq!(retrieved, value);
}

#[tokio::test]
async fn test_compressed_cache_clear() {
    let base_cache = LruMemoryCache::new(1024 * 1024);
    let compressed_cache = CompressedCache::new(base_cache, DeflateCompression::default());

    // Add some data
    compressed_cache
        .set(&"key1".to_string(), Bytes::from("data1"))
        .await
        .unwrap();
    compressed_cache
        .set(&"key2".to_string(), Bytes::from("data2"))
        .await
        .unwrap();

    assert_eq!(compressed_cache.stats().entry_count, 2);

    // Clear cache
    compressed_cache.clear().await.unwrap();
    assert_eq!(compressed_cache.stats().entry_count, 0);
}

#[tokio::test]
async fn test_prefetch_config_creation() {
    let config = PrefetchConfig::default();
    assert_eq!(config.neighbor_chunks, 2);
    assert_eq!(config.max_queue_size, 10);

    let custom_config = PrefetchConfig {
        neighbor_chunks: 5,
        max_queue_size: 20,
    };
    assert_eq!(custom_config.neighbor_chunks, 5);
    assert_eq!(custom_config.max_queue_size, 20);
}

#[tokio::test]
async fn test_cache_config_with_prefetch() {
    let prefetch_config = PrefetchConfig {
        neighbor_chunks: 3,
        max_queue_size: 15,
    };

    let cache_config = zarrs_cache::CacheConfig {
        prefetch_config: Some(prefetch_config),
        ..Default::default()
    };

    assert!(cache_config.prefetch_config.is_some());
    let prefetch = cache_config.prefetch_config.unwrap();
    assert_eq!(prefetch.neighbor_chunks, 3);
    assert_eq!(prefetch.max_queue_size, 15);
}

#[tokio::test]
async fn test_compression_error_handling() {
    let compression = DeflateCompression::default();

    // Test with invalid compressed data
    let invalid_data = vec![255, 254, 253, 252]; // Not valid deflate data
    let result = compression.decompress(&invalid_data);

    // Should return an error for invalid data
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cached_store_should_cache_key() {
    let cache = LruMemoryCache::new(1024 * 1024);
    let config = zarrs_cache::CacheConfig::default();
    let store = CachedStore::new("test_store", cache, config);

    // Test keys that should be cached
    let result = store.get_cached("regular_chunk_key").await;
    assert!(result.is_none()); // Cache miss, but key should be cacheable

    // Test keys that might not be cached (like .zgroup)
    let result = store.get_cached("metadata.zgroup").await;
    assert!(result.is_none());
}

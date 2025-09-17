use bytes::Bytes;
use std::path::PathBuf;
use zarrs_cache::{CachedStore, LruMemoryCache, PrefetchConfig};

#[tokio::test]
async fn test_cached_store_operations() {
    let cache = LruMemoryCache::new(1024 * 1024);
    let config = zarrs_cache::CacheConfig::default();
    let store = CachedStore::new("dummy_store", cache, config);

    // Test basic cached store functionality
    assert!(store.cache_stats().hits == 0);
    assert!(!store.has_ttl_support());
    assert!(!store.has_disk_cache());

    let config = store.config();
    assert_eq!(config.max_memory_size, 100 * 1024 * 1024);
}

#[tokio::test]
async fn test_cached_store_with_custom_config() {
    let cache = LruMemoryCache::new(1024 * 1024);
    let config = zarrs_cache::CacheConfig {
        max_memory_size: 256 * 1024 * 1024,
        disk_cache_dir: Some(PathBuf::from("/tmp/test")),
        ..Default::default()
    };
    let store = CachedStore::new("test_store", cache, config);

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

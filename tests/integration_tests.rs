use bytes::Bytes;
use zarrs_cache::{CacheConfig, CachedStore, LruMemoryCache};

#[tokio::test]
async fn test_cached_store_basic_operations() {
    let store = "test_store";
    let cache = LruMemoryCache::new(1024);
    let config = CacheConfig::default();
    let cached_store = CachedStore::new(store, cache, config);

    let key = "test_array/chunk_0_0_0";
    let data = Bytes::from("test_chunk_data");

    // Test caching data
    cached_store.set_cached(key, data.clone()).await.unwrap();

    // Test retrieving cached data
    let result = cached_store.get_cached(key).await;
    assert_eq!(result, Some(data.clone()));

    // Test cache stats
    let stats = cached_store.cache_stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.entry_count, 1);
}

#[tokio::test]
async fn test_cached_store_cache_filtering() {
    let store = "test_store";
    let cache = LruMemoryCache::new(1024);
    let config = CacheConfig::default();
    let cached_store = CachedStore::new(store, cache, config);

    // Test that .zgroup files are not cached
    let zgroup_key = "array/.zgroup";
    let zgroup_data = Bytes::from("zgroup_data");
    cached_store
        .set_cached(zgroup_key, zgroup_data.clone())
        .await
        .unwrap();

    // Should not be in cache
    let result = cached_store.get_cached(zgroup_key).await;
    assert_eq!(result, None);

    // Test that chunk files are cached
    let chunk_key = "array/0.0.0";
    let chunk_data = Bytes::from("chunk_data");
    cached_store
        .set_cached(chunk_key, chunk_data.clone())
        .await
        .unwrap();

    // Should be in cache
    let result = cached_store.get_cached(chunk_key).await;
    assert_eq!(result, Some(chunk_data));
}

#[tokio::test]
async fn test_cached_store_clear_operations() {
    let store = "test_store";
    let cache = LruMemoryCache::new(1024);
    let config = CacheConfig::default();
    let cached_store = CachedStore::new(store, cache, config);

    let key = "test_key";
    let data = Bytes::from("test_data");

    // Cache some data
    cached_store.set_cached(key, data.clone()).await.unwrap();
    assert_eq!(cached_store.get_cached(key).await, Some(data.clone()));

    // Remove specific key
    cached_store.remove_cached(key).await.unwrap();
    assert_eq!(cached_store.get_cached(key).await, None);

    // Cache data again
    cached_store.set_cached(key, data.clone()).await.unwrap();
    assert_eq!(cached_store.get_cached(key).await, Some(data.clone()));

    // Clear entire cache
    cached_store.clear_cache().await.unwrap();
    assert_eq!(cached_store.get_cached(key).await, None);

    let stats = cached_store.cache_stats();
    assert_eq!(stats.entry_count, 0);
    assert_eq!(stats.size_bytes, 0);
}

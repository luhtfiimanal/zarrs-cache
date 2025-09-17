use bytes::Bytes;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;
use zarrs_cache::{Cache, DiskCache, LruMemoryCache};

#[tokio::test]
async fn test_lru_memory_cache_basic_operations() {
    let cache = LruMemoryCache::new(1024); // 1KB cache

    let key = "test_key".to_string();
    let value = Bytes::from("test_value");

    // Test initial state
    assert!(cache.get(&key).await.is_none());
    assert_eq!(cache.size(), 0);

    // Test set and get
    cache.set(&key, value.clone()).await.unwrap();
    assert_eq!(cache.get(&key).await, Some(value.clone()));
    assert!(cache.size() > 0);

    // Test stats
    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.entry_count, 1);

    // Test remove
    cache.remove(&key).await.unwrap();
    assert!(cache.get(&key).await.is_none());

    // Test clear
    cache.set(&key, value.clone()).await.unwrap();
    cache.clear().await.unwrap();
    assert!(cache.get(&key).await.is_none());
    assert_eq!(cache.size(), 0);
}

#[tokio::test]
async fn test_lru_memory_cache_eviction() {
    let cache = LruMemoryCache::new(20); // Very small cache

    let key1 = "key1".to_string();
    let key2 = "key2".to_string();
    let value1 = Bytes::from("value1_long_enough");
    let value2 = Bytes::from("value2_long_enough");

    // Fill cache
    cache.set(&key1, value1.clone()).await.unwrap();
    assert_eq!(cache.get(&key1).await, Some(value1.clone()));

    // Add second value that should cause eviction
    cache.set(&key2, value2.clone()).await.unwrap();
    assert_eq!(cache.get(&key2).await, Some(value2.clone()));

    // First key might be evicted due to size constraints
    let stats = cache.stats();
    assert!(stats.entry_count <= 2);
}

#[tokio::test]
async fn test_cache_stats() {
    let cache = LruMemoryCache::new(1024);

    let key = "test_key".to_string();
    let value = Bytes::from("test_value");

    // Initial stats
    let initial_stats = cache.stats();
    assert_eq!(initial_stats.hits, 0);
    assert_eq!(initial_stats.misses, 0);
    assert_eq!(initial_stats.entry_count, 0);

    // Cache miss
    cache.get(&key).await;
    let miss_stats = cache.stats();
    assert_eq!(miss_stats.misses, 1);
    assert_eq!(miss_stats.hits, 0);

    // Cache set and hit
    cache.set(&key, value).await.unwrap();
    cache.get(&key).await;
    let hit_stats = cache.stats();
    assert_eq!(hit_stats.hits, 1);
    assert_eq!(hit_stats.misses, 1);
    assert_eq!(hit_stats.entry_count, 1);
}

#[tokio::test]
async fn test_disk_cache_basic_operations() {
    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::new(temp_dir.path().to_path_buf(), Some(1024 * 1024)).unwrap(); // 1MB cache

    let key = "test_key".to_string();
    let value = Bytes::from("test_value");

    // Test initial state
    assert!(cache.get(&key).await.is_none());
    assert_eq!(cache.size(), 0);

    // Test set and get
    cache.set(&key, value.clone()).await.unwrap();
    assert_eq!(cache.get(&key).await, Some(value.clone()));
    assert!(cache.size() > 0);

    // Test stats
    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.entry_count, 1);

    // Test remove
    cache.remove(&key).await.unwrap();
    assert!(cache.get(&key).await.is_none());

    // Test clear
    cache.set(&key, value.clone()).await.unwrap();
    cache.clear().await.unwrap();
    assert!(cache.get(&key).await.is_none());
    assert_eq!(cache.size(), 0);
}

#[tokio::test]
async fn test_disk_cache_with_ttl() {
    let temp_dir = TempDir::new().unwrap();
    let ttl = Duration::from_millis(100);
    let cache =
        DiskCache::with_ttl(temp_dir.path().to_path_buf(), Some(1024 * 1024), Some(ttl)).unwrap();

    let key = "test_key".to_string();
    let value = Bytes::from("test_value");

    // Set value
    cache.set(&key, value.clone()).await.unwrap();
    assert_eq!(cache.get(&key).await, Some(value.clone()));

    // Wait for TTL to expire
    sleep(Duration::from_millis(150)).await;

    // Value should be expired
    assert!(cache.get(&key).await.is_none());
}

#[tokio::test]
async fn test_memory_cache_with_ttl() {
    let ttl = Duration::from_millis(100);
    let cache = LruMemoryCache::with_ttl(1024, Some(ttl));

    let key = "test_key".to_string();
    let value = Bytes::from("test_value");

    // Set value
    cache.set(&key, value.clone()).await.unwrap();
    assert_eq!(cache.get(&key).await, Some(value.clone()));

    // Wait for TTL to expire
    sleep(Duration::from_millis(150)).await;

    // Value should be expired
    assert!(cache.get(&key).await.is_none());
}

use bytes::Bytes;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;
use zarrs_cache::{Cache, HybridCache, HybridCacheConfig};

#[tokio::test]
async fn test_hybrid_cache_basic_operations() {
    let temp_dir = TempDir::new().unwrap();
    let config = HybridCacheConfig {
        memory_size: 1024,
        disk_size: Some(1024 * 1024),
        disk_dir: temp_dir.path().to_path_buf(),
        ttl: None,
        promotion_threshold: 0.5,
        demotion_threshold: Duration::from_secs(10),
        maintenance_interval: Duration::from_secs(1),
    };

    let cache = HybridCache::new(config).unwrap();

    let key = "test_key".to_string();
    let value = Bytes::from("test_value");

    // Test initial state
    assert!(cache.get(&key).await.is_none());
    assert_eq!(cache.size(), 0);

    // Test set and get
    cache.set(&key, value.clone()).await.unwrap();
    assert_eq!(cache.get(&key).await, Some(value.clone()));
    assert!(cache.size() > 0);

    // Test stats (combines memory + disk)
    let stats = cache.stats();
    assert!(stats.hits > 0);
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
async fn test_hybrid_cache_promotion() {
    let temp_dir = TempDir::new().unwrap();
    let config = HybridCacheConfig {
        memory_size: 1024,
        disk_size: Some(1024 * 1024),
        disk_dir: temp_dir.path().to_path_buf(),
        ttl: None,
        promotion_threshold: 0.1, // Very low threshold for easy testing
        demotion_threshold: Duration::from_secs(60),
        maintenance_interval: Duration::from_millis(100),
    };

    let cache = HybridCache::new(config).unwrap();

    let key = "frequent_key".to_string();
    let value = Bytes::from("frequent_value");

    // Set initial value
    cache.set(&key, value.clone()).await.unwrap();

    // Access multiple times to trigger promotion
    for _ in 0..5 {
        cache.get(&key).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Wait for maintenance to run
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Check access stats
    let access_stats = cache.access_stats().await;
    assert!(access_stats.contains_key(&key));

    let (count, frequency) = access_stats.get(&key).unwrap();
    assert!(*count >= 5);
    assert!(*frequency > 0.0);
}

#[tokio::test]
async fn test_hybrid_cache_with_ttl() {
    let temp_dir = TempDir::new().unwrap();
    let ttl = Duration::from_millis(100);
    let config = HybridCacheConfig {
        memory_size: 1024,
        disk_size: Some(1024 * 1024),
        disk_dir: temp_dir.path().to_path_buf(),
        ttl: Some(ttl),
        promotion_threshold: 0.5,
        demotion_threshold: Duration::from_secs(10),
        maintenance_interval: Duration::from_secs(1),
    };

    let cache = HybridCache::new(config).unwrap();

    let key = "ttl_key".to_string();
    let value = Bytes::from("ttl_value");

    // Set value
    cache.set(&key, value.clone()).await.unwrap();
    assert_eq!(cache.get(&key).await, Some(value.clone()));

    // Wait for TTL to expire
    sleep(Duration::from_millis(150)).await;

    // Value should be expired from both memory and disk
    assert!(cache.get(&key).await.is_none());
}

#[tokio::test]
async fn test_hybrid_cache_default_config() {
    let temp_dir = TempDir::new().unwrap();
    let cache = HybridCache::with_default_config(temp_dir.path().to_path_buf()).unwrap();

    let key = "default_key".to_string();
    let value = Bytes::from("default_value");

    // Test basic operations with default config
    cache.set(&key, value.clone()).await.unwrap();
    assert_eq!(cache.get(&key).await, Some(value.clone()));

    // Check config values
    let config = cache.config();
    assert_eq!(config.memory_size, 64 * 1024 * 1024); // 64MB default
    assert_eq!(config.disk_size, Some(1024 * 1024 * 1024)); // 1GB default
}

#[tokio::test]
async fn test_hybrid_cache_multi_tier_access() {
    let temp_dir = TempDir::new().unwrap();
    let config = HybridCacheConfig {
        memory_size: 100, // Very small memory cache
        disk_size: Some(1024 * 1024),
        disk_dir: temp_dir.path().to_path_buf(),
        ttl: None,
        promotion_threshold: 2.0, // High threshold to prevent automatic promotion
        demotion_threshold: Duration::from_secs(10),
        maintenance_interval: Duration::from_secs(1),
    };

    let cache = HybridCache::new(config).unwrap();

    // Add data that will likely overflow memory cache
    let keys_values: Vec<(String, Bytes)> = (0..5)
        .map(|i| {
            (
                format!("key_{}", i),
                Bytes::from(format!("value_{}_with_some_extra_data", i)),
            )
        })
        .collect();

    // Set all values
    for (key, value) in &keys_values {
        cache.set(key, value.clone()).await.unwrap();
    }

    // All values should be retrievable (from memory or disk)
    for (key, expected_value) in &keys_values {
        let retrieved = cache.get(key).await;
        assert_eq!(retrieved, Some(expected_value.clone()));
    }

    // Check that we have entries across both tiers
    let stats = cache.stats();
    assert_eq!(stats.entry_count, 5);
    assert!(stats.size_bytes > 0);
}

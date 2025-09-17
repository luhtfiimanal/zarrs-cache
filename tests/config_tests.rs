use std::path::PathBuf;
use std::time::Duration;
use zarrs_cache::{CacheConfig, HybridCacheConfig, MetricsConfig, PrefetchConfig};

#[test]
fn test_cache_config_default() {
    let config = CacheConfig::default();

    assert_eq!(config.max_memory_size, 100 * 1024 * 1024); // 100MB
    assert_eq!(config.disk_cache_dir, None);
    assert_eq!(config.max_disk_size, None);
    assert_eq!(config.ttl, None);
    assert!(!config.enable_compression);
    assert_eq!(config.prefetch_config, None);
}

#[test]
fn test_prefetch_config_default() {
    let config = PrefetchConfig::default();

    assert_eq!(config.neighbor_chunks, 2);
    assert_eq!(config.max_queue_size, 10);
}

#[test]
fn test_hybrid_cache_config_default() {
    let config = HybridCacheConfig::default();

    assert_eq!(config.memory_size, 64 * 1024 * 1024); // 64MB
    assert_eq!(config.disk_size, Some(1024 * 1024 * 1024)); // 1GB
    assert!(config
        .disk_dir
        .to_string_lossy()
        .contains("zarrs_hybrid_cache"));
    assert_eq!(config.ttl, None);
    assert_eq!(config.promotion_threshold, 0.1);
    assert_eq!(config.demotion_threshold, Duration::from_secs(300));
    assert_eq!(config.maintenance_interval, Duration::from_secs(60));
}

#[test]
fn test_metrics_config_default() {
    let config = MetricsConfig::default();

    assert_eq!(config.max_history_size, 1000);
    assert_eq!(config.snapshot_interval, Duration::from_secs(60));
    assert!(config.track_access_patterns);
    assert!(config.track_efficiency);
}

#[test]
fn test_cache_config_custom_values() {
    let config = CacheConfig {
        max_memory_size: 256 * 1024 * 1024, // 256MB
        disk_cache_dir: Some(PathBuf::from("/custom/cache")),
        max_disk_size: Some(2 * 1024 * 1024 * 1024), // 2GB
        ttl: Some(Duration::from_secs(3600)),        // 1 hour
        enable_compression: true,
        prefetch_config: Some(PrefetchConfig {
            neighbor_chunks: 5,
            max_queue_size: 20,
        }),
    };

    assert_eq!(config.max_memory_size, 256 * 1024 * 1024);
    assert_eq!(config.disk_cache_dir, Some(PathBuf::from("/custom/cache")));
    assert_eq!(config.max_disk_size, Some(2 * 1024 * 1024 * 1024));
    assert_eq!(config.ttl, Some(Duration::from_secs(3600)));
    assert!(config.enable_compression);
    assert!(config.prefetch_config.is_some());

    let prefetch = config.prefetch_config.unwrap();
    assert_eq!(prefetch.neighbor_chunks, 5);
    assert_eq!(prefetch.max_queue_size, 20);
}

#[test]
fn test_hybrid_cache_config_custom_values() {
    let config = HybridCacheConfig {
        memory_size: 128 * 1024 * 1024,          // 128MB
        disk_size: Some(5 * 1024 * 1024 * 1024), // 5GB
        disk_dir: PathBuf::from("/tmp/my_cache"),
        ttl: Some(Duration::from_secs(7200)), // 2 hours
        promotion_threshold: 0.5,
        demotion_threshold: Duration::from_secs(600), // 10 minutes
        maintenance_interval: Duration::from_secs(120), // 2 minutes
    };

    assert_eq!(config.memory_size, 128 * 1024 * 1024);
    assert_eq!(config.disk_size, Some(5 * 1024 * 1024 * 1024));
    assert_eq!(config.disk_dir, PathBuf::from("/tmp/my_cache"));
    assert_eq!(config.ttl, Some(Duration::from_secs(7200)));
    assert_eq!(config.promotion_threshold, 0.5);
    assert_eq!(config.demotion_threshold, Duration::from_secs(600));
    assert_eq!(config.maintenance_interval, Duration::from_secs(120));
}

#[test]
fn test_config_with_partial_defaults() {
    // Test using ..Default::default() syntax
    let config = CacheConfig {
        max_memory_size: 512 * 1024 * 1024, // Override memory size
        enable_compression: true,           // Override compression
        ..Default::default()                // Use defaults for everything else
    };

    assert_eq!(config.max_memory_size, 512 * 1024 * 1024);
    assert!(config.enable_compression);
    // These should be defaults
    assert_eq!(config.disk_cache_dir, None);
    assert_eq!(config.max_disk_size, None);
    assert_eq!(config.ttl, None);
    assert_eq!(config.prefetch_config, None);
}

#[test]
fn test_metrics_config_custom_values() {
    let config = MetricsConfig {
        max_history_size: 2000,
        snapshot_interval: Duration::from_secs(30),
        track_access_patterns: false,
        track_efficiency: false,
    };

    assert_eq!(config.max_history_size, 2000);
    assert_eq!(config.snapshot_interval, Duration::from_secs(30));
    assert!(!config.track_access_patterns);
    assert!(!config.track_efficiency);
}

#[test]
fn test_config_serialization_compatibility() {
    // Test that configs can be serialized/deserialized (important for config files)
    let original_config = CacheConfig {
        max_memory_size: 128 * 1024 * 1024,
        disk_cache_dir: Some(PathBuf::from("/test/cache")),
        max_disk_size: Some(1024 * 1024 * 1024),
        ttl: Some(Duration::from_secs(1800)),
        enable_compression: true,
        prefetch_config: Some(PrefetchConfig {
            neighbor_chunks: 3,
            max_queue_size: 15,
        }),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original_config).unwrap();

    // Deserialize back
    let deserialized_config: CacheConfig = serde_json::from_str(&json).unwrap();

    // Should match original
    assert_eq!(
        deserialized_config.max_memory_size,
        original_config.max_memory_size
    );
    assert_eq!(
        deserialized_config.disk_cache_dir,
        original_config.disk_cache_dir
    );
    assert_eq!(
        deserialized_config.max_disk_size,
        original_config.max_disk_size
    );
    assert_eq!(deserialized_config.ttl, original_config.ttl);
    assert_eq!(
        deserialized_config.enable_compression,
        original_config.enable_compression
    );

    let orig_prefetch = original_config.prefetch_config.unwrap();
    let deser_prefetch = deserialized_config.prefetch_config.unwrap();
    assert_eq!(
        deser_prefetch.neighbor_chunks,
        orig_prefetch.neighbor_chunks
    );
    assert_eq!(deser_prefetch.max_queue_size, orig_prefetch.max_queue_size);
}

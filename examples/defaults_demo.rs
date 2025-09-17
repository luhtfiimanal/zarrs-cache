use std::path::PathBuf;
use zarrs_cache::{CacheConfig, HybridCache, HybridCacheConfig, MetricsCollector, MetricsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 zarrs-cache Default Configuration Demo\n");

    // 1. HybridCacheConfig with defaults
    println!("📋 HybridCacheConfig::default():");
    let hybrid_config = HybridCacheConfig::default();
    println!(
        "  Memory size: {}MB",
        hybrid_config.memory_size / (1024 * 1024)
    );
    println!(
        "  Disk size: {}GB",
        hybrid_config.disk_size.unwrap() / (1024 * 1024 * 1024)
    );
    println!("  Disk dir: {:?}", hybrid_config.disk_dir);
    println!(
        "  Promotion threshold: {} accesses/sec",
        hybrid_config.promotion_threshold
    );
    println!(
        "  Demotion threshold: {}s",
        hybrid_config.demotion_threshold.as_secs()
    );
    println!(
        "  Maintenance interval: {}s",
        hybrid_config.maintenance_interval.as_secs()
    );
    println!();

    // 2. MetricsConfig with defaults
    println!("📊 MetricsConfig::default():");
    let metrics_config = MetricsConfig::default();
    println!(
        "  Max history size: {} snapshots",
        metrics_config.max_history_size
    );
    println!(
        "  Snapshot interval: {}s",
        metrics_config.snapshot_interval.as_secs()
    );
    println!(
        "  Track access patterns: {}",
        metrics_config.track_access_patterns
    );
    println!("  Track efficiency: {}", metrics_config.track_efficiency);
    println!();

    // 3. CacheConfig with defaults
    println!("⚙️  CacheConfig::default():");
    let cache_config = CacheConfig::default();
    println!(
        "  Max memory size: {}MB",
        cache_config.max_memory_size / (1024 * 1024)
    );
    println!("  Disk cache dir: {:?}", cache_config.disk_cache_dir);
    println!("  Max disk size: {:?}", cache_config.max_disk_size);
    println!("  TTL: {:?}", cache_config.ttl);
    println!("  Prefetch config: {:?}", cache_config.prefetch_config);
    println!();

    // 4. Using defaults with overrides
    println!("🔧 Using defaults with custom overrides:");
    let custom_config = HybridCacheConfig {
        memory_size: 128 * 1024 * 1024, // 128MB instead of default 64MB
        disk_dir: PathBuf::from("/tmp/my_custom_cache"),
        ..Default::default() // Use defaults for everything else
    };

    println!(
        "  Custom memory size: {}MB",
        custom_config.memory_size / (1024 * 1024)
    );
    println!("  Custom disk dir: {:?}", custom_config.disk_dir);
    println!(
        "  Default disk size: {}GB",
        custom_config.disk_size.unwrap() / (1024 * 1024 * 1024)
    );
    println!(
        "  Default promotion threshold: {}",
        custom_config.promotion_threshold
    );
    println!();

    // 5. Create actual cache instances with defaults
    println!("🏗️  Creating cache instances with defaults:");

    // Create hybrid cache with defaults
    let _cache = HybridCache::new(HybridCacheConfig::default())?;
    println!("  ✅ HybridCache created with default config");

    // Create metrics collector with defaults
    let _metrics = MetricsCollector::new(MetricsConfig::default());
    println!("  ✅ MetricsCollector created with default config");

    println!("\n🎉 All default configurations work perfectly!");
    println!("💡 Tip: Use `..Default::default()` to override only specific fields");

    Ok(())
}

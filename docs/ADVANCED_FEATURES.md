# Advanced Features Guide

This guide covers the advanced Phase 3 features of zarrs-cache, including hybrid caching, intelligent promotion/demotion, cache warming, and comprehensive metrics collection.

## Table of Contents

1. [Hybrid Memory+Disk Cache](#hybrid-memorydisk-cache)
2. [Cache Warming Strategies](#cache-warming-strategies)
3. [Advanced Metrics and Monitoring](#advanced-metrics-and-monitoring)
4. [Performance Optimization](#performance-optimization)
5. [Best Practices](#best-practices)

## Hybrid Memory+Disk Cache

The `HybridCache` combines fast memory storage with persistent disk storage, automatically promoting frequently accessed items to memory and demoting cold items to disk.

### Configuration

```rust
use zarrs_cache::{HybridCache, HybridCacheConfig};
use std::time::Duration;

let config = HybridCacheConfig {
    memory_size: 64 * 1024 * 1024,  // 64MB memory cache
    disk_size: Some(1024 * 1024 * 1024), // 1GB disk cache
    disk_dir: "/tmp/zarrs_cache".into(),
    ttl: Some(Duration::from_secs(3600)), // 1 hour TTL
    promotion_threshold: 0.5, // 0.5 accesses per second
    demotion_threshold: Duration::from_secs(300), // 5 minutes inactivity
    maintenance_interval: Duration::from_secs(60), // 1 minute maintenance
};

let cache = HybridCache::new(config)?;
```

### Key Features

- **Intelligent Promotion**: Items accessed frequently (above `promotion_threshold`) are promoted to memory
- **Automatic Demotion**: Items inactive for longer than `demotion_threshold` are demoted to disk
- **Background Maintenance**: Periodic maintenance tasks optimize cache efficiency
- **Access Tracking**: Monitors access patterns for promotion/demotion decisions

### Usage Example

```rust
// Store data (goes to both memory and disk)
cache.set(&"key1".to_string(), data).await?;

// Retrieve data (checks memory first, then disk)
if let Some(data) = cache.get(&"key1".to_string()).await {
    // Data found and automatically promoted if frequently accessed
}

// Get access statistics
let stats = cache.access_stats().await;
for (key, (count, frequency)) in stats {
    println!("{}: {} accesses, {:.2} freq", key, count, frequency);
}
```

## Cache Warming Strategies

Cache warming preloads data based on predictive algorithms and access patterns to improve hit rates.

### Available Strategies

#### Predictive Warming
Predicts future access based on historical patterns:

```rust
use zarrs_cache::{PredictiveWarming, WarmingStrategy};

let strategy = WarmingStrategy::Predictive(
    PredictiveWarming::new(
        5,    // Look ahead 5 keys
        0.7   // 70% confidence threshold
    )
);
```

#### Neighbor Warming
Preloads neighboring chunks in multidimensional arrays:

```rust
use zarrs_cache::{NeighborWarming, WarmingStrategy};

let strategy = WarmingStrategy::Neighbor(
    NeighborWarming::new(
        2,   // Distance of 2 in each dimension
        10   // Maximum 10 keys to warm
    )
);
```

### Cache Warmer

```rust
use zarrs_cache::{CacheWarmer, WarmingContext, TimeContext};
use std::sync::Arc;

let warmer = CacheWarmer::new(Arc::new(cache));

let context = WarmingContext {
    recent_access: vec!["array/chunk_1.1.1".to_string()],
    hit_rate: 0.8,
    available_capacity: 1024 * 1024,
    time_context: TimeContext {
        hour_of_day: 14,
        day_of_week: 2,
        is_weekend: false,
    },
};

// Warm cache using configured strategies
warmer.warm_with_strategies(&[strategy], &context).await?;
```

## Advanced Metrics and Monitoring

The `MetricsCollector` provides comprehensive performance tracking and analytics.

### Configuration

```rust
use zarrs_cache::{MetricsCollector, MetricsConfig};
use std::time::Duration;

let config = MetricsConfig {
    max_history_size: 1000,
    snapshot_interval: Duration::from_secs(10),
    track_access_patterns: true,
    track_efficiency: true,
};

let metrics = MetricsCollector::new(config);
```

### Recording Metrics

```rust
// Record cache operations
let start = std::time::Instant::now();
let result = cache.get(&key).await;
let elapsed = start.elapsed();

metrics.record_operation(&key, result.is_some(), elapsed).await;

// Record performance snapshots
let snapshot = PerformanceSnapshot {
    timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    hits: stats.hits,
    misses: stats.misses,
    hit_rate: stats.hits as f64 / (stats.hits + stats.misses).max(1) as f64,
    total_size_bytes: stats.size_bytes,
    entry_count: stats.entry_count,
    operations_per_second: 100.0,
    average_response_time_ms: 2.5,
    memory_usage_bytes: stats.size_bytes / 2,
    disk_usage_bytes: stats.size_bytes / 2,
};

metrics.record_snapshot(snapshot).await;

// Record promotion/warming effectiveness
metrics.record_promotion(true).await;  // Effective promotion
metrics.record_warming(10, 8).await;   // 10 keys warmed, 8 subsequently accessed
```

### Analytics Reports

```rust
// Generate comprehensive analytics report
let report = metrics.generate_report(Duration::from_secs(3600)).await;

println!("=== PERFORMANCE SUMMARY ===");
println!("Average hit rate: {:.2}%", report.performance_summary.average_hit_rate * 100.0);
println!("Peak hit rate: {:.2}%", report.performance_summary.peak_hit_rate * 100.0);
println!("Average response time: {:.2}ms", report.performance_summary.average_response_time_ms);

println!("=== ACCESS PATTERNS ===");
for (key, count) in &report.access_patterns.most_accessed_keys {
    println!("{}: {} accesses", key, count);
}
println!("Spatial locality score: {:.3}", report.access_patterns.spatial_locality_score);

println!("=== EFFICIENCY ANALYSIS ===");
println!("Promotion effectiveness: {:.1}%", report.efficiency_analysis.promotion_effectiveness * 100.0);
println!("Warming effectiveness: {:.1}%", report.efficiency_analysis.warming_effectiveness * 100.0);

println!("=== RECOMMENDATIONS ===");
for rec in &report.recommendations {
    println!("[{}] {}", rec.category, rec.description);
    println!("Priority: {} | Impact: {}", rec.priority, rec.expected_impact);
}
```

## Performance Optimization

### Benchmarking

Run comprehensive benchmarks to measure performance:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark groups
cargo bench -- memory_cache
cargo bench -- hybrid_cache
cargo bench -- compression
```

### Key Performance Metrics

- **Memory Cache**: ~1-5μs for get/set operations
- **Disk Cache**: ~100-500μs for get/set operations  
- **Hybrid Cache**: ~1-10μs (memory hits) / ~100-500μs (disk hits)
- **Compression**: 2-10x space savings depending on data compressibility

### Optimization Recommendations

Based on metrics analysis, the system provides automatic recommendations:

- **High Miss Rate**: Increase cache size or adjust eviction policies
- **Poor Spatial Locality**: Enable neighbor-based prefetching
- **Low Promotion Effectiveness**: Adjust promotion thresholds
- **High Latency**: Consider compression trade-offs or memory allocation

## Best Practices

### 1. Cache Configuration

```rust
// For scientific workloads with large arrays
let config = HybridCacheConfig {
    memory_size: 512 * 1024 * 1024,  // 512MB for hot data
    disk_size: Some(10 * 1024 * 1024 * 1024), // 10GB for persistence
    promotion_threshold: 0.1, // Promote after 0.1 accesses/second
    demotion_threshold: Duration::from_secs(600), // 10 minutes
    maintenance_interval: Duration::from_secs(30), // Frequent maintenance
    ..Default::default()
};
```

### 2. Metrics Collection

```rust
// Enable comprehensive tracking for production
let metrics_config = MetricsConfig {
    max_history_size: 10000,
    snapshot_interval: Duration::from_secs(5),
    track_access_patterns: true,
    track_efficiency: true,
};
```

### 3. Cache Warming

```rust
// Use multiple warming strategies
let strategies = vec![
    WarmingStrategy::Predictive(PredictiveWarming::new(10, 0.8)),
    WarmingStrategy::Neighbor(NeighborWarming::new(1, 20)),
];

// Warm during low-traffic periods
if time_context.hour_of_day < 6 || time_context.hour_of_day > 22 {
    warmer.warm_with_strategies(&strategies, &context).await?;
}
```

### 4. Monitoring and Alerting

```rust
// Regular health checks
let report = metrics.generate_report(Duration::from_secs(300)).await;

if report.performance_summary.average_hit_rate < 0.7 {
    log::warn!("Cache hit rate below 70%: {:.2}%", 
               report.performance_summary.average_hit_rate * 100.0);
}

if report.performance_summary.average_response_time_ms > 10.0 {
    log::warn!("High cache latency: {:.2}ms", 
               report.performance_summary.average_response_time_ms);
}
```

### 5. Resource Management

```rust
// Implement graceful shutdown
impl Drop for MyApplication {
    fn drop(&mut self) {
        // Generate final report
        if let Ok(report) = self.metrics.generate_report(Duration::from_secs(86400)) {
            log::info!("Final cache statistics: {:.2}% hit rate", 
                      report.performance_summary.average_hit_rate * 100.0);
        }
        
        // Clear caches
        let _ = self.cache.clear();
    }
}
```

## Integration Examples

### With Zarr Arrays

```rust
use zarrs_cache::{CachedStore, HybridCache, HybridCacheConfig};

// Create hybrid cache for Zarr data
let cache_config = HybridCacheConfig {
    memory_size: 256 * 1024 * 1024,  // 256MB
    disk_size: Some(2 * 1024 * 1024 * 1024), // 2GB
    ..Default::default()
};

let cache = HybridCache::new(cache_config)?;
let cached_store = CachedStore::new(zarr_store, cache, cache_config);

// Use with zarr arrays
let array = zarrs::array::Array::new(cached_store, "/temperature")?;
let chunk_data = array.retrieve_chunk(&[0, 0, 0]).await?;
```

### Production Deployment

```rust
// Production-ready configuration
let config = HybridCacheConfig {
    memory_size: 1024 * 1024 * 1024,  // 1GB memory
    disk_size: Some(50 * 1024 * 1024 * 1024), // 50GB disk
    disk_dir: "/var/cache/zarrs".into(),
    ttl: Some(Duration::from_secs(86400)), // 24 hour TTL
    promotion_threshold: 0.05, // Aggressive promotion
    demotion_threshold: Duration::from_secs(1800), // 30 minutes
    maintenance_interval: Duration::from_secs(60), // 1 minute
};

let cache = HybridCache::new(config)?;
let metrics = MetricsCollector::new(MetricsConfig {
    max_history_size: 50000,
    snapshot_interval: Duration::from_secs(1),
    track_access_patterns: true,
    track_efficiency: true,
});

// Background metrics reporting
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        let report = metrics.generate_report(Duration::from_secs(300)).await;
        log::info!("Cache metrics: {:.2}% hit rate, {:.2}ms avg latency",
                  report.performance_summary.average_hit_rate * 100.0,
                  report.performance_summary.average_response_time_ms);
    }
});
```

This completes the advanced features documentation. The zarrs-cache library now provides enterprise-grade caching capabilities with intelligent tiering, predictive warming, and comprehensive monitoring suitable for high-performance scientific computing workloads.

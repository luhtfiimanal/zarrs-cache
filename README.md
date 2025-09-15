# 🚀 zarrs-cache

<div align="center">

**High-Performance Intelligent Caching for Scientific Computing**

*Enterprise-grade caching layer for zarrs with hybrid tiering, predictive warming, and advanced analytics*

[![Crates.io](https://img.shields.io/crates/v/zarrs-cache.svg)](https://crates.io/crates/zarrs-cache)
[![Documentation](https://docs.rs/zarrs-cache/badge.svg)](https://docs.rs/zarrs-cache)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)
[![Build Status](https://img.shields.io/github/workflow/status/luthfiimanal/zarrs-cache/CI)](https://github.com/luthfiimanal/zarrs-cache/actions)

</div>

---

## ✨ Why zarrs-cache?

Scientific computing workloads often involve massive multidimensional arrays stored in cloud storage. **zarrs-cache** provides intelligent, high-performance caching that dramatically improves data access patterns for zarr arrays.

### 🎯 **Perfect For:**
- 🧬 **Bioinformatics**: Genomic data analysis with predictable access patterns
- 🌍 **Climate Science**: Weather/climate model data with spatial locality
- 🔬 **Medical Imaging**: Large medical datasets with temporal access patterns
- 📊 **Data Analytics**: Any workload with large, chunked array data

### ⚡ **Performance That Matters**

| Cache Type | Latency | Throughput | Use Case |
|------------|---------|------------|----------|
| **Memory** | ~1-5μs | 200K+ ops/sec | Hot data, real-time analysis |
| **Hybrid** | ~1-10μs | 100K+ ops/sec | Intelligent tiering |
| **Disk** | ~100-500μs | 2K+ ops/sec | Persistent, large datasets |

---

## 🧠 Intelligent Features

### 🔄 **Hybrid Memory+Disk Tiering**
Automatically promotes frequently accessed data to memory and demotes cold data to disk.

```rust
use zarrs_cache::{HybridCache, HybridCacheConfig};
use std::time::Duration;

let config = HybridCacheConfig {
    memory_size: 512 * 1024 * 1024,  // 512MB fast memory
    disk_size: Some(10 * 1024 * 1024 * 1024), // 10GB persistent storage
    promotion_threshold: 0.1, // Promote after 0.1 accesses/second
    demotion_threshold: Duration::from_secs(300), // Demote after 5min inactivity
    ..Default::default()
};

let cache = HybridCache::new(config)?;
```

### 🔥 **Predictive Cache Warming**
Preloads data based on access patterns and spatial locality.

```rust
use zarrs_cache::{CacheWarmer, WarmingStrategy, PredictiveWarming};

let warmer = CacheWarmer::new(cache)
    .add_strategy(WarmingStrategy::Predictive(
        PredictiveWarming::new(10, 0.8) // Warm 10 keys with 80% confidence
    ));

// Automatically warm cache based on patterns
let warmed_count = warmer.warm(data_loader).await?;
println!("Warmed {} keys proactively", warmed_count);
```

### 📊 **Advanced Analytics & Monitoring**
Real-time performance insights with actionable recommendations.

```rust
use zarrs_cache::{MetricsCollector, MetricsConfig};

let metrics = MetricsCollector::new(MetricsConfig {
    track_access_patterns: true,
    track_efficiency: true,
    ..Default::default()
});

// Get comprehensive analytics
let report = metrics.generate_report(Duration::from_hours(1)).await;
println!("Hit rate: {:.1}%", report.performance_summary.average_hit_rate * 100.0);
println!("Spatial locality: {:.1}%", report.access_patterns.spatial_locality_score * 100.0);

// Automatic optimization recommendations
for rec in report.recommendations {
    println!("💡 {}: {}", rec.category, rec.description);
}
```

### ✨ **Core Features**
- 🚀 **LRU Memory Cache**: Lightning-fast in-memory caching with automatic eviction
- 💾 **Disk Cache**: Persistent storage with TTL support and compression
- 🔄 **Hybrid Tiering**: Intelligent promotion/demotion between memory and disk
- 🗜️ **Compression**: Optional deflate compression to reduce storage overhead
- 🔥 **Cache Warming**: Predictive and neighbor-based preloading strategies
- 📊 **Advanced Metrics**: Comprehensive performance monitoring and analytics
- ⚡ **Async Support**: Full async/await support for non-blocking operations
- 🔒 **Thread-Safe**: Safe for concurrent access across multiple threads

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
zarrs-cache = "0.1.0"
```

---

## 🚀 Quick Start

### Basic Memory Caching

```rust
use zarrs_cache::{LruMemoryCache, Cache};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a 50MB memory cache
    let cache = LruMemoryCache::new(50 * 1024 * 1024);
    
    // Store scientific data
    let chunk_key = "temperature_field/chunk_100.200.50".to_string();
    let chunk_data = Bytes::from(vec![0u8; 8192]); // 8KB chunk
    
    cache.set(&chunk_key, chunk_data.clone()).await?;
    
    // Lightning-fast retrieval
    let start = std::time::Instant::now();
    if let Some(data) = cache.get(&chunk_key).await {
        println!("🚀 Retrieved {}B in {:?}", data.len(), start.elapsed());
    }
    
    // Check performance stats
    let stats = cache.stats();
    println!("📊 Hit rate: {:.1}%", 
        stats.hits as f64 / (stats.hits + stats.misses) as f64 * 100.0);
    
    Ok(())
}
```

### Production-Ready Setup

```rust
use zarrs_cache::{CachedStore, HybridCache, HybridCacheConfig, CacheConfig};
use tempfile::TempDir;
use std::time::Duration;

// Configure for production workload
let temp_dir = TempDir::new()?;
let cache_config = HybridCacheConfig {
    memory_size: 2 * 1024 * 1024 * 1024,  // 2GB memory
    disk_size: Some(100 * 1024 * 1024 * 1024), // 100GB disk
    disk_dir: temp_dir.path().to_path_buf(),
    promotion_threshold: 0.05, // Aggressive promotion
    demotion_threshold: Duration::from_secs(1800), // 30min timeout
    maintenance_interval: Duration::from_secs(60), // 1min maintenance
    ..Default::default()
};

let cache = HybridCache::new(cache_config)?;
let cached_store = CachedStore::new(
    your_storage_backend, 
    cache, 
    CacheConfig::default()
);

// Now your storage operations are automatically cached!
let data = cached_store.get_cached("my_array/chunk_0.0.0").await;
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    zarrs-cache Architecture                  │
├─────────────────────────────────────────────────────────────┤
│  🔥 Cache Warming    │  📊 Analytics      │  ⚙️ Management   │
│  • Predictive       │  • Access Patterns │  • Auto-tuning   │
│  • Neighbor-based   │  • Performance     │  • Maintenance    │
│  • Time-aware       │  • Recommendations │  • Monitoring     │
├─────────────────────────────────────────────────────────────┤
│                    🧠 Hybrid Cache Layer                     │
│  ┌─────────────────┐              ┌─────────────────────────┐│
│  │   💾 Memory     │◄────────────►│      💿 Disk Cache      ││
│  │   • LRU Cache   │  Intelligent │   • Persistent Store   ││
│  │   • ~1-5μs      │   Promotion  │   • ~100-500μs         ││
│  │   • Hot Data    │   & Demotion │   • Cold Data          ││
│  └─────────────────┘              └─────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                      🗜️ Compression Layer                    │
│              • Deflate  • Transparent  • Configurable       │
├─────────────────────────────────────────────────────────────┤
│                     🔌 Storage Interface                     │
│           • S3  • Local FS  • HTTP  • Custom Backends       │
└─────────────────────────────────────────────────────────────┘
```

### 🧩 **Core Components**

| Component | Purpose | Key Features |
|-----------|---------|--------------|
| **🔌 Cache Trait** | Generic caching interface | Async, thread-safe, extensible |
| **💾 LruMemoryCache** | Lightning-fast memory cache | LRU eviction, TTL support |
| **💿 DiskCache** | Persistent storage cache | File-based, compression, TTL |
| **🧠 HybridCache** | Intelligent tiering | Auto promotion/demotion |
| **🏪 CachedStore** | Storage wrapper | Transparent caching layer |
| **📊 MetricsCollector** | Performance monitoring | Real-time analytics |

### Cache Strategy

The library implements intelligent caching that:
- Caches chunk data (e.g., `array/0.0.0`) for fast access
- Selectively caches metadata files (`.zarray`, `.zattrs`)
- Skips caching of group metadata (`.zgroup`) to avoid unnecessary memory usage

## API Reference

### CachedStore

```rust
impl<S, C> CachedStore<S, C> {
    pub fn new(store: S, cache: C, config: CacheConfig) -> Self
    pub async fn get_cached(&self, key: &str) -> Option<Bytes>
    pub async fn set_cached(&self, key: &str, value: Bytes) -> Result<(), CacheError>
    pub async fn remove_cached(&self, key: &str) -> Result<(), CacheError>
    pub async fn clear_cache(&self) -> Result<(), CacheError>
    pub fn cache_stats(&self) -> CacheStats
}
```

### LruMemoryCache

```rust
impl LruMemoryCache {
    pub fn new(max_size_bytes: usize) -> Self
}
```

### Cache Trait

```rust
#[async_trait::async_trait]
pub trait Cache: Send + Sync + 'static {
    async fn get(&self, key: &StoreKey) -> Option<Bytes>;
    async fn set(&self, key: &StoreKey, value: Bytes) -> Result<(), CacheError>;
    async fn remove(&self, key: &StoreKey) -> Result<(), CacheError>;
    async fn clear(&self) -> Result<(), CacheError>;
    fn size(&self) -> usize;
    fn stats(&self) -> CacheStats;
}
```

## Performance

The cache is designed for high performance with:
- **Cache hit latency**: < 1ms for memory cache
- **Memory efficiency**: > 90% useful data vs metadata overhead
- **Thread-safe operations**: Lock-free atomic operations where possible
- **Automatic eviction**: LRU-based eviction when cache size limits are reached

## Testing

Run the test suite:

```bash
cargo test
```

Run benchmarks:

```bash
cargo bench
```

Run the basic example:

```bash
cargo run --example basic_usage
```

## Configuration

### CacheConfig

```rust
pub struct CacheConfig {
    pub max_memory_size: usize,           // Maximum memory cache size in bytes
    pub disk_cache_dir: Option<PathBuf>,  // Optional disk cache directory
    pub max_disk_size: Option<u64>,       // Maximum disk cache size in bytes
    pub ttl: Option<Duration>,            // Time-to-live for cached entries
    pub enable_compression: bool,         // Enable compression for cached data
    pub prefetch_config: Option<PrefetchConfig>, // Prefetch strategy configuration
}
```

## Future Enhancements

- **Disk-based caching**: Persistent cache storage
- **Hybrid caching**: Memory + disk cache combination
- **Compression**: Optional data compression for cache entries
- **TTL support**: Time-based cache expiration

## 📄 License

Licensed under the **MIT License** ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT).

This permissive license allows you to use zarrs-cache in both open source and proprietary software projects.

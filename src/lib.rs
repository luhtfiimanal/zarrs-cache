//! # 🚀 zarrs-cache
//!
//! **High-Performance Intelligent Caching for Scientific Computing**
//!
//! Enterprise-grade caching layer for zarrs with hybrid tiering, predictive warming, and advanced analytics.
//! Perfect for scientific computing workloads involving massive multidimensional arrays.
//!
//! ## ✨ Why zarrs-cache?
//!
//! Scientific computing often involves accessing large, chunked array data stored in cloud storage.
//! **zarrs-cache** provides intelligent, high-performance caching that dramatically improves
//! data access patterns for zarr arrays with:
//!
//! - 🧬 **Bioinformatics**: Genomic data analysis with predictable access patterns
//! - 🌍 **Climate Science**: Weather/climate model data with spatial locality  
//! - 🔬 **Medical Imaging**: Large medical datasets with temporal access patterns
//! - 📊 **Data Analytics**: Any workload with large, chunked array data
//!
//! ## ⚡ Performance
//!
//! | Cache Type | Latency | Throughput | Use Case |
//! |------------|---------|------------|----------|
//! | **Memory** | ~1-5μs | 200K+ ops/sec | Hot data, real-time analysis |
//! | **Hybrid** | ~1-10μs | 100K+ ops/sec | Intelligent tiering |
//! | **Disk** | ~100-500μs | 2K+ ops/sec | Persistent, large datasets |
//!
//! ## 🧠 Intelligent Features
//!
//! ### 🔄 Hybrid Memory+Disk Tiering
//! ```rust
//! use zarrs_cache::{HybridCache, HybridCacheConfig};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = HybridCacheConfig {
//!     memory_size: 512 * 1024 * 1024,  // 512MB fast memory
//!     disk_size: Some(10 * 1024 * 1024 * 1024), // 10GB persistent storage
//!     promotion_threshold: 0.1, // Promote after 0.1 accesses/second
//!     demotion_threshold: Duration::from_secs(300), // Demote after 5min inactivity
//!     ..Default::default()
//! };
//!
//! let cache = HybridCache::new(config)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 🔥 Predictive Cache Warming
//! ```rust
//! use zarrs_cache::{CacheWarmer, WarmingStrategy, PredictiveWarming};
//! use std::sync::Arc;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let cache = zarrs_cache::LruMemoryCache::new(1024);
//! let warmer = CacheWarmer::new(Arc::new(cache))
//!     .add_strategy(WarmingStrategy::Predictive(
//!         PredictiveWarming::new(10, 0.8) // Warm 10 keys with 80% confidence
//!     ));
//!
//! // Automatically warm cache based on patterns
//! let warmed_count = warmer.warm(|key| async move {
//!     // Your data loading logic here
//!     Some(bytes::Bytes::from("data"))
//! }).await?;
//! println!("Warmed {} keys proactively", warmed_count);
//! # Ok(())
//! # }
//! ```
//!
//! ### 📊 Advanced Analytics & Monitoring
//! ```rust
//! use zarrs_cache::{MetricsCollector, MetricsConfig};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let metrics = MetricsCollector::new(MetricsConfig {
//!     track_access_patterns: true,
//!     track_efficiency: true,
//!     ..Default::default()
//! });
//!
//! // Get comprehensive analytics
//! let report = metrics.generate_report(Duration::from_secs(3600)).await;
//! println!("Hit rate: {:.1}%", report.performance_summary.average_hit_rate * 100.0);
//! println!("Spatial locality: {:.1}%", report.access_patterns.spatial_locality_score * 100.0);
//!
//! // Automatic optimization recommendations
//! for rec in report.recommendations {
//!     println!("💡 {}: {}", rec.category, rec.description);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## 🚀 Quick Start
//!
//! ### Basic Memory Caching
//! ```rust
//! use zarrs_cache::{LruMemoryCache, Cache};
//! use bytes::Bytes;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a 50MB memory cache
//! let cache = LruMemoryCache::new(50 * 1024 * 1024);
//!
//! // Store scientific data
//! let chunk_key = "temperature_field/chunk_100.200.50".to_string();
//! let chunk_data = Bytes::from(vec![0u8; 8192]); // 8KB chunk
//!
//! cache.set(&chunk_key, chunk_data.clone()).await?;
//!
//! // Lightning-fast retrieval
//! let start = std::time::Instant::now();
//! if let Some(data) = cache.get(&chunk_key).await {
//!     println!("🚀 Retrieved {}B in {:?}", data.len(), start.elapsed());
//! }
//!
//! // Check performance stats
//! let stats = cache.stats();
//! println!("📊 Hit rate: {:.1}%",
//!     stats.hits as f64 / (stats.hits + stats.misses) as f64 * 100.0);
//! # Ok(())
//! # }
//! ```
//!
//! ## ✨ Core Features
//!
//! - 🚀 **LRU Memory Cache**: Lightning-fast in-memory caching with automatic eviction
//! - 💾 **Disk Cache**: Persistent storage with TTL support
//! - 🔄 **Hybrid Tiering**: Intelligent promotion/demotion between memory and disk
//! - 🔥 **Cache Warming**: Predictive and neighbor-based preloading strategies
//! - 📊 **Advanced Metrics**: Comprehensive performance monitoring and analytics
//! - ⚡ **Async Support**: Full async/await support for non-blocking operations
//! - 🔒 **Thread-Safe**: Safe for concurrent access across multiple threads

pub mod cache;
pub mod config;
pub mod error;
pub mod metrics;
pub mod prefetch;
pub mod store;
pub mod warming;

// Re-export commonly used types
pub use cache::disk::DiskCache;
pub use cache::hybrid::{HybridCache, HybridCacheConfig};
pub use cache::memory::LruMemoryCache;
pub use cache::{Cache, CacheStats};
pub use config::{CacheConfig, PrefetchConfig};
pub use error::CacheError;
pub use metrics::{CacheAnalyticsReport, MetricsCollector, MetricsConfig, PerformanceSnapshot};
pub use prefetch::{NeighborChunkPrefetch, NoPrefetch, PrefetchStrategy, SequentialPrefetch};
pub use store::cached::CachedStore;
pub use warming::{
    CacheWarmer, NeighborWarming, PredictiveWarming, TimeContext, WarmingContext, WarmingStrategy,
};

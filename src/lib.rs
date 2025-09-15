pub mod cache;
pub mod compression;
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
pub use compression::{CompressedCache, Compression, DeflateCompression, NoCompression};
pub use config::{CacheConfig, PrefetchConfig};
pub use error::CacheError;
pub use metrics::{CacheAnalyticsReport, MetricsCollector, MetricsConfig, PerformanceSnapshot};
pub use prefetch::{NeighborChunkPrefetch, NoPrefetch, PrefetchStrategy, SequentialPrefetch};
pub use store::cached::CachedStore;
pub use warming::{
    CacheWarmer, NeighborWarming, PredictiveWarming, TimeContext, WarmingContext, WarmingStrategy,
};

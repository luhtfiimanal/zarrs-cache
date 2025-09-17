use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// General cache configuration
///
/// # Default Values
/// - `max_memory_size`: 100MB
/// - `disk_cache_dir`: None (memory-only)
/// - `max_disk_size`: None (unlimited)
/// - `ttl`: None (no expiration)
/// - `prefetch_config`: None (no prefetching)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum memory cache size in bytes
    pub max_memory_size: usize,

    /// Optional disk cache directory
    pub disk_cache_dir: Option<PathBuf>,

    /// Maximum disk cache size in bytes
    pub max_disk_size: Option<u64>,

    /// Time-to-live for cached entries
    pub ttl: Option<Duration>,

    /// Prefetch strategy configuration
    pub prefetch_config: Option<PrefetchConfig>,
}

/// Configuration for prefetch strategies
///
/// # Default Values
/// - `neighbor_chunks`: 2
/// - `max_queue_size`: 10
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefetchConfig {
    /// Number of neighboring chunks to prefetch
    pub neighbor_chunks: usize,

    /// Maximum prefetch queue size
    pub max_queue_size: usize,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            neighbor_chunks: 2,
            max_queue_size: 10,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_size: 100 * 1024 * 1024, // 100MB
            disk_cache_dir: None,
            max_disk_size: None,
            ttl: None,
            prefetch_config: None,
        }
    }
}

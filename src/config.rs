use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

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

    /// Enable compression for cached data
    pub enable_compression: bool,

    /// Prefetch strategy configuration
    pub prefetch_config: Option<PrefetchConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchConfig {
    /// Number of neighboring chunks to prefetch
    pub neighbor_chunks: usize,

    /// Maximum prefetch queue size
    pub max_queue_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_size: 100 * 1024 * 1024, // 100MB
            disk_cache_dir: None,
            max_disk_size: None,
            ttl: None,
            enable_compression: false,
            prefetch_config: None,
        }
    }
}

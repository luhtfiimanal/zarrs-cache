use crate::error::CacheError;
use bytes::Bytes;

pub type StoreKey = String;

/// Core caching trait for zarr data storage
#[async_trait::async_trait]
pub trait Cache: Send + Sync + 'static {
    /// Get data from cache by key
    async fn get(&self, key: &StoreKey) -> Option<Bytes>;

    /// Store data in cache with key
    async fn set(&self, key: &StoreKey, value: Bytes) -> Result<(), CacheError>;

    /// Remove data from cache
    async fn remove(&self, key: &StoreKey) -> Result<(), CacheError>;

    /// Clear all cached data
    async fn clear(&self) -> Result<(), CacheError>;

    /// Get current cache size in bytes
    fn size(&self) -> usize;

    /// Get cache statistics
    fn stats(&self) -> CacheStats;
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size_bytes: usize,
    pub entry_count: usize,
}

impl CacheStats {
    /// Calculate cache hit rate as a ratio (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

pub mod disk;
pub mod hybrid;
pub mod memory;

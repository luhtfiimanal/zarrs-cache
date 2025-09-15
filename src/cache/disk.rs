use crate::cache::{Cache, CacheStats, StoreKey};
use crate::error::CacheError;
use bytes::Bytes;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct DiskCache {
    cache_dir: PathBuf,
    max_size_bytes: Option<u64>,
    current_size: Arc<AtomicUsize>,
    stats: Arc<CacheStatsInner>,
    ttl: Option<Duration>,
    index: Arc<RwLock<HashMap<StoreKey, CacheMetadata>>>,
}

#[derive(Clone)]
struct CacheMetadata {
    file_path: PathBuf,
    size: usize,
    created_at: Instant,
    last_accessed: Instant,
}

struct CacheStatsInner {
    hits: AtomicU64,
    misses: AtomicU64,
}

impl DiskCache {
    pub fn new(cache_dir: PathBuf, max_size_bytes: Option<u64>) -> Result<Self, CacheError> {
        Self::with_ttl(cache_dir, max_size_bytes, None)
    }

    pub fn with_ttl(
        cache_dir: PathBuf,
        max_size_bytes: Option<u64>,
        ttl: Option<Duration>,
    ) -> Result<Self, CacheError> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)?;

        let cache = Self {
            cache_dir,
            max_size_bytes,
            current_size: Arc::new(AtomicUsize::new(0)),
            stats: Arc::new(CacheStatsInner {
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
            }),
            ttl,
            index: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize by scanning existing files
        cache.initialize_from_disk()?;

        Ok(cache)
    }

    fn initialize_from_disk(&self) -> Result<(), CacheError> {
        // This would scan the cache directory and rebuild the index
        // For now, we'll start with an empty cache
        Ok(())
    }

    fn key_to_path(&self, key: &StoreKey) -> PathBuf {
        // Convert key to safe filename
        let safe_key = key.replace(['/', '\\'], "_");
        self.cache_dir.join(format!("{}.cache", safe_key))
    }

    fn is_expired(&self, metadata: &CacheMetadata) -> bool {
        if let Some(ttl) = self.ttl {
            metadata.created_at.elapsed() > ttl
        } else {
            false
        }
    }

    async fn cleanup_expired(&self) -> Result<(), CacheError> {
        if self.ttl.is_none() {
            return Ok(());
        }

        let mut index = self.index.write().await;
        let mut expired_keys = Vec::new();

        // Collect expired keys
        for (key, metadata) in index.iter() {
            if self.is_expired(metadata) {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        for key in expired_keys {
            if let Some(metadata) = index.remove(&key) {
                // Remove file
                if let Err(e) = fs::remove_file(&metadata.file_path) {
                    tracing::warn!(
                        "Failed to remove expired cache file {:?}: {}",
                        metadata.file_path,
                        e
                    );
                }
                self.current_size
                    .fetch_sub(metadata.size, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    async fn evict_if_needed(&self, incoming_size: usize) -> Result<(), CacheError> {
        let Some(max_size) = self.max_size_bytes else {
            return Ok(());
        };

        let mut index = self.index.write().await;

        while self.current_size.load(Ordering::Relaxed) + incoming_size > max_size as usize {
            // Find least recently accessed item
            let lru_key = index
                .iter()
                .min_by_key(|(_, metadata)| metadata.last_accessed)
                .map(|(key, _)| key.clone());

            if let Some(key) = lru_key {
                if let Some(metadata) = index.remove(&key) {
                    // Remove file
                    if let Err(e) = fs::remove_file(&metadata.file_path) {
                        tracing::warn!(
                            "Failed to remove cache file {:?}: {}",
                            metadata.file_path,
                            e
                        );
                    }
                    self.current_size
                        .fetch_sub(metadata.size, Ordering::Relaxed);
                } else {
                    break; // No more items to evict
                }
            } else {
                return Err(CacheError::CacheFull);
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Cache for DiskCache {
    async fn get(&self, key: &StoreKey) -> Option<Bytes> {
        // Clean up expired entries periodically
        if let Err(e) = self.cleanup_expired().await {
            tracing::warn!("Failed to cleanup expired entries: {:?}", e);
        }

        let mut index = self.index.write().await;

        if let Some(metadata) = index.get(key).cloned() {
            // Check if expired
            if self.is_expired(&metadata) {
                // Remove expired entry
                index.remove(key);
                if let Err(e) = fs::remove_file(&metadata.file_path) {
                    tracing::warn!(
                        "Failed to remove expired cache file {:?}: {}",
                        metadata.file_path,
                        e
                    );
                }
                self.current_size
                    .fetch_sub(metadata.size, Ordering::Relaxed);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            // Update last accessed time
            let mut updated_metadata = metadata.clone();
            updated_metadata.last_accessed = Instant::now();
            index.insert(key.clone(), updated_metadata);

            // Read file
            match fs::read(&metadata.file_path) {
                Ok(data) => {
                    self.stats.hits.fetch_add(1, Ordering::Relaxed);
                    Some(Bytes::from(data))
                }
                Err(e) => {
                    tracing::warn!("Failed to read cache file {:?}: {}", metadata.file_path, e);
                    // Remove invalid entry
                    index.remove(key);
                    self.current_size
                        .fetch_sub(metadata.size, Ordering::Relaxed);
                    self.stats.misses.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    async fn set(&self, key: &StoreKey, value: Bytes) -> Result<(), CacheError> {
        let value_size = value.len();

        // Check if we need to evict
        self.evict_if_needed(value_size).await?;

        let file_path = self.key_to_path(key);

        // Write to disk
        fs::write(&file_path, &value)?;

        let now = Instant::now();
        let metadata = CacheMetadata {
            file_path,
            size: value_size,
            created_at: now,
            last_accessed: now,
        };

        // Update index
        let mut index = self.index.write().await;

        // Remove old entry if it exists
        if let Some(old_metadata) = index.remove(key) {
            self.current_size
                .fetch_sub(old_metadata.size, Ordering::Relaxed);
            // Old file will be overwritten
        }

        index.insert(key.clone(), metadata);
        self.current_size.fetch_add(value_size, Ordering::Relaxed);

        Ok(())
    }

    async fn remove(&self, key: &StoreKey) -> Result<(), CacheError> {
        let mut index = self.index.write().await;

        if let Some(metadata) = index.remove(key) {
            if let Err(e) = fs::remove_file(&metadata.file_path) {
                tracing::warn!(
                    "Failed to remove cache file {:?}: {}",
                    metadata.file_path,
                    e
                );
            }
            self.current_size
                .fetch_sub(metadata.size, Ordering::Relaxed);
        }

        Ok(())
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let mut index = self.index.write().await;

        // Remove all files
        for (_, metadata) in index.drain() {
            if let Err(e) = fs::remove_file(&metadata.file_path) {
                tracing::warn!(
                    "Failed to remove cache file {:?}: {}",
                    metadata.file_path,
                    e
                );
            }
        }

        self.current_size.store(0, Ordering::Relaxed);

        Ok(())
    }

    fn size(&self) -> usize {
        self.current_size.load(Ordering::Relaxed)
    }

    fn stats(&self) -> CacheStats {
        let index_guard = futures::executor::block_on(self.index.read());

        CacheStats {
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            size_bytes: self.current_size.load(Ordering::Relaxed),
            entry_count: index_guard.len(),
        }
    }
}

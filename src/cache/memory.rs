use crate::cache::{Cache, CacheStats, StoreKey};
use crate::error::CacheError;
use bytes::Bytes;
use lru::LruCache;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct LruMemoryCache {
    inner: Arc<RwLock<LruCache<StoreKey, CacheEntry>>>,
    max_size_bytes: usize,
    current_size: Arc<AtomicUsize>,
    stats: Arc<CacheStatsInner>,
    ttl: Option<Duration>,
}

struct CacheEntry {
    data: Bytes,
    timestamp: std::time::Instant,
}

struct CacheStatsInner {
    hits: AtomicU64,
    misses: AtomicU64,
}

impl LruMemoryCache {
    pub fn new(max_size_bytes: usize) -> Self {
        Self::with_ttl(max_size_bytes, None)
    }

    pub fn with_ttl(max_size_bytes: usize, ttl: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LruCache::unbounded())),
            max_size_bytes,
            current_size: Arc::new(AtomicUsize::new(0)),
            stats: Arc::new(CacheStatsInner {
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
            }),
            ttl,
        }
    }

    fn is_expired(&self, entry: &CacheEntry) -> bool {
        if let Some(ttl) = self.ttl {
            entry.timestamp.elapsed() > ttl
        } else {
            false
        }
    }

    async fn cleanup_expired(&self) {
        if self.ttl.is_none() {
            return;
        }

        let mut cache = self.inner.write().await;
        let mut expired_keys = Vec::new();

        // Collect expired keys
        for (key, entry) in cache.iter() {
            if self.is_expired(entry) {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        for key in expired_keys {
            if let Some(entry) = cache.pop(&key) {
                self.current_size
                    .fetch_sub(entry.data.len(), Ordering::Relaxed);
            }
        }
    }

    async fn evict_if_needed(&self, incoming_size: usize) -> Result<(), CacheError> {
        let mut cache = self.inner.write().await;

        while self.current_size.load(Ordering::Relaxed) + incoming_size > self.max_size_bytes {
            if let Some((_, entry)) = cache.pop_lru() {
                self.current_size
                    .fetch_sub(entry.data.len(), Ordering::Relaxed);
            } else {
                return Err(CacheError::CacheFull);
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Cache for LruMemoryCache {
    async fn get(&self, key: &StoreKey) -> Option<Bytes> {
        // Clean up expired entries periodically
        self.cleanup_expired().await;

        let mut cache = self.inner.write().await;

        if let Some(entry) = cache.get(key) {
            // Check if entry is expired
            if self.is_expired(entry) {
                // Remove expired entry
                if let Some(expired_entry) = cache.pop(key) {
                    self.current_size
                        .fetch_sub(expired_entry.data.len(), Ordering::Relaxed);
                }
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                None
            } else {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.data.clone())
            }
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    async fn set(&self, key: &StoreKey, value: Bytes) -> Result<(), CacheError> {
        let value_size = value.len();

        self.evict_if_needed(value_size).await?;

        let entry = CacheEntry {
            data: value,
            timestamp: Instant::now(),
        };

        let mut cache = self.inner.write().await;
        cache.put(key.clone(), entry);
        self.current_size.fetch_add(value_size, Ordering::Relaxed);

        Ok(())
    }

    async fn remove(&self, key: &StoreKey) -> Result<(), CacheError> {
        let mut cache = self.inner.write().await;

        if let Some(entry) = cache.pop(key) {
            self.current_size
                .fetch_sub(entry.data.len(), Ordering::Relaxed);
        }

        Ok(())
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let mut cache = self.inner.write().await;
        cache.clear();
        self.current_size.store(0, Ordering::Relaxed);
        Ok(())
    }

    fn size(&self) -> usize {
        self.current_size.load(Ordering::Relaxed)
    }

    fn stats(&self) -> CacheStats {
        let cache_guard = futures::executor::block_on(self.inner.read());

        CacheStats {
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            size_bytes: self.current_size.load(Ordering::Relaxed),
            entry_count: cache_guard.len(),
        }
    }
}

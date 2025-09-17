use crate::cache::Cache;
use crate::config::CacheConfig;
use bytes::Bytes;
use std::sync::Arc;

/// A generic caching wrapper that can work with any storage backend
pub struct CachedStore<S, C>
where
    S: Send + Sync + 'static,
    C: Cache,
{
    inner: Arc<S>,
    cache: Arc<C>,
    config: CacheConfig,
}

impl<S, C> CachedStore<S, C>
where
    S: Send + Sync + 'static,
    C: Cache,
{
    pub fn new(store: S, cache: C, config: CacheConfig) -> Self {
        Self {
            inner: Arc::new(store),
            cache: Arc::new(cache),
            config,
        }
    }

    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }

    pub fn inner(&self) -> &Arc<S> {
        &self.inner
    }

    pub fn cache(&self) -> &Arc<C> {
        &self.cache
    }

    fn should_cache_key(&self, key: &str) -> bool {
        // Cache chunks but be selective about metadata
        !key.ends_with(".zgroup") || key.contains(".zarray") || key.contains(".zattrs")
    }

    /// Check if TTL is configured and supported
    pub fn has_ttl_support(&self) -> bool {
        self.config.ttl.is_some()
    }

    /// Check if disk caching is configured
    pub fn has_disk_cache(&self) -> bool {
        self.config.disk_cache_dir.is_some()
    }

    /// Get the cache configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get data with caching
    pub async fn get_cached(&self, key: &str) -> Option<Bytes> {
        if !self.should_cache_key(key) {
            return None;
        }

        // Check cache first
        if let Some(cached_data) = self.cache.get(&key.to_string()).await {
            tracing::debug!("Cache HIT for key: {}", key);
            return Some(cached_data);
        }

        tracing::debug!("Cache MISS for key: {}", key);
        None
    }

    /// Set data with caching
    pub async fn set_cached(
        &self,
        key: &str,
        value: Bytes,
    ) -> Result<(), crate::error::CacheError> {
        if self.should_cache_key(key) {
            self.cache.set(&key.to_string(), value).await?;
        }
        Ok(())
    }

    /// Remove data from cache
    pub async fn remove_cached(&self, key: &str) -> Result<(), crate::error::CacheError> {
        self.cache.remove(&key.to_string()).await
    }

    /// Clear all cached data
    pub async fn clear_cache(&self) -> Result<(), crate::error::CacheError> {
        self.cache.clear().await
    }
}

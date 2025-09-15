use crate::cache::disk::DiskCache;
use crate::cache::memory::LruMemoryCache;
use crate::cache::{Cache, CacheStats};
use crate::error::CacheError;
use bytes::Bytes;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Access frequency tracking for promotion/demotion decisions
#[derive(Debug, Clone)]
struct AccessInfo {
    count: u64,
    last_access: Instant,
    promoted_at: Option<Instant>,
}

impl AccessInfo {
    fn new() -> Self {
        Self {
            count: 1,
            last_access: Instant::now(),
            promoted_at: None,
        }
    }

    fn update_access(&mut self) {
        self.count += 1;
        self.last_access = Instant::now();
    }

    fn mark_promoted(&mut self) {
        self.promoted_at = Some(Instant::now());
    }

    /// Calculate access frequency (accesses per second)
    fn frequency(&self) -> f64 {
        let age = self.last_access.duration_since(
            self.promoted_at
                .unwrap_or_else(|| self.last_access - Duration::from_secs(1)),
        );
        if age.as_secs_f64() > 0.0 {
            self.count as f64 / age.as_secs_f64()
        } else {
            self.count as f64
        }
    }

    /// Check if item should be demoted based on inactivity
    fn should_demote(&self, inactivity_threshold: Duration) -> bool {
        self.last_access.elapsed() > inactivity_threshold
    }
}

/// Configuration for hybrid cache behavior
#[derive(Debug, Clone)]
pub struct HybridCacheConfig {
    /// Memory cache size in bytes
    pub memory_size: usize,
    /// Disk cache size in bytes
    pub disk_size: Option<u64>,
    /// Disk cache directory
    pub disk_dir: PathBuf,
    /// TTL for cache entries
    pub ttl: Option<Duration>,
    /// Minimum access frequency to promote to memory (accesses per second)
    pub promotion_threshold: f64,
    /// Time of inactivity before considering demotion
    pub demotion_threshold: Duration,
    /// How often to run maintenance tasks
    pub maintenance_interval: Duration,
}

impl Default for HybridCacheConfig {
    fn default() -> Self {
        Self {
            memory_size: 64 * 1024 * 1024,       // 64MB
            disk_size: Some(1024 * 1024 * 1024), // 1GB
            disk_dir: std::env::temp_dir().join("zarrs_hybrid_cache"),
            ttl: None,
            promotion_threshold: 0.1, // 0.1 accesses per second
            demotion_threshold: Duration::from_secs(300), // 5 minutes
            maintenance_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Hybrid cache that combines memory and disk storage with intelligent promotion/demotion
pub struct HybridCache {
    memory_cache: Arc<LruMemoryCache>,
    disk_cache: Arc<DiskCache>,
    access_tracker: Arc<RwLock<HashMap<String, AccessInfo>>>,
    config: HybridCacheConfig,
    last_maintenance: Arc<RwLock<Instant>>,
}

impl HybridCache {
    /// Create a new hybrid cache with the given configuration
    pub fn new(config: HybridCacheConfig) -> Result<Self, CacheError> {
        // Create memory cache
        let memory_cache = if let Some(ttl) = config.ttl {
            LruMemoryCache::with_ttl(config.memory_size, Some(ttl))
        } else {
            LruMemoryCache::new(config.memory_size)
        };

        // Create disk cache
        let disk_cache = if let Some(ttl) = config.ttl {
            DiskCache::with_ttl(config.disk_dir.clone(), config.disk_size, Some(ttl))?
        } else {
            DiskCache::new(config.disk_dir.clone(), config.disk_size)?
        };

        Ok(Self {
            memory_cache: Arc::new(memory_cache),
            disk_cache: Arc::new(disk_cache),
            access_tracker: Arc::new(RwLock::new(HashMap::new())),
            config,
            last_maintenance: Arc::new(RwLock::new(Instant::now())),
        })
    }

    /// Create a hybrid cache with default configuration
    pub fn with_default_config(disk_dir: PathBuf) -> Result<Self, CacheError> {
        let config = HybridCacheConfig {
            disk_dir,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Check if maintenance should run and execute if needed
    async fn maybe_run_maintenance(&self) -> Result<(), CacheError> {
        let mut last_maintenance = self.last_maintenance.write().await;
        if last_maintenance.elapsed() >= self.config.maintenance_interval {
            *last_maintenance = Instant::now();
            drop(last_maintenance);
            self.run_maintenance().await?;
        }
        Ok(())
    }

    /// Run maintenance tasks: promote hot items, demote cold items
    async fn run_maintenance(&self) -> Result<(), CacheError> {
        let mut access_tracker = self.access_tracker.write().await;
        let mut promotions = Vec::new();
        let mut demotions = Vec::new();

        // Analyze access patterns
        for (key, access_info) in access_tracker.iter() {
            if access_info.frequency() >= self.config.promotion_threshold {
                // Check if item is in disk cache but not in memory
                if self.memory_cache.get(key).await.is_none() {
                    if let Some(data) = self.disk_cache.get(key).await {
                        promotions.push((key.clone(), data));
                    }
                }
            } else if access_info.should_demote(self.config.demotion_threshold) {
                // Check if item is in memory cache
                if let Some(data) = self.memory_cache.get(key).await {
                    demotions.push((key.clone(), data));
                }
            }
        }

        // Execute promotions
        for (key, data) in promotions {
            if let Err(e) = self.memory_cache.set(&key, data).await {
                tracing::warn!("Failed to promote key {}: {:?}", key, e);
            } else {
                if let Some(access_info) = access_tracker.get_mut(&key) {
                    access_info.mark_promoted();
                }
                tracing::debug!("Promoted key to memory: {}", key);
            }
        }

        // Execute demotions
        for (key, data) in demotions {
            if let Err(e) = self.disk_cache.set(&key, data).await {
                tracing::warn!("Failed to demote key {}: {:?}", key, e);
            } else {
                if let Err(e) = self.memory_cache.remove(&key).await {
                    tracing::warn!("Failed to remove demoted key from memory: {:?}", e);
                }
                tracing::debug!("Demoted key to disk: {}", key);
            }
        }

        // Clean up old access tracking entries
        access_tracker.retain(|_, access_info| {
            !access_info.should_demote(self.config.demotion_threshold * 2)
        });

        Ok(())
    }

    /// Update access tracking for a key
    async fn track_access(&self, key: &String) {
        let mut access_tracker = self.access_tracker.write().await;
        match access_tracker.get_mut(key) {
            Some(access_info) => access_info.update_access(),
            None => {
                access_tracker.insert(key.to_string(), AccessInfo::new());
            }
        }
    }

    /// Get cache configuration
    pub fn config(&self) -> &HybridCacheConfig {
        &self.config
    }

    /// Get access statistics for debugging
    pub async fn access_stats(&self) -> HashMap<String, (u64, f64)> {
        let access_tracker = self.access_tracker.read().await;
        access_tracker
            .iter()
            .map(|(key, info)| (key.clone(), (info.count, info.frequency())))
            .collect()
    }
}

#[async_trait::async_trait]
impl Cache for HybridCache {
    async fn get(&self, key: &String) -> Option<Bytes> {
        // Track access
        self.track_access(key).await;

        // Try memory cache first (fastest)
        if let Some(data) = self.memory_cache.get(key).await {
            return Some(data);
        }

        // Try disk cache
        if let Some(data) = self.disk_cache.get(key).await {
            // Consider promoting frequently accessed items
            let should_promote = {
                let access_tracker = self.access_tracker.read().await;
                access_tracker
                    .get(key)
                    .map(|info| info.frequency() >= self.config.promotion_threshold)
                    .unwrap_or(false)
            };

            if should_promote {
                // Promote to memory cache
                if let Err(e) = self.memory_cache.set(key, data.clone()).await {
                    tracing::warn!("Failed to promote key {}: {:?}", key, e);
                } else {
                    let mut access_tracker = self.access_tracker.write().await;
                    if let Some(access_info) = access_tracker.get_mut(key) {
                        access_info.mark_promoted();
                    }
                }
            }

            return Some(data);
        }

        // Run maintenance if needed
        if let Err(e) = self.maybe_run_maintenance().await {
            tracing::warn!("Maintenance failed: {:?}", e);
        }

        None
    }

    async fn set(&self, key: &String, value: Bytes) -> Result<(), CacheError> {
        // Track access
        self.track_access(key).await;

        // Always store in disk cache for persistence
        self.disk_cache.set(key, value.clone()).await?;

        // Store in memory cache if it fits or if frequently accessed
        let should_cache_in_memory = {
            let access_tracker = self.access_tracker.read().await;
            access_tracker
                .get(key)
                .map(|info| info.frequency() >= self.config.promotion_threshold)
                .unwrap_or(true) // Default to caching new items in memory
        };

        if should_cache_in_memory {
            if let Err(e) = self.memory_cache.set(key, value).await {
                tracing::debug!("Could not cache in memory (likely size limit): {:?}", e);
            }
        }

        Ok(())
    }

    async fn remove(&self, key: &String) -> Result<(), CacheError> {
        // Remove from both caches
        let memory_result = self.memory_cache.remove(key).await;
        let disk_result = self.disk_cache.remove(key).await;

        // Remove from access tracking
        let mut access_tracker = self.access_tracker.write().await;
        access_tracker.remove(key);

        // Return first error if any
        memory_result.and(disk_result)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let memory_result = self.memory_cache.clear().await;
        let disk_result = self.disk_cache.clear().await;

        // Clear access tracking
        let mut access_tracker = self.access_tracker.write().await;
        access_tracker.clear();

        memory_result.and(disk_result)
    }

    fn size(&self) -> usize {
        self.memory_cache.size() + self.disk_cache.size()
    }

    fn stats(&self) -> CacheStats {
        let memory_stats = self.memory_cache.stats();
        let disk_stats = self.disk_cache.stats();

        // For hybrid cache, we need to avoid double-counting entries that exist in both tiers
        // We'll use disk_stats as the authoritative count since all entries go to disk
        CacheStats {
            hits: memory_stats.hits + disk_stats.hits,
            misses: memory_stats.misses + disk_stats.misses,
            size_bytes: memory_stats.size_bytes + disk_stats.size_bytes,
            entry_count: disk_stats.entry_count, // Use disk as authoritative count
        }
    }
}

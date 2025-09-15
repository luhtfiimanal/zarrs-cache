use crate::cache::Cache;
use crate::config::PrefetchConfig;
use crate::error::CacheError;
use bytes::Bytes;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

/// Prefetching strategy trait
#[async_trait::async_trait]
pub trait PrefetchStrategy: Send + Sync + 'static {
    /// Generate keys to prefetch based on the accessed key
    fn generate_prefetch_keys(&self, accessed_key: &str) -> Vec<String>;

    /// Execute prefetching for the given keys
    async fn prefetch<C, F, Fut>(
        &self,
        cache: &C,
        keys: Vec<String>,
        loader: F,
    ) -> Result<(), CacheError>
    where
        C: Cache,
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<Bytes>> + Send;
}

/// Neighboring chunk prefetching strategy
pub struct NeighborChunkPrefetch {
    neighbor_count: usize,
    max_queue_size: usize,
    prefetch_queue: Arc<RwLock<VecDeque<String>>>,
    #[allow(dead_code)]
    semaphore: Arc<Semaphore>,
}

impl NeighborChunkPrefetch {
    pub fn new(config: &PrefetchConfig) -> Self {
        Self {
            neighbor_count: config.neighbor_chunks,
            max_queue_size: config.max_queue_size,
            prefetch_queue: Arc::new(RwLock::new(VecDeque::new())),
            semaphore: Arc::new(Semaphore::new(config.max_queue_size)),
        }
    }

    fn parse_chunk_coordinates(&self, key: &str) -> Option<(String, Vec<i32>)> {
        // Parse zarr chunk key format: "array_name/x.y.z"
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        let array_name = parts[0].to_string();
        let coord_str = parts[1];

        let coords: Result<Vec<i32>, _> = coord_str.split('.').map(|s| s.parse::<i32>()).collect();

        coords.ok().map(|c| (array_name, c))
    }

    fn generate_neighbor_coordinates(&self, coords: &[i32]) -> Vec<Vec<i32>> {
        let mut neighbors = Vec::new();

        // Generate neighboring coordinates in each dimension
        for dim in 0..coords.len() {
            for offset in 1..=self.neighbor_count as i32 {
                // Positive direction
                let mut pos_coord = coords.to_vec();
                pos_coord[dim] += offset;
                if pos_coord[dim] >= 0 {
                    neighbors.push(pos_coord);
                }

                // Negative direction
                let mut neg_coord = coords.to_vec();
                neg_coord[dim] -= offset;
                if neg_coord[dim] >= 0 {
                    neighbors.push(neg_coord);
                }
            }
        }

        neighbors
    }
}

#[async_trait::async_trait]
impl PrefetchStrategy for NeighborChunkPrefetch {
    fn generate_prefetch_keys(&self, accessed_key: &str) -> Vec<String> {
        let Some((array_name, coords)) = self.parse_chunk_coordinates(accessed_key) else {
            return Vec::new();
        };

        let neighbor_coords = self.generate_neighbor_coordinates(&coords);

        neighbor_coords
            .into_iter()
            .map(|coord| {
                let coord_str = coord
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                format!("{}/{}", array_name, coord_str)
            })
            .collect()
    }

    async fn prefetch<C, F, Fut>(
        &self,
        cache: &C,
        keys: Vec<String>,
        loader: F,
    ) -> Result<(), CacheError>
    where
        C: Cache,
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<Bytes>> + Send,
    {
        let mut queue = self.prefetch_queue.write().await;

        // Add keys to prefetch queue
        for key in keys {
            if queue.len() >= self.max_queue_size {
                queue.pop_front(); // Remove oldest if queue is full
            }

            // Only add if not already cached
            if cache.get(&key).await.is_none() {
                queue.push_back(key);
            }
        }

        // Process prefetch queue synchronously for now
        // In a real implementation, this would use a background worker
        let keys_to_fetch: Vec<String> = queue.drain(..).take(self.max_queue_size).collect();
        drop(queue);

        for key in keys_to_fetch {
            if let Some(data) = loader(key.clone()).await {
                if let Err(e) = cache.set(&key, data).await {
                    tracing::warn!("Failed to prefetch key {}: {:?}", key, e);
                } else {
                    tracing::debug!("Prefetched key: {}", key);
                }
            }
        }

        Ok(())
    }
}

/// Sequential prefetching strategy
pub struct SequentialPrefetch {
    lookahead: usize,
    max_queue_size: usize,
}

impl SequentialPrefetch {
    pub fn new(config: &PrefetchConfig) -> Self {
        Self {
            lookahead: config.neighbor_chunks,
            max_queue_size: config.max_queue_size,
        }
    }
}

#[async_trait::async_trait]
impl PrefetchStrategy for SequentialPrefetch {
    fn generate_prefetch_keys(&self, accessed_key: &str) -> Vec<String> {
        // For sequential access patterns, predict next keys
        let parts: Vec<&str> = accessed_key.split('/').collect();
        if parts.len() != 2 {
            return Vec::new();
        }

        let array_name = parts[0];
        let coord_str = parts[1];

        let coords: Result<Vec<i32>, _> = coord_str.split('.').map(|s| s.parse::<i32>()).collect();

        let Ok(mut coords) = coords else {
            return Vec::new();
        };

        let mut prefetch_keys = Vec::new();

        // Generate sequential keys (increment last dimension)
        for i in 1..=self.lookahead {
            if let Some(last_coord) = coords.last_mut() {
                *last_coord += i as i32;
                let coord_str = coords
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                prefetch_keys.push(format!("{}/{}", array_name, coord_str));
            }
        }

        prefetch_keys
    }

    async fn prefetch<C, F, Fut>(
        &self,
        cache: &C,
        keys: Vec<String>,
        loader: F,
    ) -> Result<(), CacheError>
    where
        C: Cache,
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<Bytes>> + Send,
    {
        // Simple implementation: prefetch first N keys that aren't cached
        let mut count = 0;
        for key in keys {
            if count >= self.max_queue_size {
                break;
            }

            if cache.get(&key).await.is_none() {
                if let Some(data) = loader(key.clone()).await {
                    if let Err(e) = cache.set(&key, data).await {
                        tracing::warn!("Failed to prefetch key {}: {:?}", key, e);
                    } else {
                        tracing::debug!("Prefetched key: {}", key);
                        count += 1;
                    }
                }
            }
        }

        Ok(())
    }
}

/// No-op prefetching strategy
pub struct NoPrefetch;

#[async_trait::async_trait]
impl PrefetchStrategy for NoPrefetch {
    fn generate_prefetch_keys(&self, _accessed_key: &str) -> Vec<String> {
        Vec::new()
    }

    async fn prefetch<C, F, Fut>(
        &self,
        _cache: &C,
        _keys: Vec<String>,
        _loader: F,
    ) -> Result<(), CacheError>
    where
        C: Cache,
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<Bytes>> + Send,
    {
        Ok(())
    }
}

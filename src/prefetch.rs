use crate::cache::Cache;
use crate::config::PrefetchConfig;
use crate::error::CacheError;
use bytes::Bytes;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

/// Parse zarr chunk key into array name and coordinates
/// Format: "array_name/x.y.z" -> ("array_name", [x, y, z])
fn parse_zarr_chunk_key(key: &str) -> Option<(String, Vec<i32>)> {
    let parts: Vec<&str> = key.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let array_name = parts[0].to_string();
    let coord_str = parts[1];

    let coords: Result<Vec<i32>, _> = coord_str.split('.').map(|s| s.parse::<i32>()).collect();

    coords.ok().map(|c| (array_name, c))
}

#[cfg(test)]
mod parse_zarr_chunk_key_tests {
    use super::*;

    #[test]
    fn test_valid_chunk_keys() {
        assert_eq!(
            parse_zarr_chunk_key("array/1.2.3"),
            Some(("array".to_string(), vec![1, 2, 3]))
        );

        assert_eq!(
            parse_zarr_chunk_key("my_dataset/0.5.10"),
            Some(("my_dataset".to_string(), vec![0, 5, 10]))
        );

        assert_eq!(
            parse_zarr_chunk_key("data/42"),
            Some(("data".to_string(), vec![42]))
        );
    }

    #[test]
    fn test_invalid_chunk_keys() {
        assert_eq!(parse_zarr_chunk_key("invalid_key"), None);
        assert_eq!(parse_zarr_chunk_key("array/invalid/coords"), None);
        assert_eq!(parse_zarr_chunk_key("array/not.numbers.here"), None);
        assert_eq!(parse_zarr_chunk_key(""), None);
        assert_eq!(parse_zarr_chunk_key("array/"), None);
        // Note: "/1.2.3" is actually valid (empty array name)
        assert_eq!(
            parse_zarr_chunk_key("/1.2.3"),
            Some(("".to_string(), vec![1, 2, 3]))
        );
    }

    #[test]
    fn test_negative_coordinates() {
        assert_eq!(
            parse_zarr_chunk_key("array/-1.2.-3"),
            Some(("array".to_string(), vec![-1, 2, -3]))
        );
    }
}

/// Generate neighbor coordinates in all dimensions
/// For each dimension, generate neighbors at specified distances
fn generate_neighbor_coordinates(coords: &[i32], neighbor_count: usize) -> Vec<Vec<i32>> {
    let mut neighbors = Vec::new();

    // Generate neighboring coordinates in each dimension
    for dim in 0..coords.len() {
        for offset in 1..=neighbor_count as i32 {
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

#[cfg(test)]
mod generate_neighbor_coordinates_tests {
    use super::*;

    #[test]
    fn test_2d_single_neighbor() {
        let coords = vec![5, 10];
        let neighbors = generate_neighbor_coordinates(&coords, 1);

        assert_eq!(neighbors.len(), 4); // 2 dims * 2 directions * 1 neighbor
        assert!(neighbors.contains(&vec![6, 10])); // +1 in first dim
        assert!(neighbors.contains(&vec![4, 10])); // -1 in first dim
        assert!(neighbors.contains(&vec![5, 11])); // +1 in second dim
        assert!(neighbors.contains(&vec![5, 9])); // -1 in second dim
    }

    #[test]
    fn test_3d_multiple_neighbors() {
        let coords = vec![5, 5, 5];
        let neighbors = generate_neighbor_coordinates(&coords, 2);

        assert_eq!(neighbors.len(), 12); // 3 dims * 2 directions * 2 neighbors

        // Check some expected neighbors
        assert!(neighbors.contains(&vec![6, 5, 5])); // +1 in first dim
        assert!(neighbors.contains(&vec![7, 5, 5])); // +2 in first dim
        assert!(neighbors.contains(&vec![4, 5, 5])); // -1 in first dim
        assert!(neighbors.contains(&vec![3, 5, 5])); // -2 in first dim
    }

    #[test]
    fn test_boundary_conditions() {
        let coords = vec![0, 1];
        let neighbors = generate_neighbor_coordinates(&coords, 2);

        // Should not generate negative coordinates
        for neighbor in &neighbors {
            for &coord in neighbor {
                assert!(coord >= 0, "Generated negative coordinate: {:?}", neighbor);
            }
        }

        // Should have fewer neighbors due to boundary constraints
        assert!(neighbors.len() < 8); // Would be 8 if no boundary constraints
    }

    #[test]
    fn test_single_dimension() {
        let coords = vec![10];
        let neighbors = generate_neighbor_coordinates(&coords, 1);

        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&vec![11]));
        assert!(neighbors.contains(&vec![9]));
    }
}

/// Convert coordinates back to zarr chunk key format
fn coordinates_to_zarr_key(array_name: &str, coords: &[i32]) -> String {
    let coord_str = coords
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(".");
    format!("{}/{}", array_name, coord_str)
}

#[cfg(test)]
mod coordinates_to_zarr_key_tests {
    use super::*;

    #[test]
    fn test_coordinate_conversion() {
        assert_eq!(coordinates_to_zarr_key("array", &[1, 2, 3]), "array/1.2.3");

        assert_eq!(coordinates_to_zarr_key("dataset", &[0]), "dataset/0");

        assert_eq!(
            coordinates_to_zarr_key("data", &[10, 20, 30, 40]),
            "data/10.20.30.40"
        );
    }

    #[test]
    fn test_negative_coordinates() {
        assert_eq!(
            coordinates_to_zarr_key("array", &[-1, 2, -3]),
            "array/-1.2.-3"
        );
    }
}

/// Generate sequential chunk keys by incrementing the last dimension
fn generate_sequential_keys(array_name: &str, coords: &[i32], lookahead: usize) -> Vec<String> {
    let mut sequential_keys = Vec::new();

    if coords.is_empty() {
        return sequential_keys;
    }

    // Generate sequential keys by incrementing the last dimension
    for i in 1..=lookahead {
        let mut new_coords = coords.to_vec();
        if let Some(last_coord) = new_coords.last_mut() {
            *last_coord += i as i32;
            sequential_keys.push(coordinates_to_zarr_key(array_name, &new_coords));
        }
    }

    sequential_keys
}

#[cfg(test)]
mod generate_sequential_keys_tests {
    use super::*;

    #[test]
    fn test_sequential_generation() {
        let keys = generate_sequential_keys("array", &[5, 10, 15], 3);

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], "array/5.10.16");
        assert_eq!(keys[1], "array/5.10.17");
        assert_eq!(keys[2], "array/5.10.18");
    }

    #[test]
    fn test_single_dimension() {
        let keys = generate_sequential_keys("data", &[42], 2);

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], "data/43");
        assert_eq!(keys[1], "data/44");
    }

    #[test]
    fn test_empty_coordinates() {
        let keys = generate_sequential_keys("array", &[], 3);
        assert!(keys.is_empty());
    }

    #[test]
    fn test_zero_lookahead() {
        let keys = generate_sequential_keys("array", &[1, 2, 3], 0);
        assert!(keys.is_empty());
    }
}

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
}

#[async_trait::async_trait]
impl PrefetchStrategy for NeighborChunkPrefetch {
    fn generate_prefetch_keys(&self, accessed_key: &str) -> Vec<String> {
        let Some((array_name, coords)) = parse_zarr_chunk_key(accessed_key) else {
            return Vec::new();
        };

        let neighbor_coords = generate_neighbor_coordinates(&coords, self.neighbor_count);

        neighbor_coords
            .into_iter()
            .map(|coord| coordinates_to_zarr_key(&array_name, &coord))
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
        let Some((array_name, coords)) = parse_zarr_chunk_key(accessed_key) else {
            return Vec::new();
        };

        generate_sequential_keys(&array_name, &coords, self.lookahead)
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

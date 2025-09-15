# zarrs-cache Project Specification

## Project Overview

Create a high-performance caching layer for zarrs S3 storage to improve performance and reduce S3 API costs. The library provides transparent caching for Zarr chunks and metadata with support for both memory and disk-based caching strategies.

## Project Structure

```
zarrs-cache/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── cache/
│   │   ├── mod.rs
│   │   ├── memory.rs      # LRU memory cache implementation
│   │   ├── disk.rs        # Disk-based cache implementation  
│   │   └── hybrid.rs      # Memory + Disk hybrid cache
│   ├── store/
│   │   ├── mod.rs
│   │   └── cached.rs      # CachedStore wrapper implementation
│   ├── config.rs          # Cache configuration structures
│   ├── error.rs           # Error types and handling
│   └── metrics.rs         # Cache metrics and statistics
├── examples/
│   ├── basic_usage.rs     # Basic usage example
│   ├── s3_example.rs      # S3 integration example
│   └── benchmarks.rs      # Performance benchmarks
├── tests/
│   ├── integration_tests.rs
│   └── cache_tests.rs
├── benches/
│   └── cache_performance.rs
├── README.md
└── CHANGELOG.md
```

## Dependencies (Cargo.toml)

```toml
[package]
name = "zarrs-cache"
version = "0.1.0"
edition = "2021"
description = "High-performance caching layer for zarrs S3 storage"
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/zarrs-cache"
keywords = ["zarr", "cache", "s3", "storage", "performance"]
categories = ["caching", "filesystem"]

[dependencies]
# Core zarrs dependency
zarrs = "0.15"

# Async runtime and utilities
tokio = { version = "1.0", features = ["full"] }
tokio-util = "0.7"

# Data handling
bytes = "1.7"
serde = { version = "1.0", features = ["derive"] }

# Caching
lru = "0.12"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Filesystem operations
tempfile = "3.8"

# Metrics (optional)
metrics = { version = "0.21", optional = true }

# Compression (optional)
zstd = { version = "0.13", optional = true }

# Logging
tracing = "0.1"

[dev-dependencies]
tokio-test = "0.4"
criterion = { version = "0.5", features = ["html_reports"] }
tempfile = "3.8"

[features]
default = ["metrics"]
metrics = ["dep:metrics"]
compression = ["dep:zstd"]

[[bench]]
name = "cache_performance"
harness = false
```

## Core API Design

### 1. Cache Trait (`src/cache/mod.rs`)

```rust
use bytes::Bytes;
use std::future::Future;
use crate::error::CacheError;

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
```

### 2. Cache Configuration (`src/config.rs`)

```rust
use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};

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
```

### 3. Error Types (`src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache is full and cannot evict more entries")]
    CacheFull,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Compression error: {0}")]
    Compression(String),
    
    #[error("Invalid cache key: {0}")]
    InvalidKey(String),
}
```

### 4. Memory Cache Implementation (`src/cache/memory.rs`)

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::sync::RwLock;
use lru::LruCache;
use bytes::Bytes;
use crate::cache::{Cache, CacheStats, StoreKey};
use crate::error::CacheError;

pub struct LruMemoryCache {
    inner: Arc<RwLock<LruCache<StoreKey, CacheEntry>>>,
    max_size_bytes: usize,
    current_size: Arc<AtomicUsize>,
    stats: Arc<CacheStatsInner>,
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
        Self {
            inner: Arc::new(RwLock::new(LruCache::unbounded())),
            max_size_bytes,
            current_size: Arc::new(AtomicUsize::new(0)),
            stats: Arc::new(CacheStatsInner {
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
            }),
        }
    }
    
    async fn evict_if_needed(&self, incoming_size: usize) -> Result<(), CacheError> {
        let mut cache = self.inner.write().await;
        
        while self.current_size.load(Ordering::Relaxed) + incoming_size > self.max_size_bytes {
            if let Some((_, entry)) = cache.pop_lru() {
                self.current_size.fetch_sub(entry.data.len(), Ordering::Relaxed);
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
        let mut cache = self.inner.write().await;
        
        if let Some(entry) = cache.get(key) {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.data.clone())
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
            timestamp: std::time::Instant::now(),
        };
        
        let mut cache = self.inner.write().await;
        cache.put(key.clone(), entry);
        self.current_size.fetch_add(value_size, Ordering::Relaxed);
        
        Ok(())
    }
    
    async fn remove(&self, key: &StoreKey) -> Result<(), CacheError> {
        let mut cache = self.inner.write().await;
        
        if let Some(entry) = cache.pop(key) {
            self.current_size.fetch_sub(entry.data.len(), Ordering::Relaxed);
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
```

### 5. Cached Store Implementation (`src/store/cached.rs`)

```rust
use std::sync::Arc;
use bytes::Bytes;
use zarrs::storage::{ReadableStorageSync, WritableStorageSync, StorageError, StoreKey};
use crate::cache::Cache;
use crate::config::CacheConfig;

pub struct CachedStore<S, C> 
where
    S: ReadableStorageSync + WritableStorageSync + Send + Sync + 'static,
    C: Cache,
{
    inner: Arc<S>,
    cache: Arc<C>,
    config: CacheConfig,
}

impl<S, C> CachedStore<S, C>
where
    S: ReadableStorageSync + WritableStorageSync + Send + Sync + 'static,
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
    
    fn should_cache_key(&self, key: &StoreKey) -> bool {
        // Cache chunks but be selective about metadata
        !key.ends_with(".zgroup") || key.contains(".zarray") || key.contains(".zattrs")
    }
}

#[async_trait::async_trait]
impl<S, C> ReadableStorageSync for CachedStore<S, C>
where
    S: ReadableStorageSync + WritableStorageSync + Send + Sync + 'static,
    C: Cache,
{
    async fn get(&self, key: &StoreKey) -> Result<Option<Bytes>, StorageError> {
        if !self.should_cache_key(key) {
            return self.inner.get(key).await;
        }
        
        // Check cache first
        if let Some(cached_data) = self.cache.get(key).await {
            tracing::debug!("Cache HIT for key: {}", key);
            return Ok(Some(cached_data));
        }
        
        tracing::debug!("Cache MISS for key: {}", key);
        
        // Fetch from underlying store
        let result = self.inner.get(key).await?;
        
        // Cache successful results
        if let Some(ref data) = result {
            if let Err(e) = self.cache.set(key, data.clone()).await {
                tracing::warn!("Failed to cache key {}: {:?}", key, e);
            }
        }
        
        Ok(result)
    }
    
    async fn list(&self) -> Result<Vec<StoreKey>, StorageError> {
        self.inner.list().await
    }
    
    async fn list_prefix(&self, prefix: &str) -> Result<Vec<StoreKey>, StorageError> {
        self.inner.list_prefix(prefix).await
    }
    
    async fn list_dir(&self, prefix: &str) -> Result<Vec<StoreKey>, StorageError> {
        self.inner.list_dir(prefix).await
    }
}

#[async_trait::async_trait]
impl<S, C> WritableStorageSync for CachedStore<S, C>
where
    S: ReadableStorageSync + WritableStorageSync + Send + Sync + 'static,
    C: Cache,
{
    async fn set(&self, key: &StoreKey, value: Bytes) -> Result<(), StorageError> {
        // Write to underlying store first
        self.inner.set(key, value.clone()).await?;
        
        // Update cache if applicable
        if self.should_cache_key(key) {
            if let Err(e) = self.cache.set(key, value).await {
                tracing::warn!("Failed to update cache for key {}: {:?}", key, e);
            }
        }
        
        Ok(())
    }
    
    async fn erase(&self, key: &StoreKey) -> Result<(), StorageError> {
        // Remove from underlying store
        self.inner.erase(key).await?;
        
        // Remove from cache
        if let Err(e) = self.cache.remove(key).await {
            tracing::warn!("Failed to remove key {} from cache: {:?}", key, e);
        }
        
        Ok(())
    }
    
    async fn erase_prefix(&self, prefix: &str) -> Result<(), StorageError> {
        // This is tricky - we'd need to track what keys match the prefix
        // For now, just clear the entire cache to be safe
        self.inner.erase_prefix(prefix).await?;
        
        if let Err(e) = self.cache.clear().await {
            tracing::warn!("Failed to clear cache after prefix erase: {:?}", e);
        }
        
        Ok(())
    }
}
```

### 6. Example Usage (`examples/basic_usage.rs`)

```rust
use zarrs_cache::{CachedStore, LruMemoryCache, CacheConfig};
use zarrs::storage::{store::MemoryStore};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    // Create underlying store (replace with S3Store in real usage)
    let store = MemoryStore::new();
    
    // Create cache with 50MB limit
    let cache = LruMemoryCache::new(50 * 1024 * 1024);
    
    // Create cached store
    let config = CacheConfig::default();
    let cached_store = CachedStore::new(store, cache, config);
    
    // Example usage
    let key = "test_array/0.0.0";
    let data = Bytes::from("Hello, cached world!");
    
    // First write
    cached_store.set(key, data.clone()).await?;
    
    // First read (cache miss -> cache population)
    let result1 = cached_store.get(key).await?;
    println!("First read: {:?}", result1);
    
    // Second read (cache hit)
    let result2 = cached_store.get(key).await?;
    println!("Second read: {:?}", result2);
    
    // Print cache statistics
    let stats = cached_store.cache_stats();
    println!("Cache stats: {:?}", stats);
    
    Ok(())
}
```

## Implementation Priority

### Phase 1 (MVP):
1. Create project structure with Cargo.toml
2. Implement basic Cache trait and error types
3. Implement LruMemoryCache
4. Implement CachedStore wrapper
5. Add basic example and tests

### Phase 2 (Enhanced):
1. Add disk-based caching
2. Add cache compression
3. Add TTL support
4. Add prefetching strategies

### Phase 3 (Advanced):
1. Add hybrid memory+disk cache
2. Add cache warming
3. Add advanced metrics
4. Performance optimizations

## Testing Strategy

- Unit tests for each cache implementation
- Integration tests with mock storage backends
- Performance benchmarks comparing cached vs uncached operations
- Memory usage and leak tests

## Performance Targets

- Cache hit latency: < 1ms
- Cache miss overhead: < 10% compared to direct storage access
- Memory efficiency: > 90% useful data vs metadata overhead
- Support for datasets up to 1TB with reasonable memory usage

## Documentation Requirements

- Comprehensive API documentation with examples
- Architecture overview explaining cache strategies
- Configuration guide with best practices
- Performance tuning guide
- Migration guide from plain zarrs
# zarrs-cache

A high-performance caching layer for zarrs S3 storage to improve performance and reduce S3 API costs. The library provides transparent caching for Zarr chunks and metadata with support for memory-based caching strategies.

## Features

- **LRU Memory Cache**: Efficient memory-based caching with automatic eviction
- **Generic Storage Wrapper**: Works with any storage backend
- **Selective Caching**: Smart filtering to cache chunks while being selective about metadata
- **Cache Statistics**: Built-in metrics for cache hits, misses, and memory usage
- **Async Support**: Fully asynchronous API using tokio
- **Thread-Safe**: Safe for concurrent access across multiple threads

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
zarrs-cache = "0.1.0"
```

## Basic Usage

```rust
use zarrs_cache::{CachedStore, LruMemoryCache, CacheConfig};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create cache with 50MB limit
    let cache = LruMemoryCache::new(50 * 1024 * 1024);
    
    // Create cached store
    let config = CacheConfig::default();
    let cached_store = CachedStore::new("your_storage", cache, config);
    
    // Cache data
    let key = "array/chunk_0_0_0";
    let data = Bytes::from("chunk_data");
    cached_store.set_cached(key, data.clone()).await?;
    
    // Retrieve cached data
    if let Some(cached_data) = cached_store.get_cached(key).await {
        println!("Cache hit: {:?}", cached_data);
    }
    
    // View cache statistics
    let stats = cached_store.cache_stats();
    println!("Cache stats: {:?}", stats);
    
    Ok(())
}
```

## Architecture

### Core Components

1. **Cache Trait**: Generic caching interface that can be implemented for different storage backends
2. **LruMemoryCache**: LRU-based in-memory cache implementation
3. **CachedStore**: Generic wrapper that adds caching capabilities to any storage backend
4. **CacheConfig**: Configuration structure for cache behavior
5. **CacheError**: Error types for cache operations

### Cache Strategy

The library implements intelligent caching that:
- Caches chunk data (e.g., `array/0.0.0`) for fast access
- Selectively caches metadata files (`.zarray`, `.zattrs`)
- Skips caching of group metadata (`.zgroup`) to avoid unnecessary memory usage

## API Reference

### CachedStore

```rust
impl<S, C> CachedStore<S, C> {
    pub fn new(store: S, cache: C, config: CacheConfig) -> Self
    pub async fn get_cached(&self, key: &str) -> Option<Bytes>
    pub async fn set_cached(&self, key: &str, value: Bytes) -> Result<(), CacheError>
    pub async fn remove_cached(&self, key: &str) -> Result<(), CacheError>
    pub async fn clear_cache(&self) -> Result<(), CacheError>
    pub fn cache_stats(&self) -> CacheStats
}
```

### LruMemoryCache

```rust
impl LruMemoryCache {
    pub fn new(max_size_bytes: usize) -> Self
}
```

### Cache Trait

```rust
#[async_trait::async_trait]
pub trait Cache: Send + Sync + 'static {
    async fn get(&self, key: &StoreKey) -> Option<Bytes>;
    async fn set(&self, key: &StoreKey, value: Bytes) -> Result<(), CacheError>;
    async fn remove(&self, key: &StoreKey) -> Result<(), CacheError>;
    async fn clear(&self) -> Result<(), CacheError>;
    fn size(&self) -> usize;
    fn stats(&self) -> CacheStats;
}
```

## Performance

The cache is designed for high performance with:
- **Cache hit latency**: < 1ms for memory cache
- **Memory efficiency**: > 90% useful data vs metadata overhead
- **Thread-safe operations**: Lock-free atomic operations where possible
- **Automatic eviction**: LRU-based eviction when cache size limits are reached

## Testing

Run the test suite:

```bash
cargo test
```

Run benchmarks:

```bash
cargo bench
```

Run the basic example:

```bash
cargo run --example basic_usage
```

## Configuration

### CacheConfig

```rust
pub struct CacheConfig {
    pub max_memory_size: usize,           // Maximum memory cache size in bytes
    pub disk_cache_dir: Option<PathBuf>,  // Optional disk cache directory
    pub max_disk_size: Option<u64>,       // Maximum disk cache size in bytes
    pub ttl: Option<Duration>,            // Time-to-live for cached entries
    pub enable_compression: bool,         // Enable compression for cached data
    pub prefetch_config: Option<PrefetchConfig>, // Prefetch strategy configuration
}
```

## Future Enhancements

- **Disk-based caching**: Persistent cache storage
- **Hybrid caching**: Memory + disk cache combination
- **Compression**: Optional data compression for cache entries
- **TTL support**: Time-based cache expiration
- **Prefetching**: Intelligent chunk prefetching strategies
- **Cache warming**: Pre-populate cache with frequently accessed data

## License

This project is licensed under either of

- Apache License, Version 2.0
- MIT License

at your option.

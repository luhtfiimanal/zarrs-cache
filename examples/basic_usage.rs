use bytes::Bytes;
use zarrs_cache::{CacheConfig, CachedStore, LruMemoryCache};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create a simple string-based store for demonstration
    let store = "demo_store";

    // Create cache with 50MB limit
    let cache = LruMemoryCache::new(50 * 1024 * 1024);

    // Create cached store
    let config = CacheConfig::default();
    let cached_store = CachedStore::new(store, cache, config);

    // Example usage
    let key = "test_array/0.0.0";
    let data = Bytes::from("Hello, cached world!");

    // Cache the data
    cached_store.set_cached(key, data.clone()).await?;
    println!("Data cached for key: {}", key);

    // First read (cache hit)
    if let Some(result1) = cached_store.get_cached(key).await {
        println!(
            "First read (cache hit): {:?}",
            String::from_utf8_lossy(&result1)
        );
    } else {
        println!("First read: cache miss");
    }

    // Second read (cache hit)
    if let Some(result2) = cached_store.get_cached(key).await {
        println!(
            "Second read (cache hit): {:?}",
            String::from_utf8_lossy(&result2)
        );
    } else {
        println!("Second read: cache miss");
    }

    // Print cache statistics
    let stats = cached_store.cache_stats();
    println!("Cache stats: {:?}", stats);

    // Clear cache
    cached_store.clear_cache().await?;
    println!("Cache cleared");

    // Try to read after clearing (should be cache miss)
    if let Some(_result3) = cached_store.get_cached(key).await {
        println!("Third read: unexpected cache hit");
    } else {
        println!("Third read: cache miss (expected after clear)");
    }

    // Print final cache statistics
    let final_stats = cached_store.cache_stats();
    println!("Final cache stats: {:?}", final_stats);

    Ok(())
}

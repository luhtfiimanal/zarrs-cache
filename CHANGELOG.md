# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] - 2025-09-17

### Removed
- **BREAKING**: Removed compression module entirely (`CompressedCache`, `DeflateCompression`, `NoCompression`)
- **BREAKING**: Removed `enable_compression` field from `CacheConfig`
- **BREAKING**: Removed `has_compression()` method from `CachedStore`
- Removed `flate2` dependency from `Cargo.toml`

### Changed
- **Simplified API**: Library now focuses purely on S3 caching without redundant compression
- **Improved Performance**: Eliminated unnecessary compression/decompression overhead
- **Cleaner Documentation**: Updated all docs to remove compression references
- **Reduced Complexity**: Removed ~200 lines of compression-related code

### Rationale
- Zarr already handles compression optimally at the storage level (blosc, gzip, lz4, etc.)
- Cache-level compression was redundant and often counterproductive (double compression)
- Compressing already-compressed zarr chunks typically made data larger, not smaller
- Removal simplifies the API and eliminates a common source of confusion
- Library now has a clear, focused purpose: intelligent S3 caching for zarr data

### Migration Guide
- Remove any usage of `CompressedCache`, `DeflateCompression`, or `NoCompression`
- Remove `enable_compression: true/false` from `CacheConfig` structs
- Remove calls to `has_compression()` method
- Zarr compression should be configured at the array creation level, not cache level

## [0.1.2] - 2025-09-17

### Fixed
- **Critical Fix**: Implemented clean slate initialization for disk cache to prevent dangling cache files
- Disk cache now removes all existing cache files on startup, ensuring consistent state
- Eliminates cache corruption issues after application restarts
- Maintains S3 as the single source of truth for data integrity
- **Bug Fix**: Fixed sequential prefetch logic that was incorrectly modifying coordinates in-place
- **Bug Fix**: Improved coordinate parsing and validation in prefetch strategies

### Changed
- `DiskCache::initialize_from_disk()` now performs complete cleanup on startup
- Added comprehensive logging for cache initialization process
- Improved cache directory management with proper error handling
- **Refactored**: Broke down complex prefetch logic into smaller, testable functions
- **Improved**: Added comprehensive unit tests for prefetch coordinate generation

### Technical Details
- Cache files are now truly temporary and cleaned on each restart
- No more orphaned cache files consuming disk space
- Thread-safe cache operations remain unaffected
- Maintains backward compatibility with existing cache APIs
- Prefetch module now has 65% test coverage with inline unit tests
- Sequential key generation now correctly increments coordinates without side effects

## [0.1.1] - 2025-09-15

### Added
- High-performance caching layer for zarrs S3 storage
- LRU-based memory cache with configurable size limits
- Hybrid cache combining memory and disk storage
- Cache statistics and metrics collection
- Comprehensive test suite
- Performance benchmarks
- Complete examples and documentation

### Features
- Generic `Cache` trait for extensible caching implementations
- `LruMemoryCache` with automatic eviction and thread-safe operations
- `DiskCache` for persistent storage with compression support
- `HybridCache` combining multiple cache tiers
- `CachedStore` wrapper for any storage backend
- TTL (Time-To-Live) support for cache entries
- Cache warming and promotion strategies
- Detailed performance analytics and recommendations

### Dependencies
- Compatible with zarrs v0.21
- Async/await support with tokio
- Thread-safe operations using Arc and atomic operations
- Optional metrics collection support

## [0.1.0] - 2025-09-15

### Added
- Initial release
- Basic caching functionality
- Memory cache implementation
- Integration with zarrs storage

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

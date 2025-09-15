# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2024-12-15

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

## [0.1.0] - 2024-12-01

### Added
- Initial release
- Basic caching functionality
- Memory cache implementation
- Integration with zarrs storage

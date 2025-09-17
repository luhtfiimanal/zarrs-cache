# Benchmarking zarrs-cache with Real Data

This document explains how to run performance benchmarks using real satellite data stored in local MinIO S3.

## 🎯 **Overview**

The benchmarks test cache performance with realistic zarr chunk access patterns using actual satellite data:
- **Dataset**: Configurable satellite data in zarr format
- **S3 Storage**: Local MinIO (configurable endpoint)
- **Chunk Format**: `temperature/c/x/y/z` (configurable)

## 🚀 **Quick Start**

### **0. Setup Environment**
```bash
# Copy environment template
cp .env.example .env

# Edit .env with your MinIO credentials and test data paths
# AWS_ACCESS_KEY_ID=your_access_key
# AWS_SECRET_ACCESS_KEY=your_secret_key  
# AWS_ENDPOINT_URL=http://localhost:9000
# S3_BUCKET=your-test-bucket
# S3_ZARR_PATH=your-dataset.zarr
# ZARR_CHUNK_PREFIX=temperature/c
```

### **1. Run Integration Tests**
```bash
# Run all S3 integration tests (requires MinIO to be running)
cargo test --features s3-tests --ignored

# Run specific test
cargo test test_satellite_zarr_cache_performance --features s3-tests --ignored
```

### **2. Run Performance Benchmarks**
```bash
# Run all S3 benchmarks
cargo bench --features s3-tests

# Run specific benchmark group
cargo bench --features s3-tests satellite_cache
cargo bench --features s3-tests access_patterns
cargo bench --features s3-tests cache_sizes
```

## 📊 **Available Benchmarks**

### **Integration Tests** (`tests/integration/s3_tests.rs`)

1. **`test_satellite_zarr_cache_performance`**
   - Tests basic cache hit/miss with real chunk keys
   - Measures cache effectiveness over multiple passes
   - Validates performance expectations

2. **`test_hybrid_cache_with_satellite_data`**
   - Tests memory → disk promotion with many chunks
   - Simulates realistic dataset sizes (75 chunks)
   - Measures hybrid cache behavior

3. **`bench_cache_hit_ratio_with_real_data`**
   - Tests different access patterns:
     - Sequential access (good for prefetching)
     - Random access (challenging for cache)
     - Spatial locality (neighboring chunks)

### **Performance Benchmarks** (`benches/s3_performance.rs`)

1. **`satellite_cache`** - Cache type comparison
   - `memory_cache_satellite`: LRU memory cache performance
   - `hybrid_cache_satellite`: Memory + disk cache performance

2. **`access_patterns`** - Access pattern analysis
   - `sequential_access`: Sequential chunk reading
   - `random_access`: Random chunk access
   - `spatial_locality`: Neighboring chunk access

3. **`cache_sizes`** - Cache size impact
   - `small_cache_10mb`: High eviction scenario
   - `large_cache_100mb`: Low eviction scenario

## 🔧 **Configuration**

### **S3 Credentials**
The benchmarks automatically configure MinIO credentials:
```rust
AWS_ACCESS_KEY_ID=Bi5rYYBB873dSSjA3Nz4
AWS_SECRET_ACCESS_KEY=OYhPkhCeSFmt9bF1AkoI862hC8mWV2STxsOVPYa2
AWS_ENDPOINT_URL=http://192.168.20.24:9001
AWS_REGION=us-east-1
```

### **Test Data Structure**
```
s3://zarrs/satellite_simple.zarr/
└── brightness_temperature/
    └── c/
        ├── 1/1/1  ← Test chunks
        ├── 1/1/2
        ├── 1/2/1
        └── ...
```

## 📈 **Expected Results**

### **Performance Metrics**
- **Cache Hit Latency**: < 1ms (memory), < 10ms (disk)
- **Cache Miss Latency**: 50-200ms (S3 fetch)
- **Hit Rate**: > 80% for sequential access, > 50% for spatial locality
- **Throughput**: > 100 chunks/second for cached data

### **Sample Output**
```
🚀 Testing satellite zarr data caching...
❌ Cache MISS for brightness_temperature/c/1/1/1: 156ms
❌ Cache MISS for brightness_temperature/c/1/1/2: 143ms
📊 First pass - Hits: 0, Misses: 4
✅ Cache HIT for brightness_temperature/c/1/1/1: 0.8ms
✅ Cache HIT for brightness_temperature/c/1/1/2: 0.6ms
📊 Second pass - Hits: 4, Misses: 0
📈 Cache Stats: hits=4, misses=4, hit_rate=50.00%
```

## 🛠️ **Troubleshooting**

### **MinIO Not Running**
```bash
# Check if MinIO is accessible
curl http://192.168.20.24:9001/minio/health/live
```

### **Missing Test Data**
Ensure the satellite zarr data exists:
```bash
# Check via MinIO browser
http://192.168.20.24:9001/browser/zarrs/satellite_simple.zarr
```

### **Slow Performance**
- Check network latency to MinIO
- Verify sufficient memory for cache sizes
- Monitor disk I/O for hybrid cache tests

## 🎯 **Interpreting Results**

### **Good Performance Indicators**
- ✅ Hit rate > 70% for repeated access
- ✅ Cache hit latency < 5ms
- ✅ Memory usage stays within limits
- ✅ No cache corruption or errors

### **Performance Issues**
- ❌ Hit rate < 30% (cache too small or poor access patterns)
- ❌ High memory usage (memory leaks)
- ❌ Slow cache hits (disk I/O bottleneck)
- ❌ Frequent evictions (cache size vs. working set mismatch)

## 📝 **Adding New Benchmarks**

To add new benchmarks:

1. **Add to integration tests** for correctness
2. **Add to criterion benchmarks** for performance measurement
3. **Use realistic chunk keys** matching satellite data structure
4. **Include multiple access patterns** to test different scenarios
5. **Set appropriate timeouts** for S3 operations

Example:
```rust
#[tokio::test]
#[ignore]
async fn test_your_new_scenario() {
    setup_s3_credentials();
    // Your test here
}
```

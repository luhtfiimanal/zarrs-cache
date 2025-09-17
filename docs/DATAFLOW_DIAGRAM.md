# Zarrs-Cache Dataflow Diagram

## Visual Architecture Overview

```
                    ZARRS-CACHE DATAFLOW ARCHITECTURE
    
    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                              APPLICATION LAYER                                      │
    └─────────────────────────────────────────────────────────────────────────────────────┘
                                            │
                                            ▼
    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                               ZARRS LIBRARY                                         │
    │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
    │  │   Array     │  │  Metadata   │  │ Compression │  │   Codecs    │                 │
    │  │  Operations │  │  Handling   │  │   Engine    │  │   System    │                 │
    │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘                 │
    └─────────────────────────────────────────────────────────────────────────────────────┘
                                            │
                                            ▼ Storage Interface Calls
    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                          ZARRS-CACHE INTERCEPTION LAYER                             │
    │                                                                                     │
    │  ┌──────────────────────────────────────────────────────────────────────────────┐   │
    │  │                        CachedStore Wrapper                                   │   │
    │  │                                                                              │   │
    │  │  get(key) ──┐  set(key,val) ──┐  exists(key) ──┐  list(prefix) ──┐           │   │
    │  │             │                 │                │                 │           │   │
    │  │             ▼                 ▼                ▼                 ▼           │   │
    │  │  ┌─────────────────────────────────────────────────────────────────────┐     │   │
    │  │  │                    CACHE DECISION ENGINE                            │     │   │
    │  │  │                                                                     │     │   │
    │  │  │  • Should cache this key?                                           │     │   │
    │  │  │  • Cache hit or miss?                                               │     │   │
    │  │  │  • Eviction needed?                                                 │     │   │
    │  │  │  • Promotion/demotion logic                                         │     │   │
    │  │  └─────────────────────────────────────────────────────────────────────┘     │   │
    │  └──────────────────────────────────────────────────────────────────────────────┘   │
    └─────────────────────────────────────────────────────────────────────────────────────┘
                                            │
                                            ▼
    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                              MULTI-TIER CACHE SYSTEM                                │
    │                                                                                     │
    │  ┌─────────────────────────────────────────────────────────────────────────────┐    │
    │  │                           L1: MEMORY CACHE                                  │    │
    │  │                                                                             │    │
    │  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │    │
    │  │  │    Chunk    │ │  Metadata   │ │   Access    │ │     LRU     │            │    │
    │  │  │    Data     │ │    Cache    │ │  Statistics │ │   Eviction  │            │    │
    │  │  │             │ │             │ │             │ │             │            │    │
    │  │  │ Key: /arr/  │ │ Key: .zarr  │ │ Hit Count   │ │ Timestamp   │            │    │
    │  │  │ chunks/0.0  │ │ Value: JSON │ │ Access Time │ │ Ordering    │            │    │
    │  │  │ Value: Data │ │ Size: 256B  │ │ Frequency   │ │ Size Limit  │            │    │
    │  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘            │    │
    │  └─────────────────────────────────────────────────────────────────────────────┘    │
    │                                     │                                               │
    │                                     ▼ Cache Miss / Overflow                         │
    │  ┌─────────────────────────────────────────────────────────────────────────────┐    │
    │  │                           L2: DISK CACHE                                    │    │
    │  │                                                                             │    │
    │  │  File System Layout:                                                        │    │
    │  │  /cache_dir/                                                                │    │
    │  │  ├── chunks/                                                                │    │
    │  │  │   ├── hash_abc123.cache  ← chunk data                                    │    │
    │  │  │   ├── hash_def456.cache                                                  │    │
    │  │  │   └── ...                                                                │    │
    │  │  ├── metadata/                                                              │    │
    │  │  │   ├── hash_abc123.meta   ← Access stats, TTL, size                       │    │
    │  │  │   └── hash_def456.meta                                                   │    │
    │  │  └── index.db               ← Cache index and analytics                     │    │
    │  │                                                                             │    │
    │  │  Features:                                                                  │    │
    │  │  • TTL expiration                                                           │    │
    │  │  • Size-based eviction                                                      │    │
    │  └─────────────────────────────────────────────────────────────────────────────┘    │
    └─────────────────────────────────────────────────────────────────────────────────────┘
                                            │
                                            ▼ Cache Miss: Fetch from Origin
    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                             ORIGINAL STORAGE BACKEND                                │
    │                                                                                     │
    │  ┌─────────────────────────────────────────────────────────────────────────────┐    │
    │  │                              S3 STORAGE                                     │    │
    │  │                                                                             │    │
    │  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │    │
    │  │  │   Bucket    │ │   Network   │ │    REST     │ │  Object     │            │    │
    │  │  │  Storage    │ │   Latency   │ │     API     │ │   Metadata  │            │    │
    │  │  │             │ │             │ │             │ │             │            │    │
    │  │  │ Objects:    │ │ 50-200ms    │ │ GET/PUT/    │ │ Size, ETag  │            │    │
    │  │  │ /arr/chunks │ │ per request │ │ DELETE/LIST │ │ Modified    │            │    │
    │  │  │ /arr/.zarray│ │             │ │             │ │ Content-Type│            │    │
    │  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘            │    │
    │  └─────────────────────────────────────────────────────────────────────────────┘    │
    └─────────────────────────────────────────────────────────────────────────────────────┘

                                    DETAILED OPERATION FLOWS

    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                                READ OPERATION                                       │
    │                                                                                     │
    │  Application Request                                                                │
    │         │                                                                           │
    │         ▼                                                                           │
    │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐           │
    │  │    zarrs    │───▶│ CachedStore │───▶│   Memory    │───▶│    Disk     │           │
    │  │ array.get() │    │   .get()    │    │   Cache     │    │   Cache     │           │
    │  └─────────────┘    └─────────────┘    │             │    │             │           │
    │         ▲                              │  ┌───────┐  │    │  ┌───────┐  │           │
    │         │                              │  │ HIT?  │  │    │  │ HIT?  │  │           │
    │         │                              │  └───┬───┘  │    │  └───┬───┘  │           │
    │         │                              │      │      │    │      │      │           │
    │         │                              │      ▼      │    │      ▼      │           │
    │         │                              │  ┌───────┐  │    │  ┌───────┐  │           │
    │         │                              │  │Return │  │    │  │Load to│  │           │
    │         │                              │  │ Data  │  │    │  │Memory │  │           │
    │         │                              │  └───────┘  │    │  │Return │  │           │
    │         │                              └─────────────┘    │  └───────┘  │           │
    │         │                                     ▲           └─────────────┘           │
    │         │                                     │                  ▲                  │
    │         │                                     │                  │                  │
    │         │                                     └──────────────────┘                  │
    │         │                                                        │                  │
    │         │                                                        ▼                  │
    │         │                               ┌─────────────────────────────────────┐     │
    │         │                               │            CACHE MISS               │     │
    │         │                               │                                     │     │
    │         │                               │  1. Fetch from S3                   │     │
    │         │                               │  2. Store in Memory Cache           │     │
    │         │                               │  3. Store in Disk Cache             │     │
    │         │                               │  4. Update Statistics               │     │
    │         │                               │  5. Return Data                     │     │
    │         └───────────────────────────────┤                                     │     │
    │                                         └─────────────────────────────────────┘     │
    └─────────────────────────────────────────────────────────────────────────────────────┘

    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                               WRITE OPERATION                                       │
    │                                                                                     │
    │  Application Request                                                                │
    │         │                                                                           │
    │         ▼                                                                           │
    │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                              │
    │  │    zarrs    │───▶│ CachedStore │───▶│ S3 Storage  │                              │
    │  │ array.set() │    │   .set()    │    │  (Primary)  │                              │
    │  └─────────────┘    └─────────────┘    └─────────────┘                              │
    │         ▲                                      │                                    │
    │         │                                      ▼                                    │
    │         │                               ┌─────────────┐                             │
    │         │                               │ Write       │                             │
    │         │                               │ Successful? │                             │
    │         │                               └──────┬──────┘                             │
    │         │                                      │                                    │
    │         │                                      ▼                                    │
    │         │                               ┌─────────────────────────────────────┐     │
    │         │                               │        UPDATE CACHES                │     │
    │         │                               │                                     │     │
    │         │                               │  1. Update Memory Cache             │     │
    │         │                               │  2. Update Disk Cache               │     │
    │         │                               │  3. Update Access Statistics        │     │
    │         │                               │  4. Trigger Cache Maintenance       │     │
    │         └───────────────────────────────┤                                     │     │
    │                                         └─────────────────────────────────────┘     │
    └─────────────────────────────────────────────────────────────────────────────────────┘

                                      CACHE KEY MAPPING

    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                              ZARRS PATH → CACHE KEY                                 │
    │                                                                                     │
    │  Zarrs Request Path              Cache Key                    Storage Location      │
    │  ═══════════════════════════════════════════════════════════════════════════════════│
    │                                                                                     │
    │  /weather/temp/chunks/0.0.0  →  weather_temp_chunks_0_0_0   Memory + Disk           │
    │  /weather/temp/chunks/1.2.3  →  weather_temp_chunks_1_2_3   Memory + Disk           │
    │  /weather/temp/.zarray       →  weather_temp_zarray         Memory (High Priority)  │
    │  /weather/temp/.zattrs       →  weather_temp_zattrs         Memory (High Priority)  │
    │  /weather/.zgroup            →  weather_zgroup              Memory (High Priority)  │
    │                                                                                     │
    │  Key Transformation Rules:                                                          │
    │  • Replace '/' with '_'                                                             │
    │  • Replace '.' with '_'                                                             │
    │  • Hash long keys (SHA256)                                                          │
    │  • Prefix with cache version                                                        │
    │                                                                                     │
    │  Example: "/weather/temperature/chunks/10.5.2"                                      │
    │  → "v1_weather_temperature_chunks_10_5_2"                                           │
    │  → Hash: "v1_a1b2c3d4e5f6..."  (if key too long)                                    │
    └─────────────────────────────────────────────────────────────────────────────────────┘

                                    PERFORMANCE METRICS

    ┌─────────────────────────────────────────────────────────────────────────────────────┐
    │                            LATENCY COMPARISON                                       │
    │                                                                                     │
    │  Operation Type          │ Without Cache │ Memory Hit │ Disk Hit │ S3 Direct        │
    │  ═══════════════════════════════════════════════════════════════════════════════════│
    │  Small chunk (1KB)       │    50-200ms   │   0.1ms   │   1-5ms  │   50-200ms        │
    │  Medium chunk (100KB)    │   100-500ms   │   0.5ms   │  10-50ms │  100-500ms        │
    │  Large chunk (10MB)      │   500-2000ms  │   50ms    │ 100-500ms│  500-2000ms       │
    │  Metadata (.zarray)      │    30-100ms   │  0.05ms   │   0.5ms  │   30-100ms        │
    │  Directory listing       │   100-300ms   │   0.1ms   │   2-10ms │  100-300ms        │
    │                                                                                     │
    │  Cache Hit Rates (Typical):                                                         │
    │  • Scientific workflows: 85-95%                                                     │
    │  • Visualization: 90-99%                                                            │
    │  • Iterative analysis: 70-90%                                                       │
    │  • Random access: 30-60%                                                            │
    └─────────────────────────────────────────────────────────────────────────────────────┘
```

## Key Interception Points

### 1. **Storage Interface Wrapper**
The `CachedStore` implements all zarrs storage traits (`ReadableStorage`, `WritableStorage`, `ListableStorage`) and wraps the original S3 store, intercepting every storage operation.

### 2. **Method-Level Interception**
- **`get(key)`** → Check cache hierarchy, fetch from S3 if miss
- **`set(key, value)`** → Write to S3, then update caches  
- **`exists(key)`** → Check cache first, then S3
- **`list(prefix)`** → Cache directory listings
- **`delete(key)`** → Remove from S3 and invalidate caches

### 3. **Chunk Storage Strategy**
- **Chunks** stored as binary data
- **Metadata** stored as JSON with high priority
- **Keys** transformed to filesystem-safe names
- **TTL** applied based on access patterns

### 4. **Cache Promotion/Demotion**
- Frequently accessed data promoted to memory
- Cold data demoted to disk only
- Unused data evicted completely
- Analytics drive automatic optimization

This architecture provides transparent caching with 50-2000x performance improvements for cached data while maintaining full compatibility with existing zarrs applications.

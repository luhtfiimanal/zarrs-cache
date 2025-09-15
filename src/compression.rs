use crate::error::CacheError;
use bytes::Bytes;

/// Compression trait for cache data
pub trait Compression: Send + Sync + 'static {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError>;
}

/// No-op compression (passthrough)
#[derive(Debug, Clone)]
pub struct NoCompression;

impl Compression for NoCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        Ok(data.to_vec())
    }
}

/// Simple deflate compression using flate2
#[derive(Debug, Clone)]
pub struct DeflateCompression {
    level: u32,
}

impl DeflateCompression {
    pub fn new() -> Self {
        Self { level: 6 } // Default compression level
    }

    pub fn with_level(level: u32) -> Self {
        Self {
            level: level.min(9),
        }
    }
}

impl Default for DeflateCompression {
    fn default() -> Self {
        Self::new()
    }
}

impl Compression for DeflateCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression as FlateCompression;
        use std::io::Write;

        let mut encoder = DeflateEncoder::new(Vec::new(), FlateCompression::new(self.level));
        encoder
            .write_all(data)
            .map_err(|e| CacheError::Compression(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| CacheError::Compression(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        use flate2::read::DeflateDecoder;
        use std::io::Read;

        let mut decoder = DeflateDecoder::new(data);
        let mut result = Vec::new();
        decoder
            .read_to_end(&mut result)
            .map_err(|e| CacheError::Compression(e.to_string()))?;
        Ok(result)
    }
}

/// Compressed cache wrapper
pub struct CompressedCache<C, Comp> {
    inner: C,
    compression: Comp,
}

impl<C, Comp> CompressedCache<C, Comp>
where
    C: crate::cache::Cache,
    Comp: Compression,
{
    pub fn new(cache: C, compression: Comp) -> Self {
        Self {
            inner: cache,
            compression,
        }
    }
}

#[async_trait::async_trait]
impl<C, Comp> crate::cache::Cache for CompressedCache<C, Comp>
where
    C: crate::cache::Cache,
    Comp: Compression,
{
    async fn get(&self, key: &crate::cache::StoreKey) -> Option<Bytes> {
        if let Some(compressed_data) = self.inner.get(key).await {
            match self.compression.decompress(&compressed_data) {
                Ok(decompressed) => Some(Bytes::from(decompressed)),
                Err(e) => {
                    tracing::warn!("Failed to decompress cache entry for key {}: {:?}", key, e);
                    None
                }
            }
        } else {
            None
        }
    }

    async fn set(&self, key: &crate::cache::StoreKey, value: Bytes) -> Result<(), CacheError> {
        match self.compression.compress(&value) {
            Ok(compressed) => self.inner.set(key, Bytes::from(compressed)).await,
            Err(e) => {
                tracing::warn!("Failed to compress cache entry for key {}: {:?}", key, e);
                // Fall back to storing uncompressed
                self.inner.set(key, value).await
            }
        }
    }

    async fn remove(&self, key: &crate::cache::StoreKey) -> Result<(), CacheError> {
        self.inner.remove(key).await
    }

    async fn clear(&self) -> Result<(), CacheError> {
        self.inner.clear().await
    }

    fn size(&self) -> usize {
        self.inner.size()
    }

    fn stats(&self) -> crate::cache::CacheStats {
        self.inner.stats()
    }
}

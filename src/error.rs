use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache is full and cannot evict more entries")]
    CacheFull,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid cache key: {0}")]
    InvalidKey(String),
}

use crate::cache::Cache;
use crate::error::CacheError;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache warming strategy enum to avoid trait object issues
#[derive(Debug)]
pub enum WarmingStrategy {
    Predictive(PredictiveWarming),
    Neighbor(NeighborWarming),
}

impl WarmingStrategy {
    /// Generate keys to warm based on access patterns or predictions
    pub async fn generate_warming_keys(&self, context: &WarmingContext) -> Vec<String> {
        match self {
            WarmingStrategy::Predictive(strategy) => strategy.generate_warming_keys(context).await,
            WarmingStrategy::Neighbor(strategy) => strategy.generate_warming_keys(context).await,
        }
    }

    /// Execute cache warming for the given keys
    pub async fn warm_cache<C, F, Fut>(
        &self,
        cache: &C,
        keys: Vec<String>,
        loader: F,
    ) -> Result<usize, CacheError>
    where
        C: Cache,
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<Bytes>> + Send,
    {
        match self {
            WarmingStrategy::Predictive(strategy) => strategy.warm_cache(cache, keys, loader).await,
            WarmingStrategy::Neighbor(strategy) => strategy.warm_cache(cache, keys, loader).await,
        }
    }
}

/// Context information for cache warming decisions
#[derive(Debug, Clone)]
pub struct WarmingContext {
    /// Recently accessed keys with their access counts
    pub recent_access: HashMap<String, u64>,
    /// Current cache hit rate
    pub hit_rate: f64,
    /// Available cache capacity (bytes)
    pub available_capacity: usize,
    /// Time-based patterns (hour of day, day of week, etc.)
    pub time_context: TimeContext,
}

#[derive(Debug, Clone)]
pub struct TimeContext {
    pub hour_of_day: u8,
    pub day_of_week: u8,
    pub is_weekend: bool,
}

impl TimeContext {
    pub fn current() -> Self {
        use chrono::{Datelike, Timelike, Utc};
        let now = Utc::now();

        Self {
            hour_of_day: now.hour() as u8,
            day_of_week: now.weekday().num_days_from_monday() as u8,
            is_weekend: now.weekday().num_days_from_monday() >= 5,
        }
    }
}

/// Predictive warming based on access patterns
#[derive(Debug)]
pub struct PredictiveWarming {
    /// Historical access patterns
    access_history: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    /// Maximum keys to warm in one operation
    max_warm_keys: usize,
    /// Minimum access frequency to consider for warming
    min_frequency: f64,
}

impl PredictiveWarming {
    pub fn new(max_warm_keys: usize, min_frequency: f64) -> Self {
        Self {
            access_history: Arc::new(RwLock::new(HashMap::new())),
            max_warm_keys,
            min_frequency,
        }
    }

    /// Record access for pattern learning
    pub async fn record_access(&self, key: &str) {
        let mut history = self.access_history.write().await;
        let entry = history.entry(key.to_string()).or_insert_with(Vec::new);

        // Record timestamp (simplified as incrementing counter)
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        entry.push(timestamp);

        // Keep only recent history (last 1000 accesses)
        if entry.len() > 1000 {
            entry.drain(0..entry.len() - 1000);
        }
    }

    /// Predict next likely accessed keys based on patterns
    async fn predict_next_keys(&self, context: &WarmingContext) -> Vec<String> {
        let history = self.access_history.read().await;
        let mut predictions = Vec::new();

        for (key, accesses) in history.iter() {
            if accesses.len() < 2 {
                continue;
            }

            // Calculate access frequency
            let time_span = accesses.last().unwrap() - accesses.first().unwrap();
            if time_span == 0 {
                continue;
            }

            let frequency = accesses.len() as f64 / time_span as f64;

            if frequency >= self.min_frequency {
                // Check if this key fits current time patterns
                if self.matches_time_pattern(key, &context.time_context) {
                    predictions.push(key.clone());
                }
            }
        }

        // Sort by recent access frequency
        predictions.sort_by(|a, b| {
            let a_count = context.recent_access.get(a).unwrap_or(&0);
            let b_count = context.recent_access.get(b).unwrap_or(&0);
            b_count.cmp(a_count)
        });

        predictions.truncate(self.max_warm_keys);
        predictions
    }

    /// Check if key matches time-based access patterns
    fn matches_time_pattern(&self, _key: &str, _time_context: &TimeContext) -> bool {
        // Simplified implementation - in practice, this would analyze historical
        // access patterns for time-based correlations
        true
    }

    /// Generate warming keys based on predictions
    pub async fn generate_warming_keys(&self, context: &WarmingContext) -> Vec<String> {
        self.predict_next_keys(context).await
    }

    /// Execute cache warming
    pub async fn warm_cache<C, F, Fut>(
        &self,
        cache: &C,
        keys: Vec<String>,
        loader: F,
    ) -> Result<usize, CacheError>
    where
        C: Cache,
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<Bytes>> + Send,
    {
        let mut warmed_count = 0;

        for key in keys {
            // Skip if already cached
            if cache.get(&key).await.is_some() {
                continue;
            }

            // Load and cache the data
            if let Some(data) = loader(key.clone()).await {
                cache.set(&key, data).await?;
                warmed_count += 1;
                tracing::debug!("Warmed cache key: {}", key);
            }
        }

        Ok(warmed_count)
    }
}

/// Neighboring keys warming strategy
#[derive(Debug)]
pub struct NeighborWarming {
    neighbor_distance: usize,
    max_warm_keys: usize,
}

impl NeighborWarming {
    pub fn new(neighbor_distance: usize, max_warm_keys: usize) -> Self {
        Self {
            neighbor_distance,
            max_warm_keys,
        }
    }

    /// Generate neighboring chunk keys
    fn generate_neighbors(&self, key: &str) -> Vec<String> {
        // Parse zarr chunk key format: "array_name/x.y.z"
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 2 {
            return Vec::new();
        }

        let array_name = parts[0];
        let coord_str = parts[1];

        let coords: Result<Vec<i32>, _> = coord_str.split('.').map(|s| s.parse::<i32>()).collect();

        let Ok(coords) = coords else {
            return Vec::new();
        };

        let mut neighbors = Vec::new();

        // Generate neighbors in each dimension
        for dim in 0..coords.len() {
            for offset in 1..=self.neighbor_distance as i32 {
                // Positive direction
                let mut pos_coord = coords.clone();
                pos_coord[dim] += offset;
                if pos_coord[dim] >= 0 {
                    let coord_str = pos_coord
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(".");
                    neighbors.push(format!("{}/{}", array_name, coord_str));
                }

                // Negative direction
                let mut neg_coord = coords.clone();
                neg_coord[dim] -= offset;
                if neg_coord[dim] >= 0 {
                    let coord_str = neg_coord
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(".");
                    neighbors.push(format!("{}/{}", array_name, coord_str));
                }
            }
        }

        neighbors.truncate(self.max_warm_keys);
        neighbors
    }
}

impl NeighborWarming {
    /// Generate warming keys based on neighbors
    pub async fn generate_warming_keys(&self, context: &WarmingContext) -> Vec<String> {
        let mut all_neighbors = Vec::new();

        // Generate neighbors for recently accessed keys
        for key in context.recent_access.keys() {
            let neighbors = self.generate_neighbors(key);
            all_neighbors.extend(neighbors);
        }

        // Remove duplicates and limit
        all_neighbors.sort();
        all_neighbors.dedup();
        all_neighbors.truncate(self.max_warm_keys);

        all_neighbors
    }

    /// Execute cache warming for neighbor keys
    pub async fn warm_cache<C, F, Fut>(
        &self,
        cache: &C,
        keys: Vec<String>,
        loader: F,
    ) -> Result<usize, CacheError>
    where
        C: Cache,
        F: Fn(String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<Bytes>> + Send,
    {
        let mut warmed_count = 0;

        for key in keys {
            // Skip if already cached
            if cache.get(&key).await.is_some() {
                continue;
            }

            // Load and cache the data
            if let Some(data) = loader(key.clone()).await {
                cache.set(&key, data).await?;
                warmed_count += 1;
                tracing::debug!("Warmed neighbor key: {}", key);
            }
        }

        Ok(warmed_count)
    }
}

/// Cache warmer that coordinates warming strategies
pub struct CacheWarmer<C: Cache> {
    cache: Arc<C>,
    strategies: Vec<WarmingStrategy>,
    access_tracker: Arc<RwLock<HashMap<String, u64>>>,
}

impl<C: Cache> CacheWarmer<C> {
    pub fn new(cache: Arc<C>) -> Self {
        Self {
            cache,
            strategies: Vec::new(),
            access_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a warming strategy
    pub fn add_strategy(mut self, strategy: WarmingStrategy) -> Self {
        self.strategies.push(strategy);
        self
    }

    /// Record access for warming decisions
    pub async fn record_access(&self, key: &str) {
        let mut tracker = self.access_tracker.write().await;
        *tracker.entry(key.to_string()).or_insert(0) += 1;
    }

    /// Execute cache warming using all configured strategies
    pub async fn warm<F, Fut>(&self, loader: F) -> Result<usize, CacheError>
    where
        F: Fn(String) -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = Option<Bytes>> + Send,
    {
        let context = self.build_warming_context().await;
        let mut total_warmed = 0;

        for strategy in &self.strategies {
            let keys = strategy.generate_warming_keys(&context).await;
            if !keys.is_empty() {
                let warmed = strategy
                    .warm_cache(&*self.cache, keys, loader.clone())
                    .await?;
                total_warmed += warmed;
            }
        }

        Ok(total_warmed)
    }

    /// Build warming context from current state
    async fn build_warming_context(&self) -> WarmingContext {
        let recent_access = self.access_tracker.read().await.clone();
        let stats = self.cache.stats();

        let hit_rate = if stats.hits + stats.misses > 0 {
            stats.hits as f64 / (stats.hits + stats.misses) as f64
        } else {
            0.0
        };

        // Estimate available capacity (simplified)
        let available_capacity = (1024_usize * 1024 * 100).saturating_sub(stats.size_bytes);

        WarmingContext {
            recent_access,
            hit_rate,
            available_capacity,
            time_context: TimeContext::current(),
        }
    }

    /// Clear access tracking history
    pub async fn clear_access_history(&self) {
        let mut tracker = self.access_tracker.write().await;
        tracker.clear();
    }
}

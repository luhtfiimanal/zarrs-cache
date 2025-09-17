use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Advanced metrics collector for cache performance monitoring
#[derive(Debug)]
pub struct MetricsCollector {
    /// Performance metrics over time
    performance_history: Arc<RwLock<VecDeque<PerformanceSnapshot>>>,
    /// Access pattern analysis
    access_patterns: Arc<RwLock<AccessPatternAnalyzer>>,
    /// Cache efficiency metrics
    efficiency_tracker: Arc<RwLock<EfficiencyTracker>>,
    /// Configuration for metrics collection
    config: MetricsConfig,
}

/// Configuration for metrics collection
///
/// # Default Values
/// - `max_history_size`: 1000 snapshots
/// - `snapshot_interval`: 60 seconds
/// - `track_access_patterns`: true
/// - `track_efficiency`: true
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Maximum number of performance snapshots to keep
    pub max_history_size: usize,
    /// Interval between automatic snapshots
    pub snapshot_interval: Duration,
    /// Enable detailed access pattern tracking
    pub track_access_patterns: bool,
    /// Enable cache efficiency analysis
    pub track_efficiency: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            max_history_size: 1000,
            snapshot_interval: Duration::from_secs(60),
            track_access_patterns: true,
            track_efficiency: true,
        }
    }
}

/// Point-in-time performance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub timestamp: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_size_bytes: usize,
    pub entry_count: usize,
    pub operations_per_second: f64,
    pub average_response_time_ms: f64,
    pub memory_usage_bytes: usize,
    pub disk_usage_bytes: usize,
}

/// Access pattern analysis data
#[derive(Debug)]
struct AccessPatternAnalyzer {
    /// Key access frequencies
    key_frequencies: HashMap<String, KeyAccessInfo>,
    /// Temporal access patterns
    temporal_patterns: VecDeque<TemporalAccess>,
    /// Spatial locality analysis (for zarr chunks)
    spatial_locality: SpatialLocalityTracker,
}

#[derive(Debug, Clone)]
struct KeyAccessInfo {
    total_accesses: u64,
    last_access: Instant,
    access_intervals: VecDeque<Duration>,
    cache_hits: u64,
    cache_misses: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TemporalAccess {
    timestamp: Instant,
    key: String,
    was_hit: bool,
    response_time: Duration,
}

#[derive(Debug)]
struct SpatialLocalityTracker {
    /// Track chunk coordinate access patterns
    chunk_accesses: HashMap<ChunkCoordinate, u64>,
    /// Recent access sequence for locality analysis
    recent_sequence: VecDeque<ChunkCoordinate>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ChunkCoordinate {
    array_name: String,
    coordinates: Vec<i32>,
}

/// Cache efficiency tracking
#[derive(Debug)]
struct EfficiencyTracker {
    /// Promotion/demotion effectiveness
    promotion_stats: PromotionStats,
    /// Cache warming effectiveness
    warming_stats: WarmingStats,
    /// Resource utilization
    resource_utilization: ResourceUtilization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionStats {
    pub promotions_executed: u64,
    pub promotions_effective: u64,
    pub demotions_executed: u64,
    pub demotions_effective: u64,
    pub promotion_accuracy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmingStats {
    pub warming_operations: u64,
    pub keys_warmed: u64,
    pub warming_hit_rate: f64,
    pub warming_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    pub memory_utilization: f64,
    pub disk_utilization: f64,
    pub cpu_time_ms: u64,
    pub io_operations: u64,
}

/// Comprehensive cache analytics report
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheAnalyticsReport {
    pub generated_at: u64,
    pub time_range: Duration,
    pub performance_summary: PerformanceSummary,
    pub access_patterns: AccessPatternSummary,
    pub efficiency_analysis: EfficiencyAnalysis,
    pub recommendations: Vec<OptimizationRecommendation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub average_hit_rate: f64,
    pub peak_hit_rate: f64,
    pub average_response_time_ms: f64,
    pub throughput_ops_per_second: f64,
    pub cache_size_trend: String, // "increasing", "decreasing", "stable"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessPatternSummary {
    pub most_accessed_keys: Vec<(String, u64)>,
    pub temporal_hotspots: Vec<String>, // Time periods with high activity
    pub spatial_locality_score: f64,
    pub access_distribution: String, // "uniform", "skewed", "clustered"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EfficiencyAnalysis {
    pub promotion_effectiveness: f64,
    pub warming_effectiveness: f64,
    pub resource_efficiency: f64,
    pub bottleneck_analysis: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub category: String,
    pub priority: String, // "high", "medium", "low"
    pub description: String,
    pub expected_impact: String,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            performance_history: Arc::new(RwLock::new(VecDeque::new())),
            access_patterns: Arc::new(RwLock::new(AccessPatternAnalyzer::new())),
            efficiency_tracker: Arc::new(RwLock::new(EfficiencyTracker::new())),
            config,
        }
    }

    /// Record a cache operation for metrics
    pub async fn record_operation(&self, key: &str, was_hit: bool, response_time: Duration) {
        if self.config.track_access_patterns {
            let mut patterns = self.access_patterns.write().await;
            patterns.record_access(key, was_hit, response_time);
        }
    }

    /// Record a performance snapshot
    pub async fn record_snapshot(&self, snapshot: PerformanceSnapshot) {
        let mut history = self.performance_history.write().await;
        history.push_back(snapshot);

        // Maintain history size limit
        while history.len() > self.config.max_history_size {
            history.pop_front();
        }
    }

    /// Record cache promotion/demotion event
    pub async fn record_promotion(&self, was_effective: bool) {
        if self.config.track_efficiency {
            let mut efficiency = self.efficiency_tracker.write().await;
            efficiency.promotion_stats.promotions_executed += 1;
            if was_effective {
                efficiency.promotion_stats.promotions_effective += 1;
            }
            efficiency.update_promotion_accuracy();
        }
    }

    /// Record cache warming event
    pub async fn record_warming(&self, keys_warmed: u64, subsequent_hits: u64) {
        if self.config.track_efficiency {
            let mut efficiency = self.efficiency_tracker.write().await;
            efficiency.warming_stats.warming_operations += 1;
            efficiency.warming_stats.keys_warmed += keys_warmed;

            if keys_warmed > 0 {
                let hit_rate = subsequent_hits as f64 / keys_warmed as f64;
                efficiency.warming_stats.warming_hit_rate =
                    (efficiency.warming_stats.warming_hit_rate + hit_rate) / 2.0;
            }
        }
    }

    /// Generate comprehensive analytics report
    pub async fn generate_report(&self, time_range: Duration) -> CacheAnalyticsReport {
        let history = self.performance_history.read().await;
        let patterns = self.access_patterns.read().await;
        let efficiency = self.efficiency_tracker.read().await;

        let performance_summary = self.analyze_performance(&history, time_range);
        let access_patterns_summary = patterns.analyze_patterns();
        let efficiency_analysis = efficiency.analyze_efficiency();
        let recommendations = self.generate_recommendations(
            &performance_summary,
            &access_patterns_summary,
            &efficiency_analysis,
        );

        CacheAnalyticsReport {
            generated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            time_range,
            performance_summary,
            access_patterns: access_patterns_summary,
            efficiency_analysis,
            recommendations,
        }
    }

    /// Get current performance metrics
    pub async fn current_metrics(&self) -> Option<PerformanceSnapshot> {
        let history = self.performance_history.read().await;
        history.back().cloned()
    }

    /// Get access pattern statistics
    pub async fn access_statistics(&self) -> HashMap<String, (u64, f64)> {
        let patterns = self.access_patterns.read().await;
        patterns.get_access_statistics()
    }

    fn analyze_performance(
        &self,
        history: &VecDeque<PerformanceSnapshot>,
        _time_range: Duration,
    ) -> PerformanceSummary {
        if history.is_empty() {
            return PerformanceSummary {
                average_hit_rate: 0.0,
                peak_hit_rate: 0.0,
                average_response_time_ms: 0.0,
                throughput_ops_per_second: 0.0,
                cache_size_trend: "unknown".to_string(),
            };
        }

        let hit_rates: Vec<f64> = history.iter().map(|s| s.hit_rate).collect();
        let response_times: Vec<f64> = history.iter().map(|s| s.average_response_time_ms).collect();
        let throughputs: Vec<f64> = history.iter().map(|s| s.operations_per_second).collect();

        let average_hit_rate = hit_rates.iter().sum::<f64>() / hit_rates.len() as f64;
        let peak_hit_rate = hit_rates.iter().fold(0.0f64, |a, &b| a.max(b));
        let average_response_time =
            response_times.iter().sum::<f64>() / response_times.len() as f64;
        let average_throughput = throughputs.iter().sum::<f64>() / throughputs.len() as f64;

        // Analyze cache size trend
        let cache_size_trend = if history.len() >= 2 {
            let first_size = history.front().unwrap().total_size_bytes;
            let last_size = history.back().unwrap().total_size_bytes;
            let change_ratio = last_size as f64 / first_size as f64;

            if change_ratio > 1.1 {
                "increasing".to_string()
            } else if change_ratio < 0.9 {
                "decreasing".to_string()
            } else {
                "stable".to_string()
            }
        } else {
            "unknown".to_string()
        };

        PerformanceSummary {
            average_hit_rate,
            peak_hit_rate,
            average_response_time_ms: average_response_time,
            throughput_ops_per_second: average_throughput,
            cache_size_trend,
        }
    }

    fn generate_recommendations(
        &self,
        performance: &PerformanceSummary,
        access_patterns: &AccessPatternSummary,
        efficiency: &EfficiencyAnalysis,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // Hit rate recommendations
        if performance.average_hit_rate < 0.8 {
            recommendations.push(OptimizationRecommendation {
                category: "Performance".to_string(),
                priority: "high".to_string(),
                description: "Hit rate is below 80%. Consider increasing cache size or improving warming strategies.".to_string(),
                expected_impact: "20-40% performance improvement".to_string(),
            });
        }

        // Response time recommendations
        if performance.average_response_time_ms > 10.0 {
            recommendations.push(OptimizationRecommendation {
                category: "Latency".to_string(),
                priority: "medium".to_string(),
                description: "Average response time is high. Consider optimizing cache lookup algorithms or reducing serialization overhead.".to_string(),
                expected_impact: "30-50% latency reduction".to_string(),
            });
        }

        // Access pattern recommendations
        if access_patterns.spatial_locality_score < 0.5 {
            recommendations.push(OptimizationRecommendation {
                category: "Access Patterns".to_string(),
                priority: "medium".to_string(),
                description: "Low spatial locality detected. Consider implementing more aggressive prefetching for neighboring chunks.".to_string(),
                expected_impact: "15-25% hit rate improvement".to_string(),
            });
        }

        // Efficiency recommendations
        if efficiency.warming_effectiveness < 0.6 {
            recommendations.push(OptimizationRecommendation {
                category: "Cache Warming".to_string(),
                priority: "low".to_string(),
                description:
                    "Cache warming effectiveness is low. Review warming strategies and thresholds."
                        .to_string(),
                expected_impact: "10-20% cache efficiency improvement".to_string(),
            });
        }

        recommendations
    }
}

impl AccessPatternAnalyzer {
    fn new() -> Self {
        Self {
            key_frequencies: HashMap::new(),
            temporal_patterns: VecDeque::new(),
            spatial_locality: SpatialLocalityTracker::new(),
        }
    }

    fn record_access(&mut self, key: &str, was_hit: bool, response_time: Duration) {
        // Update key frequency info
        let key_info = self
            .key_frequencies
            .entry(key.to_string())
            .or_insert_with(|| KeyAccessInfo {
                total_accesses: 0,
                last_access: Instant::now(),
                access_intervals: VecDeque::new(),
                cache_hits: 0,
                cache_misses: 0,
            });

        let now = Instant::now();
        if key_info.total_accesses > 0 {
            let interval = now.duration_since(key_info.last_access);
            key_info.access_intervals.push_back(interval);
            if key_info.access_intervals.len() > 100 {
                key_info.access_intervals.pop_front();
            }
        }

        key_info.total_accesses += 1;
        key_info.last_access = now;

        if was_hit {
            key_info.cache_hits += 1;
        } else {
            key_info.cache_misses += 1;
        }

        // Record temporal pattern
        self.temporal_patterns.push_back(TemporalAccess {
            timestamp: now,
            key: key.to_string(),
            was_hit,
            response_time,
        });

        // Maintain temporal history size
        if self.temporal_patterns.len() > 10000 {
            self.temporal_patterns.pop_front();
        }

        // Update spatial locality if it's a chunk key
        self.spatial_locality.record_chunk_access(key);
    }

    fn analyze_patterns(&self) -> AccessPatternSummary {
        let mut most_accessed: Vec<(String, u64)> = self
            .key_frequencies
            .iter()
            .map(|(k, v)| (k.clone(), v.total_accesses))
            .collect();
        most_accessed.sort_by(|a, b| b.1.cmp(&a.1));
        most_accessed.truncate(10);

        let spatial_locality_score = self.spatial_locality.calculate_locality_score();

        AccessPatternSummary {
            most_accessed_keys: most_accessed,
            temporal_hotspots: vec![], // Simplified for now
            spatial_locality_score,
            access_distribution: "mixed".to_string(), // Simplified analysis
        }
    }

    fn get_access_statistics(&self) -> HashMap<String, (u64, f64)> {
        self.key_frequencies
            .iter()
            .map(|(key, info)| {
                let hit_rate = if info.total_accesses > 0 {
                    info.cache_hits as f64 / info.total_accesses as f64
                } else {
                    0.0
                };
                (key.clone(), (info.total_accesses, hit_rate))
            })
            .collect()
    }
}

impl SpatialLocalityTracker {
    fn new() -> Self {
        Self {
            chunk_accesses: HashMap::new(),
            recent_sequence: VecDeque::new(),
        }
    }

    fn record_chunk_access(&mut self, key: &str) {
        if let Some(coord) = self.parse_chunk_coordinate(key) {
            *self.chunk_accesses.entry(coord.clone()).or_insert(0) += 1;
            self.recent_sequence.push_back(coord);

            if self.recent_sequence.len() > 1000 {
                self.recent_sequence.pop_front();
            }
        }
    }

    fn parse_chunk_coordinate(&self, key: &str) -> Option<ChunkCoordinate> {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        let array_name = parts[0].to_string();
        let coord_part = parts[1];

        if let Some(chunk_part) = coord_part.strip_prefix("chunk_") {
            let coords: Result<Vec<i32>, _> =
                chunk_part.split('.').map(|s| s.parse::<i32>()).collect();

            coords.ok().map(|c| ChunkCoordinate {
                array_name,
                coordinates: c,
            })
        } else {
            None
        }
    }

    fn calculate_locality_score(&self) -> f64 {
        if self.recent_sequence.len() < 2 {
            return 0.0;
        }

        let mut locality_events = 0;
        let mut total_transitions = 0;

        let sequence_vec: Vec<_> = self.recent_sequence.iter().collect();
        for window in sequence_vec.windows(2) {
            if let (Some(a), Some(b)) = (window.first(), window.get(1)) {
                total_transitions += 1;
                if self.are_neighbors(a, b) {
                    locality_events += 1;
                }
            }
        }

        if total_transitions > 0 {
            locality_events as f64 / total_transitions as f64
        } else {
            0.0
        }
    }

    fn are_neighbors(&self, a: &ChunkCoordinate, b: &ChunkCoordinate) -> bool {
        if a.array_name != b.array_name || a.coordinates.len() != b.coordinates.len() {
            return false;
        }

        let mut diff_count = 0;
        for (coord_a, coord_b) in a.coordinates.iter().zip(b.coordinates.iter()) {
            let diff = (coord_a - coord_b).abs();
            if diff > 1 {
                return false;
            }
            if diff == 1 {
                diff_count += 1;
            }
        }

        diff_count == 1 // Exactly one dimension differs by 1
    }
}

impl EfficiencyTracker {
    fn new() -> Self {
        Self {
            promotion_stats: PromotionStats {
                promotions_executed: 0,
                promotions_effective: 0,
                demotions_executed: 0,
                demotions_effective: 0,
                promotion_accuracy: 0.0,
            },
            warming_stats: WarmingStats {
                warming_operations: 0,
                keys_warmed: 0,
                warming_hit_rate: 0.0,
                warming_efficiency: 0.0,
            },
            resource_utilization: ResourceUtilization {
                memory_utilization: 0.0,
                disk_utilization: 0.0,
                cpu_time_ms: 0,
                io_operations: 0,
            },
        }
    }

    fn update_promotion_accuracy(&mut self) {
        if self.promotion_stats.promotions_executed > 0 {
            self.promotion_stats.promotion_accuracy = self.promotion_stats.promotions_effective
                as f64
                / self.promotion_stats.promotions_executed as f64;
        }
    }

    fn analyze_efficiency(&self) -> EfficiencyAnalysis {
        let promotion_effectiveness = self.promotion_stats.promotion_accuracy;
        let warming_effectiveness = self.warming_stats.warming_hit_rate;
        let resource_efficiency = (self.resource_utilization.memory_utilization
            + self.resource_utilization.disk_utilization)
            / 2.0;

        let mut bottlenecks = Vec::new();
        if promotion_effectiveness < 0.7 {
            bottlenecks.push("Low promotion accuracy".to_string());
        }
        if warming_effectiveness < 0.6 {
            bottlenecks.push("Ineffective cache warming".to_string());
        }
        if resource_efficiency > 0.9 {
            bottlenecks.push("High resource utilization".to_string());
        }

        EfficiencyAnalysis {
            promotion_effectiveness,
            warming_effectiveness,
            resource_efficiency,
            bottleneck_analysis: bottlenecks,
        }
    }
}

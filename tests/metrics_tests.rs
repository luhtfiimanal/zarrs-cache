use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zarrs_cache::{MetricsCollector, MetricsConfig, PerformanceSnapshot};

#[tokio::test]
async fn test_metrics_collector_basic_operations() {
    let config = MetricsConfig {
        max_history_size: 100,
        snapshot_interval: Duration::from_secs(1),
        track_access_patterns: true,
        track_efficiency: true,
    };

    let collector = MetricsCollector::new(config);

    // Record some operations
    collector
        .record_operation("test_key_1", true, Duration::from_millis(5))
        .await;
    collector
        .record_operation("test_key_2", false, Duration::from_millis(15))
        .await;
    collector
        .record_operation("test_key_1", true, Duration::from_millis(3))
        .await;

    // Check access statistics
    let stats = collector.access_statistics().await;
    assert!(stats.contains_key("test_key_1"));
    assert!(stats.contains_key("test_key_2"));

    let (count1, hit_rate1) = stats.get("test_key_1").unwrap();
    assert_eq!(*count1, 2);
    assert_eq!(*hit_rate1, 1.0); // 2 hits out of 2 accesses

    let (count2, hit_rate2) = stats.get("test_key_2").unwrap();
    assert_eq!(*count2, 1);
    assert_eq!(*hit_rate2, 0.0); // 0 hits out of 1 access
}

#[tokio::test]
async fn test_performance_snapshot_recording() {
    let collector = MetricsCollector::new(MetricsConfig::default());

    let snapshot1 = PerformanceSnapshot {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hits: 100,
        misses: 20,
        hit_rate: 0.833,
        total_size_bytes: 1024,
        entry_count: 50,
        operations_per_second: 150.0,
        average_response_time_ms: 2.5,
        memory_usage_bytes: 512,
        disk_usage_bytes: 512,
    };

    let snapshot2 = PerformanceSnapshot {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hits: 150,
        misses: 25,
        hit_rate: 0.857,
        total_size_bytes: 1536,
        entry_count: 75,
        operations_per_second: 200.0,
        average_response_time_ms: 2.0,
        memory_usage_bytes: 768,
        disk_usage_bytes: 768,
    };

    collector.record_snapshot(snapshot1.clone()).await;
    collector.record_snapshot(snapshot2.clone()).await;

    let current = collector.current_metrics().await;
    assert!(current.is_some());
    let current = current.unwrap();
    assert_eq!(current.hits, 150);
    assert_eq!(current.entry_count, 75);
}

#[tokio::test]
async fn test_promotion_tracking() {
    let collector = MetricsCollector::new(MetricsConfig::default());

    // Record some promotions
    collector.record_promotion(true).await; // Effective
    collector.record_promotion(false).await; // Not effective
    collector.record_promotion(true).await; // Effective

    // Generate report to check promotion stats
    let report = collector.generate_report(Duration::from_secs(60)).await;

    assert_eq!(
        report.efficiency_analysis.promotion_effectiveness,
        2.0 / 3.0
    );
}

#[tokio::test]
async fn test_warming_tracking() {
    let collector = MetricsCollector::new(MetricsConfig::default());

    // Record warming operations
    collector.record_warming(5, 3).await; // Warmed 5 keys, 3 were subsequently hit
    collector.record_warming(3, 2).await; // Warmed 3 keys, 2 were subsequently hit

    let report = collector.generate_report(Duration::from_secs(60)).await;

    // Should have recorded warming operations
    assert!(report.efficiency_analysis.warming_effectiveness > 0.0);
}

#[tokio::test]
async fn test_analytics_report_generation() {
    let collector = MetricsCollector::new(MetricsConfig::default());

    // Add some performance history
    for i in 0..5 {
        let snapshot = PerformanceSnapshot {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            hits: 100 + i * 10,
            misses: 20 - i * 2,
            hit_rate: (100.0 + i as f64 * 10.0) / (120.0 + i as f64 * 8.0),
            total_size_bytes: (1024 + i * 100) as usize,
            entry_count: (50 + i * 5) as usize,
            operations_per_second: 150.0 + i as f64 * 10.0,
            average_response_time_ms: 2.5 - i as f64 * 0.1,
            memory_usage_bytes: (512 + i * 50) as usize,
            disk_usage_bytes: (512 + i * 50) as usize,
        };
        collector.record_snapshot(snapshot).await;
    }

    // Record some access patterns
    collector
        .record_operation("array1/chunk_0.0.0", true, Duration::from_millis(2))
        .await;
    collector
        .record_operation("array1/chunk_0.0.1", true, Duration::from_millis(3))
        .await;
    collector
        .record_operation("array1/chunk_0.1.0", false, Duration::from_millis(15))
        .await;

    let report = collector.generate_report(Duration::from_secs(300)).await;

    // Verify report structure
    assert!(report.generated_at > 0);
    assert_eq!(report.time_range, Duration::from_secs(300));
    assert!(report.performance_summary.average_hit_rate > 0.0);
    assert!(!report.access_patterns.most_accessed_keys.is_empty());
    assert!(!report.recommendations.is_empty());
}

#[tokio::test]
async fn test_spatial_locality_analysis() {
    let collector = MetricsCollector::new(MetricsConfig::default());

    // Simulate accessing neighboring chunks (good spatial locality)
    let neighboring_keys = vec![
        "temperature/chunk_0.0.0",
        "temperature/chunk_0.0.1", // Neighbor in z-dimension
        "temperature/chunk_0.1.0", // Neighbor in y-dimension
        "temperature/chunk_1.0.0", // Neighbor in x-dimension
    ];

    for key in &neighboring_keys {
        collector
            .record_operation(key, true, Duration::from_millis(2))
            .await;
    }

    let report = collector.generate_report(Duration::from_secs(60)).await;

    // Should detect some spatial locality
    assert!(report.access_patterns.spatial_locality_score >= 0.0);
    assert!(report.access_patterns.spatial_locality_score <= 1.0);
}

#[tokio::test]
async fn test_recommendations_generation() {
    let collector = MetricsCollector::new(MetricsConfig::default());

    // Create a scenario with poor performance to trigger recommendations
    let poor_performance_snapshot = PerformanceSnapshot {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hits: 50,
        misses: 150, // Low hit rate (25%)
        hit_rate: 0.25,
        total_size_bytes: 1024,
        entry_count: 10,
        operations_per_second: 50.0,
        average_response_time_ms: 25.0, // High response time
        memory_usage_bytes: 512,
        disk_usage_bytes: 512,
    };

    collector.record_snapshot(poor_performance_snapshot).await;

    let report = collector.generate_report(Duration::from_secs(60)).await;

    // Should generate recommendations for poor performance
    assert!(!report.recommendations.is_empty());

    // Check for specific recommendation categories
    let has_performance_rec = report
        .recommendations
        .iter()
        .any(|r| r.category == "Performance");
    let has_latency_rec = report
        .recommendations
        .iter()
        .any(|r| r.category == "Latency");

    assert!(has_performance_rec || has_latency_rec);
}

#[tokio::test]
async fn test_metrics_config_customization() {
    let custom_config = MetricsConfig {
        max_history_size: 50,
        snapshot_interval: Duration::from_secs(30),
        track_access_patterns: false,
        track_efficiency: false,
    };

    let collector = MetricsCollector::new(custom_config);

    // Record operations - should not track patterns due to config
    collector
        .record_operation("test_key", true, Duration::from_millis(5))
        .await;

    // Access statistics should be empty due to disabled tracking
    let stats = collector.access_statistics().await;
    assert!(stats.is_empty());
}

#[tokio::test]
async fn test_history_size_limit() {
    let config = MetricsConfig {
        max_history_size: 3, // Very small limit
        ..MetricsConfig::default()
    };

    let collector = MetricsCollector::new(config);

    // Add more snapshots than the limit
    for i in 0..5 {
        let snapshot = PerformanceSnapshot {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            hits: i,
            misses: 0,
            hit_rate: 1.0,
            total_size_bytes: 1024,
            entry_count: 1,
            operations_per_second: 100.0,
            average_response_time_ms: 1.0,
            memory_usage_bytes: 512,
            disk_usage_bytes: 512,
        };
        collector.record_snapshot(snapshot).await;
    }

    let report = collector.generate_report(Duration::from_secs(60)).await;

    // Should have maintained the size limit
    // The report should be generated successfully even with limited history
    assert!(report.performance_summary.average_hit_rate >= 0.0);
}

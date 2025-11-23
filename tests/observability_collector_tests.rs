use audiotab::observability::{NodeMetrics, MetricsCollector};
use std::sync::Arc;

#[test]
fn test_collector_registration() {
    let mut collector = MetricsCollector::new();
    let metrics = Arc::new(NodeMetrics::new("node1"));

    collector.register("node1", metrics.clone());

    let snapshot = collector.snapshot();
    assert_eq!(snapshot.len(), 1);
    assert!(snapshot.contains_key("node1"));
}

#[test]
fn test_collector_aggregation() {
    let mut collector = MetricsCollector::new();

    let m1 = Arc::new(NodeMetrics::new("node1"));
    let m2 = Arc::new(NodeMetrics::new("node2"));

    m1.record_frame_processed();
    m1.record_frame_processed();
    m2.record_frame_processed();

    collector.register("node1", m1);
    collector.register("node2", m2);

    let snapshot = collector.snapshot();

    assert_eq!(snapshot.get("node1").unwrap().frames_processed, 2);
    assert_eq!(snapshot.get("node2").unwrap().frames_processed, 1);
}

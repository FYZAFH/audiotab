use audiotab::observability::NodeMetrics;
use std::sync::Arc;

#[test]
fn test_metrics_creation() {
    let metrics = NodeMetrics::new("test_node");
    assert_eq!(metrics.node_id(), "test_node");
    assert_eq!(metrics.frames_processed(), 0);
    assert_eq!(metrics.errors_count(), 0);
}

#[test]
fn test_metrics_increment() {
    let metrics = Arc::new(NodeMetrics::new("test_node"));

    metrics.record_frame_processed();
    metrics.record_frame_processed();
    assert_eq!(metrics.frames_processed(), 2);

    metrics.record_error();
    assert_eq!(metrics.errors_count(), 1);
}

#[tokio::test]
async fn test_metrics_latency_tracking() {
    let metrics = NodeMetrics::new("test_node");

    let start = metrics.start_processing();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    metrics.finish_processing(start);

    let avg_latency = metrics.avg_latency_us();
    assert!(avg_latency >= 10_000); // At least 10ms in microseconds
}

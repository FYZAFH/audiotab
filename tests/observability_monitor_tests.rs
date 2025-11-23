use audiotab::observability::{NodeMetrics, MetricsCollector, PipelineMonitor};
use std::sync::Arc;

#[test]
fn test_monitor_report() {
    let mut collector = MetricsCollector::new();

    let m1 = Arc::new(NodeMetrics::new("gen"));
    let m2 = Arc::new(NodeMetrics::new("gain"));

    m1.record_frame_processed();
    m1.record_frame_processed();
    m2.record_frame_processed();
    m2.record_error();

    collector.register("gen", m1);
    collector.register("gain", m2);

    let monitor = PipelineMonitor::new(collector);
    let report = monitor.generate_report();

    assert!(report.contains("gen"));
    assert!(report.contains("gain"));
    assert!(report.contains("2 frames"));
    assert!(report.contains("1 error"));
}

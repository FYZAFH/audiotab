use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;

#[tokio::test]
async fn test_pipeline_with_metrics() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {"label": "Test"}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ]
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();
    pipeline.start().await.unwrap();

    // Trigger some frames
    for i in 0..5 {
        pipeline.trigger(DataFrame::new(i * 100, i)).await.unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get metrics
    let monitor = pipeline.get_monitor().unwrap();
    let report = monitor.generate_report();

    assert!(report.contains("gen"));
    assert!(report.contains("gain"));
    assert!(report.contains("print"));

    pipeline.stop().await.unwrap();
}

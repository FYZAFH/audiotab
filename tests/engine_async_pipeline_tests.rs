use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;

#[tokio::test]
async fn test_async_pipeline_creation() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {"label": "AsyncTest"}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ]
    });

    let pipeline = AsyncPipeline::from_json(config).await;
    assert!(pipeline.is_ok());
}

#[tokio::test]
async fn test_async_pipeline_execution() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"}
        ]
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Start pipeline (spawns node tasks)
    pipeline.start().await.unwrap();

    // Trigger 3 executions
    for i in 0..3 {
        pipeline.trigger(DataFrame::new(i * 1000, i)).await.unwrap();
    }

    // Wait a bit for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Stop pipeline
    pipeline.stop().await.unwrap();
}

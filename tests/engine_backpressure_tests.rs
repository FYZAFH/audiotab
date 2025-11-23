use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_backpressure_blocks_when_full() {
    let config = serde_json::json!({
        "pipeline_config": {
            "channel_capacity": 2  // Small buffer to trigger backpressure
        },
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}}
        ],
        "connections": []
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();
    pipeline.start().await.unwrap();

    // Fill the channel (capacity = 2)
    pipeline.trigger(DataFrame::new(0, 0)).await.unwrap();
    pipeline.trigger(DataFrame::new(1000, 1)).await.unwrap();

    // Third trigger should block or timeout since no consumer
    let result = timeout(
        Duration::from_millis(50),
        pipeline.trigger(DataFrame::new(2000, 2))
    ).await;

    // Should timeout because channel is full and no one is consuming
    assert!(result.is_err(), "Expected timeout due to backpressure");

    pipeline.stop().await.unwrap();
}

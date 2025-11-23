use audiotab::engine::PipelinePool;
use audiotab::core::DataFrame;

#[tokio::test]
async fn test_pipeline_pool_concurrent_execution() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"}
        ]
    });

    let mut pool = PipelinePool::new(config, 5).await.unwrap(); // 5 concurrent instances

    // Trigger 10 executions rapidly
    let mut handles = vec![];
    for i in 0..10 {
        let trigger_frame = DataFrame::new(i * 100, i);
        let handle = pool.execute(trigger_frame).await.unwrap();
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // All 10 should complete even though only 5 can run concurrently
}

#[tokio::test]
async fn test_pipeline_pool_resource_reuse() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 50}}
        ],
        "connections": []
    });

    let mut pool = PipelinePool::new(config, 2).await.unwrap();

    // Execute 5 times - should reuse the 2 pipeline instances
    for i in 0..5 {
        let handle = pool.execute(DataFrame::new(i * 100, i)).await.unwrap();
        handle.await.unwrap().unwrap();
    }
}

use audiotab::engine::Pipeline;

#[tokio::test]
async fn test_pipeline_creation() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {"label": "Output"}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ]
    });

    let pipeline = Pipeline::from_json(config).await;
    assert!(pipeline.is_ok());
}

#[tokio::test]
async fn test_pipeline_execute() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "print", "type": "Print", "config": {"label": "Test"}}
        ],
        "connections": [
            {"from": "gen", "to": "print"}
        ]
    });

    let mut pipeline = Pipeline::from_json(config).await.unwrap();

    // Trigger one execution
    let result = pipeline.execute_once().await;
    assert!(result.is_ok());
}

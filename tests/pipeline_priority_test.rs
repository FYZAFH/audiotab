use audiotab::engine::{AsyncPipeline, Priority};
use serde_json::json;

#[tokio::test]
async fn test_pipeline_with_priority() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {}}
        ],
        "connections": [],
        "pipeline_config": {
            "priority": "High"
        }
    });

    let pipeline = AsyncPipeline::from_json(config).await.unwrap();
    assert_eq!(pipeline.priority(), Priority::High);
}

#[tokio::test]
async fn test_pipeline_default_priority() {
    let config = json!({
        "nodes": [{"id": "gen", "type": "SineGenerator", "config": {}}],
        "connections": [],
        "pipeline_config": {}
    });

    let pipeline = AsyncPipeline::from_json(config).await.unwrap();
    assert_eq!(pipeline.priority(), Priority::Normal);
}

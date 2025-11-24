use audiotab::engine::{AsyncPipeline, PipelineState};
use audiotab::core::DataFrame;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_pipeline_state_transitions() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "sample_rate": 48000}},
            {"id": "print", "type": "Print", "config": {}}
        ],
        "connections": [
            {"from": "gen", "to": "print"}
        ],
        "pipeline_config": {
            "channel_capacity": 10
        }
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Initially Idle
    assert_eq!(pipeline.state().name(), "Idle");

    // Start pipeline (should transition Idle -> Initializing -> Running)
    pipeline.start().await.unwrap();
    assert_eq!(pipeline.state().name(), "Running");

    // Send a trigger frame
    let frame = DataFrame::new(0, 0);
    pipeline.trigger(frame).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // Stop pipeline (should transition to Completed)
    pipeline.stop().await.unwrap();
    assert_eq!(pipeline.state().name(), "Completed");
}

#[tokio::test]
async fn test_set_state_bypasses_validation() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {}}
        ],
        "connections": [],
        "pipeline_config": {}
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Initially Idle
    assert_eq!(pipeline.state().name(), "Idle");

    // set_state bypasses validation
    pipeline.set_state(PipelineState::Initializing { progress: 50 });
    assert_eq!(pipeline.state().name(), "Initializing");
}

#[tokio::test]
async fn test_invalid_state_transition() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {}}
        ],
        "connections": [],
        "pipeline_config": {}
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Try invalid transition: Idle -> Completed (should fail)
    let result = pipeline.transition_to(PipelineState::Completed {
        duration: None,
        total_frames: 0,
    });

    assert!(result.is_err());
    assert_eq!(pipeline.state().name(), "Idle");
}

#[tokio::test]
async fn test_valid_state_transition() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {}}
        ],
        "connections": [],
        "pipeline_config": {}
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Valid transition: Idle -> Initializing
    let result = pipeline.transition_to(PipelineState::Initializing { progress: 0 });
    assert!(result.is_ok());
    assert_eq!(pipeline.state().name(), "Initializing");

    // Valid transition: Initializing -> Running
    let result = pipeline.transition_to(PipelineState::Running {
        start_time: None,
        frames_processed: 0,
    });
    assert!(result.is_ok());
    assert_eq!(pipeline.state().name(), "Running");
}

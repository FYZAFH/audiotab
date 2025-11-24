use audiotab::engine::{AsyncPipeline, Priority};
use audiotab::hal::registry::DeviceRegistry;
use audiotab::core::DataFrame;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_phase1_complete_integration() {
    // Test 1: HAL with mock devices
    let registry = DeviceRegistry::with_defaults();
    let mut audio_source = registry.create_source("SimulatedAudio").unwrap();

    let audio_config = json!({
        "frequency": 1000.0,
        "sample_rate": 48000,
        "amplitude": 0.5,
        "samples_per_frame": 512
    });

    audio_source.configure(audio_config).await.unwrap();
    audio_source.open().await.unwrap();
    audio_source.start().await.unwrap();

    let frame = audio_source.read_frame().await.unwrap();
    assert_eq!(frame.payload.get("audio").unwrap().len(), 512);

    audio_source.stop().await.unwrap();
    audio_source.close().await.unwrap();

    // Test 2: Pipeline with state machine
    let pipeline_config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ],
        "pipeline_config": {
            "channel_capacity": 10,
            "priority": "High"
        }
    });

    let mut pipeline = AsyncPipeline::from_json(pipeline_config).await.unwrap();

    // Verify initial state
    assert_eq!(pipeline.state().name(), "Idle");
    assert_eq!(pipeline.priority(), Priority::High);

    // Start pipeline (transitions Idle -> Initializing -> Running)
    pipeline.start().await.unwrap();
    assert_eq!(pipeline.state().name(), "Running");

    // Trigger a few frames
    for i in 0..5 {
        let frame = DataFrame::new(i * 1000, i);
        pipeline.trigger(frame).await.unwrap();
    }

    sleep(Duration::from_millis(500)).await;

    // Stop pipeline (transitions Running -> Completed)
    pipeline.stop().await.unwrap();
    // State checked in stop() method

    println!("Phase 1 integration test passed!");
}

#[tokio::test]
async fn test_hal_trigger_integration() {
    let registry = DeviceRegistry::with_defaults();
    let mut trigger = registry.create_source("SimulatedTrigger").unwrap();

    let config = json!({
        "mode": "periodic",
        "interval_ms": 20
    });

    trigger.configure(config).await.unwrap();
    trigger.open().await.unwrap();
    trigger.start().await.unwrap();

    // Read 3 trigger frames
    for _ in 0..3 {
        let frame = trigger.read_frame().await.unwrap();
        assert!(frame.payload.is_empty());
        assert_eq!(frame.metadata.get("trigger_mode"), Some(&"periodic".to_string()));
    }

    trigger.stop().await.unwrap();
    trigger.close().await.unwrap();

    println!("HAL trigger integration test passed!");
}

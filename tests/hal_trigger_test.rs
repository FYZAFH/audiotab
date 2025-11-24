use audiotab::hal::mock::SimulatedTriggerSource;
use audiotab::hal::DeviceSource;
use serde_json::json;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_trigger_periodic_mode() {
    let mut trigger = SimulatedTriggerSource::new();

    // Configure for 10ms period (100 Hz)
    let config = json!({
        "mode": "periodic",
        "interval_ms": 10
    });

    trigger.configure(config).await.unwrap();
    trigger.open().await.unwrap();
    trigger.start().await.unwrap();

    // Read two trigger frames
    let frame1 = timeout(Duration::from_millis(50), trigger.read_frame())
        .await
        .unwrap()
        .unwrap();

    let frame2 = timeout(Duration::from_millis(50), trigger.read_frame())
        .await
        .unwrap()
        .unwrap();

    // Triggers should have empty payload (just timestamp)
    assert!(frame1.payload.is_empty());
    assert!(frame2.sequence_id > frame1.sequence_id);

    trigger.stop().await.unwrap();
    trigger.close().await.unwrap();
}

#[tokio::test]
async fn test_trigger_manual_mode() {
    let mut trigger = SimulatedTriggerSource::new();

    let config = json!({
        "mode": "manual"
    });

    trigger.configure(config).await.unwrap();
    trigger.open().await.unwrap();
    trigger.start().await.unwrap();

    // In manual mode, trigger() must be called explicitly
    trigger.trigger();

    let frame = timeout(Duration::from_millis(50), trigger.read_frame())
        .await
        .unwrap()
        .unwrap();

    assert!(frame.payload.is_empty());
    assert_eq!(frame.metadata.get("trigger_mode"), Some(&"manual".to_string()));

    trigger.stop().await.unwrap();
    trigger.close().await.unwrap();
}

use audiotab::hal::mock::SimulatedAudioSource;
use audiotab::hal::{DeviceSource, DeviceState};
use serde_json::json;

#[tokio::test]
async fn test_simulated_audio_source_lifecycle() {
    let mut source = SimulatedAudioSource::new();

    // Initially unopened
    assert_eq!(source.state(), DeviceState::Unopened);

    // Configure with 1kHz sine wave
    let config = json!({
        "frequency": 1000.0,
        "sample_rate": 48000,
        "amplitude": 1.0,
        "samples_per_frame": 1024
    });
    source.configure(config).await.unwrap();

    // Open device
    source.open().await.unwrap();
    assert_eq!(source.state(), DeviceState::Opened);

    // Start streaming
    source.start().await.unwrap();
    assert_eq!(source.state(), DeviceState::Running);

    // Read a frame
    let frame = source.read_frame().await.unwrap();
    assert_eq!(frame.payload.get("audio").unwrap().len(), 1024);

    // Stop and close
    source.stop().await.unwrap();
    assert_eq!(source.state(), DeviceState::Stopped);

    source.close().await.unwrap();
    assert_eq!(source.state(), DeviceState::Closed);
}

#[tokio::test]
async fn test_simulated_audio_generates_sine_wave() {
    let mut source = SimulatedAudioSource::new();

    let config = json!({
        "frequency": 440.0,  // A4 note
        "sample_rate": 48000,
        "amplitude": 1.0,
        "samples_per_frame": 48
    });

    source.configure(config).await.unwrap();
    source.open().await.unwrap();
    source.start().await.unwrap();

    let frame = source.read_frame().await.unwrap();
    let samples = frame.payload.get("audio").unwrap();

    // Should be a sine wave - check first sample is near amplitude * sin(0)
    assert!(samples[0].abs() < 0.1); // sin(0) ≈ 0

    // Check that values are within [-1, 1]
    for sample in samples.iter() {
        assert!(sample.abs() <= 1.0);
    }
}

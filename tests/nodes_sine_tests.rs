use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::SineGenerator;

#[tokio::test]
async fn test_sine_generator_creates_data() {
    let mut generator = SineGenerator::new();
    let config = serde_json::json!({
        "frequency": 440.0,
        "sample_rate": 48000.0,
        "frame_size": 1024
    });

    generator.on_create(config).await.unwrap();

    let empty_frame = DataFrame::new(0, 0);
    let result = generator.process(empty_frame).await.unwrap();

    // Should create main_channel data
    assert!(result.payload.contains_key("main_channel"));
    assert_eq!(result.payload.get("main_channel").unwrap().len(), 1024);
}

#[tokio::test]
async fn test_sine_wave_values() {
    let mut generator = SineGenerator::new();
    let config = serde_json::json!({
        "frequency": 1.0,  // 1 Hz for easy verification
        "sample_rate": 8.0,  // 8 samples per second
        "frame_size": 8
    });

    generator.on_create(config).await.unwrap();

    let result = generator.process(DataFrame::new(0, 0)).await.unwrap();
    let data = result.payload.get("main_channel").unwrap();

    // At 1 Hz with 8 samples/sec, we should get one complete sine cycle
    // Samples at 0°, 45°, 90°, 135°, 180°, 225°, 270°, 315°
    assert!(data[0].abs() < 0.01); // sin(0) ≈ 0
    assert!((data[2] - 1.0).abs() < 0.01); // sin(90°) ≈ 1
    assert!(data[4].abs() < 0.01); // sin(180°) ≈ 0
}

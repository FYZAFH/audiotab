use audiotab::core::{DataFrame, ProcessingNode};
use audiotab::nodes::SineGenerator;
use tokio::sync::mpsc;

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

#[tokio::test]
async fn test_sine_generator_streaming() {
    let mut generator = SineGenerator::new();
    let config = serde_json::json!({
        "frequency": 1.0,
        "sample_rate": 4.0,
        "frame_size": 4
    });
    generator.on_create(config).await.unwrap();

    let (tx_in, rx_in) = mpsc::channel::<DataFrame>(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    let handle = tokio::spawn(async move {
        generator.run(rx_in, tx_out).await
    });

    // Send 3 empty trigger frames
    for i in 0..3 {
        tx_in.send(DataFrame::new(i * 1000, i)).await.unwrap();
    }
    drop(tx_in);

    // Receive 3 generated frames
    let frame1 = rx_out.recv().await.unwrap();
    let frame2 = rx_out.recv().await.unwrap();
    let frame3 = rx_out.recv().await.unwrap();

    // Verify data was generated
    assert_eq!(frame1.payload.get("main_channel").unwrap().len(), 4);
    assert_eq!(frame2.payload.get("main_channel").unwrap().len(), 4);
    assert_eq!(frame3.payload.get("main_channel").unwrap().len(), 4);

    handle.await.unwrap().unwrap();
}

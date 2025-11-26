use audiotab::hal::*;

// Note: This test may hang on macOS in CI/headless environments due to CPAL audio enumeration.
// Run manually with: cargo test test_audio_driver_discovery --ignored
#[tokio::test]
#[ignore = "CPAL audio enumeration may hang on macOS in CI environments"]
async fn test_audio_driver_discovery() {
    let driver = AudioDriver::new();
    assert_eq!(driver.driver_id(), "cpal-audio");

    let devices = driver.discover_devices().await.unwrap();
    // Should find at least default input/output
    assert!(devices.len() >= 1, "No audio devices found");

    // Verify devices have acoustic type
    for device in devices {
        assert_eq!(device.hardware_type, HardwareType::Acoustic);
    }
}

#[tokio::test]
async fn test_audio_device_creation() {
    use audiotab::hal::*;

    let driver = AudioDriver::new();
    let config = DeviceConfig {
        name: "Test Device".to_string(),
        sample_rate: 48000,
        format: SampleFormat::F32,
        buffer_size: 1024,
        channel_mapping: ChannelMapping::default(),
        calibration: Calibration::default(),
    };

    let mut device = driver.create_device("test-id", config).unwrap();

    // Should not be streaming initially
    assert!(!device.is_streaming());

    // Get channels
    let channels = device.get_channels();
    assert!(channels.filled_rx.is_empty());
}

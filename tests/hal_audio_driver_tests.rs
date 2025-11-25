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

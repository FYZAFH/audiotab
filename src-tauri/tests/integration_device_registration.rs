use audiotab::hal::{RegisteredHardware, HardwareType, Direction, AudioProtocol, ChannelMapping, Calibration, ChannelRoute};
use app_lib::hardware_manager::HardwareConfigManager;
use tempfile::tempdir;

#[tokio::test]
async fn test_full_device_registration_workflow() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("hardware_config.json");

    let manager = HardwareConfigManager::new(config_path.clone());
    manager.ensure_config_file().await.unwrap();
    manager.load().await.unwrap();

    // Register first device
    let mic = RegisteredHardware {
        registration_id: "reg-mic-001".to_string(),
        device_id: "dev-mic-001".to_string(),
        hardware_name: "MacBook Pro Microphone".to_string(),
        driver_id: "cpal".to_string(),
        hardware_type: HardwareType::Acoustic,
        direction: Direction::Input,
        user_name: "Main Mic".to_string(),
        enabled: true,
        protocol: Some(AudioProtocol::CoreAudio),
        sample_rate: 48000,
        channels: 2,
        channel_mapping: ChannelMapping {
            physical_channels: 2,
            virtual_channels: 2,
            routing: vec![ChannelRoute::Direct(0), ChannelRoute::Direct(1)],
        },
        calibration: Calibration { gain: 1.0, offset: 0.0 },
        max_voltage: 0.0,
        notes: "Primary recording device".to_string(),
    };

    manager.register_device(mic).await.unwrap();

    // Register second device
    let speakers = RegisteredHardware {
        registration_id: "reg-spk-001".to_string(),
        device_id: "dev-spk-001".to_string(),
        hardware_name: "MacBook Pro Speakers".to_string(),
        driver_id: "cpal".to_string(),
        hardware_type: HardwareType::Acoustic,
        direction: Direction::Output,
        user_name: "Main Speakers".to_string(),
        enabled: true,
        protocol: Some(AudioProtocol::CoreAudio),
        sample_rate: 48000,
        channels: 2,
        channel_mapping: ChannelMapping {
            physical_channels: 2,
            virtual_channels: 2,
            routing: vec![ChannelRoute::Direct(0), ChannelRoute::Direct(1)],
        },
        calibration: Calibration { gain: 1.0, offset: 0.0 },
        max_voltage: 0.0,
        notes: "Primary playback device".to_string(),
    };

    manager.register_device(speakers).await.unwrap();

    // Verify list
    let devices = manager.get_registered_devices().await.unwrap();
    assert_eq!(devices.len(), 2);

    // Update device
    let mut updated_mic = devices.iter()
        .find(|d| d.registration_id == "reg-mic-001")
        .unwrap()
        .clone();
    updated_mic.sample_rate = 96000;
    manager.update_device("reg-mic-001", updated_mic).await.unwrap();

    // Reload from disk
    let manager2 = HardwareConfigManager::new(config_path);
    manager2.load().await.unwrap();
    let devices2 = manager2.get_registered_devices().await.unwrap();

    assert_eq!(devices2.len(), 2);
    let updated_mic_check = devices2.iter()
        .find(|d| d.registration_id == "reg-mic-001")
        .unwrap();
    assert_eq!(updated_mic_check.sample_rate, 96000);

    // Remove device
    manager2.remove_device("reg-spk-001").await.unwrap();
    let devices3 = manager2.get_registered_devices().await.unwrap();
    assert_eq!(devices3.len(), 1);
    assert_eq!(devices3[0].registration_id, "reg-mic-001");
}

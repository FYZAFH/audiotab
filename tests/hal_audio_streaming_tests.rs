use audiotab::hal::*;
use std::time::Duration;

#[tokio::test]
async fn test_audio_streaming_basic() {
    let driver = AudioDriver::new();

    // Discover default input device
    let devices = driver.discover_devices().await.unwrap();
    let input_device = devices.iter()
        .find(|d| d.name.contains("Input"))
        .expect("No input device found");

    let config = DeviceConfig {
        name: input_device.name.clone(),
        sample_rate: 48000,
        format: SampleFormat::F32,
        buffer_size: 1024,
        channel_mapping: ChannelMapping {
            physical_channels: 2,
            virtual_channels: 2,
            routing: vec![],
        },
        calibration: Calibration::default(),
    };

    let mut device = driver.create_device(&input_device.id, config).unwrap();
    let mut channels = device.get_channels();

    // Start streaming
    device.start().await.unwrap();
    assert!(device.is_streaming());

    // Wait for a buffer (with timeout)
    tokio::select! {
        buffer = tokio::task::spawn_blocking(move || channels.filled_rx.recv()) => {
            let packet = buffer.unwrap().unwrap();
            assert!(packet.sample_rate > 0);
            println!("Received audio packet: {} samples", match &packet.data {
                SampleData::F32(v) => v.len(),
                _ => 0,
            });
        }
        _ = tokio::time::sleep(Duration::from_secs(2)) => {
            panic!("Timeout waiting for audio packet");
        }
    }

    device.stop().await.unwrap();
}

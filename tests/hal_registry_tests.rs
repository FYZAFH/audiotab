use audiotab::hal::*;
use async_trait::async_trait;
use anyhow::Result;

struct MockDriver;

#[async_trait]
impl HardwareDriver for MockDriver {
    fn driver_id(&self) -> &str {
        "mock-driver"
    }

    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>> {
        Ok(vec![DeviceInfo {
            id: "mock-device-1".to_string(),
            name: "Mock Device".to_string(),
            hardware_type: HardwareType::Acoustic,
            driver_id: "mock-driver".to_string(),
        }])
    }

    fn create_device(&self, _id: &str, config: DeviceConfig) -> Result<Box<dyn Device>> {
        Ok(Box::new(MockDevice::new(config)))
    }
}

struct MockDevice {
    _config: DeviceConfig,
    streaming: bool,
}

impl MockDevice {
    fn new(config: DeviceConfig) -> Self {
        Self {
            _config: config,
            streaming: false,
        }
    }
}

#[async_trait]
impl Device for MockDevice {
    async fn start(&mut self) -> Result<()> {
        self.streaming = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.streaming = false;
        Ok(())
    }

    fn get_channels(&mut self) -> DeviceChannels {
        let (_filled_tx, filled_rx) = crossbeam_channel::bounded(2);
        let (empty_tx, _empty_rx) = crossbeam_channel::bounded(2);
        DeviceChannels { filled_rx, empty_tx }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            can_input: true,
            can_output: false,
            supported_formats: vec![SampleFormat::F32],
            supported_sample_rates: vec![48000],
            max_channels: 2,
        }
    }

    fn is_streaming(&self) -> bool {
        self.streaming
    }
}

#[tokio::test]
async fn test_registry_register_and_list() {
    let mut registry = HardwareRegistry::new();

    // Initially empty
    assert_eq!(registry.list_drivers().len(), 0);

    // Register mock driver
    registry.register(MockDriver);
    assert_eq!(registry.list_drivers().len(), 1);
    assert!(registry.list_drivers().contains(&"mock-driver".to_string()));
}

#[tokio::test]
async fn test_registry_discover_all() {
    let mut registry = HardwareRegistry::new();
    registry.register(MockDriver);

    let devices = registry.discover_all().await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, "mock-device-1");
    assert_eq!(devices[0].hardware_type, HardwareType::Acoustic);
}

#[tokio::test]
async fn test_registry_create_device() {
    let mut registry = HardwareRegistry::new();
    registry.register(MockDriver);

    let config = DeviceConfig {
        name: "Test Device".to_string(),
        sample_rate: 48000,
        format: SampleFormat::F32,
        buffer_size: 1024,
        channel_mapping: ChannelMapping::default(),
        calibration: Calibration::default(),
    };

    let mut device = registry.create_device("mock-driver", "mock-device-1", config).unwrap();

    // Verify device works
    assert!(!device.is_streaming());
    device.start().await.unwrap();
    assert!(device.is_streaming());
    device.stop().await.unwrap();
    assert!(!device.is_streaming());
}

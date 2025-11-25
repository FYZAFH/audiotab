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

    fn create_device(&self, _id: &str, _config: DeviceConfig) -> Result<Box<dyn Device>> {
        anyhow::bail!("Not implemented for mock")
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

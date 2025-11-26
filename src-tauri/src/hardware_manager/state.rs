use std::sync::Arc;
use tokio::sync::RwLock;
use audiotab::hal::*;
use anyhow::Result;

pub struct HardwareManagerState {
    registry: Arc<RwLock<HardwareRegistry>>,
}

impl HardwareManagerState {
    pub fn new() -> Self {
        let mut registry = HardwareRegistry::new();

        // Register built-in drivers
        registry.register(AudioDriver::new());

        Self {
            registry: Arc::new(RwLock::new(registry)),
        }
    }

    pub async fn discover_devices(&self) -> Result<Vec<DeviceInfo>> {
        let registry = self.registry.read().await;
        registry.discover_all().await
    }

    pub async fn create_device(
        &self,
        driver_id: &str,
        device_id: &str,
        config: DeviceConfig,
    ) -> Result<()> {
        let registry = self.registry.read().await;
        let _device = registry.create_device(driver_id, device_id, config)?;
        // TODO: Store device in state
        Ok(())
    }
}

impl Default for HardwareManagerState {
    fn default() -> Self {
        Self::new()
    }
}

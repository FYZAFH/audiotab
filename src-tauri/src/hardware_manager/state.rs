use std::sync::Arc;
use tokio::sync::RwLock;
use audiotab::hal::*;
use anyhow::Result;
use super::config::HardwareConfigManager;

pub struct HardwareManagerState {
    registry: Arc<RwLock<HardwareRegistry>>,
    config_manager: Arc<HardwareConfigManager>,
}

impl HardwareManagerState {
    pub fn new() -> Self {
        let mut registry = HardwareRegistry::new();

        // Register built-in drivers
        registry.register(AudioDriver::new());

        // Use home directory for config
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join(".audiotab")
            .join("hardware_config.json");

        let config_manager = Arc::new(HardwareConfigManager::new(config_path));

        Self {
            registry: Arc::new(RwLock::new(registry)),
            config_manager,
        }
    }

    /// Get a clone of the registry Arc for sharing with other components like KernelManager
    pub fn get_registry_arc(&self) -> Arc<RwLock<HardwareRegistry>> {
        Arc::clone(&self.registry)
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

    pub fn config_manager(&self) -> &HardwareConfigManager {
        &self.config_manager
    }
}

impl Default for HardwareManagerState {
    fn default() -> Self {
        Self::new()
    }
}

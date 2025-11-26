use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use super::traits::HardwareDriver;
use super::types::{DeviceInfo, DeviceConfig};
use super::Device;

/// Central registry for hardware drivers
pub struct HardwareRegistry {
    drivers: HashMap<String, Arc<dyn HardwareDriver>>,
}

impl HardwareRegistry {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
        }
    }

    /// Register a hardware driver
    pub fn register(&mut self, driver: impl HardwareDriver + 'static) {
        let driver_id = driver.driver_id().to_string();
        self.drivers.insert(driver_id, Arc::new(driver));
    }

    /// List all registered drivers
    pub fn list_drivers(&self) -> Vec<String> {
        self.drivers.keys().cloned().collect()
    }

    /// Get driver by ID
    pub fn get_driver(&self, driver_id: &str) -> Option<Arc<dyn HardwareDriver>> {
        self.drivers.get(driver_id).cloned()
    }

    /// Discover devices from all drivers
    pub async fn discover_all(&self) -> Result<Vec<DeviceInfo>> {
        let mut all_devices = Vec::new();

        for driver in self.drivers.values() {
            match driver.discover_devices().await {
                Ok(devices) => all_devices.extend(devices),
                Err(e) => eprintln!("Driver {} discovery failed: {}", driver.driver_id(), e),
            }
        }

        Ok(all_devices)
    }

    /// Create device from any registered driver
    pub fn create_device(
        &self,
        driver_id: &str,
        device_id: &str,
        config: DeviceConfig,
    ) -> Result<Box<dyn Device>> {
        let driver = self.get_driver(driver_id)
            .ok_or_else(|| anyhow::anyhow!("Driver {} not found", driver_id))?;

        driver.create_device(device_id, config)
    }
}

impl Default for HardwareRegistry {
    fn default() -> Self {
        Self::new()
    }
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use anyhow::{Result, Context};
use super::{
    HardwareRegistry, HardwareDriver, Device,
    DeviceProfile, DeviceStorage, DeviceInfo,
};

/// Manages hardware devices and their configurations
pub struct DeviceManager {
    /// Hardware driver registry
    registry: HardwareRegistry,

    /// Device profile storage
    storage: DeviceStorage,

    /// Active device profiles (loaded in memory)
    profiles: HashMap<String, DeviceProfile>,

    /// Active device instances
    active_devices: Arc<Mutex<HashMap<String, Box<dyn Device>>>>,
}

impl DeviceManager {
    /// Create new device manager
    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        let storage = DeviceStorage::new(storage_dir)?;
        let profiles = storage.list_all()
            .context("Failed to load device profiles")?
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();

        Ok(Self {
            registry: HardwareRegistry::new(),
            storage,
            profiles,
            active_devices: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Register a hardware driver
    pub fn register_driver(&mut self, driver: impl HardwareDriver + 'static) {
        self.registry.register(driver);
    }

    /// Discover all available devices from all drivers
    pub async fn discover_all(&self) -> Result<Vec<DeviceInfo>> {
        self.registry.discover_all().await
    }

    /// Add a new device profile
    pub fn add_profile(&mut self, profile: DeviceProfile) -> Result<()> {
        self.storage.save(&profile)?;
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    /// Update an existing device profile
    pub fn update_profile(&mut self, profile: DeviceProfile) -> Result<()> {
        if !self.profiles.contains_key(&profile.id) {
            anyhow::bail!("Profile {} not found", profile.id);
        }

        self.storage.save(&profile)?;
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    /// Delete a device profile
    pub fn delete_profile(&mut self, id: &str) -> Result<()> {
        self.storage.delete(id)?;
        self.profiles.remove(id);
        Ok(())
    }

    /// Get a device profile by ID
    pub fn get_profile(&self, id: &str) -> Option<&DeviceProfile> {
        self.profiles.get(id)
    }

    /// List all device profiles
    pub fn list_profiles(&self) -> Vec<&DeviceProfile> {
        self.profiles.values().collect()
    }

    /// Create a device instance from a profile
    pub fn create_device(&self, profile_id: &str) -> Result<Box<dyn Device>> {
        let profile = self.get_profile(profile_id)
            .ok_or_else(|| anyhow::anyhow!("Profile {} not found", profile_id))?;

        self.registry.create_device(
            &profile.driver_id,
            &profile.device_id,
            profile.config.clone(),
        )
    }

    /// Start a device and track it as active
    pub async fn start_device(&self, profile_id: &str) -> Result<()> {
        let mut device = self.create_device(profile_id)?;
        device.start().await?;

        let mut active = self.active_devices.lock().unwrap();
        active.insert(profile_id.to_string(), device);
        Ok(())
    }

    /// Stop an active device
    pub async fn stop_device(&self, profile_id: &str) -> Result<()> {
        let mut active = self.active_devices.lock().unwrap();

        if let Some(mut device) = active.remove(profile_id) {
            device.stop().await?;
        }

        Ok(())
    }

    /// Check if a device is currently active
    pub fn is_device_active(&self, profile_id: &str) -> bool {
        let active = self.active_devices.lock().unwrap();
        active.contains_key(profile_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::hal::drivers::AudioDriver;
    use crate::hal::types::{DeviceConfig, SampleFormat, ChannelMapping, Calibration};
    use crate::hal::device_profile::{DeviceProfile, DeviceMetadata};

    #[tokio::test]
    async fn test_register_and_retrieve_profile() {
        let dir = tempdir().unwrap();
        let mut manager = DeviceManager::new(dir.path().to_path_buf()).unwrap();

        // Register audio driver
        manager.register_driver(AudioDriver::new());

        let profile = DeviceProfile {
            id: "test-mic".to_string(),
            alias: "Test Microphone".to_string(),
            driver_id: "cpal-audio".to_string(),
            device_id: "input-0".to_string(),
            config: DeviceConfig {
                name: "Test".to_string(),
                sample_rate: 48000,
                format: SampleFormat::F32,
                buffer_size: 1024,
                channel_mapping: ChannelMapping::default(),
                calibration: Calibration::default(),
            },
            metadata: DeviceMetadata::default(),
        };

        manager.add_profile(profile.clone()).unwrap();
        let retrieved = manager.get_profile("test-mic").unwrap();
        assert_eq!(retrieved.alias, "Test Microphone");
    }

    #[tokio::test]
    async fn test_discover_devices() {
        let dir = tempdir().unwrap();
        let mut manager = DeviceManager::new(dir.path().to_path_buf()).unwrap();
        manager.register_driver(AudioDriver::new());

        let devices = manager.discover_all().await.unwrap();
        // Should find at least some audio devices on the system
        assert!(devices.len() > 0);
    }
}

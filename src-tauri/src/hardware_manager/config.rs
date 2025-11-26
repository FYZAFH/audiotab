use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use audiotab::hal::{RegisteredHardware, HardwareConfig};
use anyhow::{Result, Context};

/// Manages hardware configuration persistence
pub struct HardwareConfigManager {
    config_path: PathBuf,
    state: Arc<RwLock<HardwareConfig>>,
}

impl HardwareConfigManager {
    /// Creates a new HardwareConfigManager.
    ///
    /// # Important
    /// This creates an empty in-memory state. You must call `load()` after
    /// construction to read the configuration from disk.
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            state: Arc::new(RwLock::new(HardwareConfig::default())),
        }
    }

    pub async fn ensure_config_file(&self) -> Result<()> {
        if !self.config_path.exists() {
            // Create parent directory
            if let Some(parent) = self.config_path.parent() {
                fs::create_dir_all(parent).await
                    .context("Failed to create config directory")?;
            }

            // Write default config
            let default_config = HardwareConfig::default();
            let json = serde_json::to_string_pretty(&default_config)?;
            fs::write(&self.config_path, json).await
                .context("Failed to write default config")?;
        }

        Ok(())
    }

    pub async fn load(&self) -> Result<()> {
        self.ensure_config_file().await?;

        let content = fs::read_to_string(&self.config_path).await
            .context("Failed to read config file")?;

        let config: HardwareConfig = serde_json::from_str(&content)
            .context("Failed to parse config JSON")?;

        *self.state.write().await = config;
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        let config = self.state.read().await;
        let json = serde_json::to_string_pretty(&*config)?;

        // Write to temporary file first
        let temp_path = self.config_path.with_extension("tmp");
        fs::write(&temp_path, json).await
            .context("Failed to write temporary config file")?;

        // Atomic rename
        fs::rename(&temp_path, &self.config_path).await
            .context("Failed to atomically update config file")?;

        Ok(())
    }

    pub async fn get_registered_devices(&self) -> Result<Vec<RegisteredHardware>> {
        let config = self.state.read().await;
        Ok(config.registered_devices.clone())
    }

    pub async fn register_device(&self, device: RegisteredHardware) -> Result<()> {
        let mut config = self.state.write().await;

        // Check for duplicate user_name
        if config.registered_devices.iter().any(|d| d.user_name == device.user_name) {
            anyhow::bail!("Device with user name '{}' already exists", device.user_name);
        }

        config.registered_devices.push(device);
        drop(config); // Release lock before saving

        self.save().await?;
        Ok(())
    }

    pub async fn update_device(&self, registration_id: &str, updated: RegisteredHardware) -> Result<()> {
        let mut config = self.state.write().await;

        // Find device position first
        let device_pos = config.registered_devices
            .iter()
            .position(|d| d.registration_id == registration_id)
            .context("Device not found")?;

        let old_user_name = &config.registered_devices[device_pos].user_name;

        // Check if new user_name conflicts with another device
        if old_user_name != &updated.user_name {
            if config.registered_devices.iter().any(|d| d.registration_id != registration_id && d.user_name == updated.user_name) {
                anyhow::bail!("Device with user name '{}' already exists", updated.user_name);
            }
        }

        config.registered_devices[device_pos] = updated;
        drop(config);

        self.save().await?;
        Ok(())
    }

    pub async fn remove_device(&self, registration_id: &str) -> Result<()> {
        let mut config = self.state.write().await;

        let len_before = config.registered_devices.len();
        config.registered_devices.retain(|d| d.registration_id != registration_id);

        if config.registered_devices.len() == len_before {
            anyhow::bail!("Device not found");
        }

        drop(config);
        self.save().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_manager_creates_config_dir() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("hardware_config.json");

        let manager = HardwareConfigManager::new(config_path.clone());
        manager.ensure_config_file().await.unwrap();

        assert!(config_path.exists());
    }

    #[tokio::test]
    async fn test_config_manager_loads_empty_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("hardware_config.json");

        let manager = HardwareConfigManager::new(config_path);
        manager.ensure_config_file().await.unwrap();
        manager.load().await.unwrap();

        let devices = manager.get_registered_devices().await.unwrap();
        assert_eq!(devices.len(), 0);
    }

    #[tokio::test]
    async fn test_register_device() {
        use audiotab::hal::{HardwareType, Direction, AudioProtocol, ChannelMapping, Calibration, ChannelRoute};

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("hardware_config.json");

        let manager = HardwareConfigManager::new(config_path.clone());
        manager.ensure_config_file().await.unwrap();
        manager.load().await.unwrap();

        let hw = RegisteredHardware {
            registration_id: "reg-001".to_string(),
            device_id: "dev-001".to_string(),
            hardware_name: "Test Mic".to_string(),
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
            notes: "".to_string(),
        };

        manager.register_device(hw.clone()).await.unwrap();

        let devices = manager.get_registered_devices().await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].user_name, "Main Mic");

        // Verify persistence
        let content = fs::read_to_string(&config_path).await.unwrap();
        assert!(content.contains("Main Mic"));
    }

    #[tokio::test]
    async fn test_register_device_duplicate_user_name_fails() {
        use audiotab::hal::{HardwareType, Direction, AudioProtocol, ChannelMapping, Calibration, ChannelRoute};

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("hardware_config.json");

        let manager = HardwareConfigManager::new(config_path);
        manager.ensure_config_file().await.unwrap();
        manager.load().await.unwrap();

        let hw1 = RegisteredHardware {
            registration_id: "reg-001".to_string(),
            device_id: "dev-001".to_string(),
            hardware_name: "Test Mic".to_string(),
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
            notes: "".to_string(),
        };

        let mut hw2 = hw1.clone();
        hw2.registration_id = "reg-002".to_string();

        manager.register_device(hw1).await.unwrap();
        let result = manager.register_device(hw2).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_update_device() {
        use audiotab::hal::{HardwareType, Direction, AudioProtocol, ChannelMapping, Calibration, ChannelRoute};

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("hardware_config.json");

        let manager = HardwareConfigManager::new(config_path);
        manager.ensure_config_file().await.unwrap();
        manager.load().await.unwrap();

        let hw = RegisteredHardware {
            registration_id: "reg-001".to_string(),
            device_id: "dev-001".to_string(),
            hardware_name: "Test Mic".to_string(),
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
            notes: "".to_string(),
        };

        manager.register_device(hw).await.unwrap();

        let mut updated = manager.get_registered_devices().await.unwrap()[0].clone();
        updated.user_name = "Updated Mic".to_string();
        updated.sample_rate = 96000;

        manager.update_device("reg-001", updated).await.unwrap();

        let devices = manager.get_registered_devices().await.unwrap();
        assert_eq!(devices[0].user_name, "Updated Mic");
        assert_eq!(devices[0].sample_rate, 96000);
    }

    #[tokio::test]
    async fn test_remove_device() {
        use audiotab::hal::{HardwareType, Direction, AudioProtocol, ChannelMapping, Calibration, ChannelRoute};

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("hardware_config.json");

        let manager = HardwareConfigManager::new(config_path);
        manager.ensure_config_file().await.unwrap();
        manager.load().await.unwrap();

        let hw = RegisteredHardware {
            registration_id: "reg-001".to_string(),
            device_id: "dev-001".to_string(),
            hardware_name: "Test Mic".to_string(),
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
            notes: "".to_string(),
        };

        manager.register_device(hw).await.unwrap();
        assert_eq!(manager.get_registered_devices().await.unwrap().len(), 1);

        manager.remove_device("reg-001").await.unwrap();
        assert_eq!(manager.get_registered_devices().await.unwrap().len(), 0);
    }
}

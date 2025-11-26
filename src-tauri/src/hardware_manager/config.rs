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
}

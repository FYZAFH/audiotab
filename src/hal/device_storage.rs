use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use super::device_profile::DeviceProfile;

/// Manages persistence of device profiles to disk
pub struct DeviceStorage {
    storage_dir: PathBuf,
}

impl DeviceStorage {
    /// Create new storage manager
    ///
    /// Creates the storage directory if it doesn't exist
    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&storage_dir)
            .context("Failed to create device storage directory")?;

        Ok(Self { storage_dir })
    }

    /// Save device profile to disk
    pub fn save(&self, profile: &DeviceProfile) -> Result<()> {
        let path = self.profile_path(&profile.id);
        let json = serde_json::to_string_pretty(profile)
            .context("Failed to serialize device profile")?;

        fs::write(&path, json)
            .context(format!("Failed to write profile to {:?}", path))?;

        Ok(())
    }

    /// Load device profile from disk
    pub fn load(&self, id: &str) -> Result<DeviceProfile> {
        let path = self.profile_path(id);
        let json = fs::read_to_string(&path)
            .context(format!("Failed to read profile from {:?}", path))?;

        let profile: DeviceProfile = serde_json::from_str(&json)
            .context("Failed to deserialize device profile")?;

        Ok(profile)
    }

    /// Delete device profile from disk
    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.profile_path(id);
        if path.exists() {
            fs::remove_file(&path)
                .context(format!("Failed to delete profile at {:?}", path))?;
        }
        Ok(())
    }

    /// List all device profiles
    pub fn list_all(&self) -> Result<Vec<DeviceProfile>> {
        let mut profiles = Vec::new();

        if !self.storage_dir.exists() {
            return Ok(profiles);
        }

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json = fs::read_to_string(&path)?;
                if let Ok(profile) = serde_json::from_str::<DeviceProfile>(&json) {
                    profiles.push(profile);
                }
            }
        }

        Ok(profiles)
    }

    fn profile_path(&self, id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::hal::types::{DeviceConfig, SampleFormat, ChannelMapping, Calibration};
    use crate::hal::device_profile::{DeviceProfile, DeviceMetadata};

    #[test]
    fn test_save_and_load_profile() {
        let dir = tempdir().unwrap();
        let storage = DeviceStorage::new(dir.path().to_path_buf()).unwrap();

        let profile = DeviceProfile {
            id: "test-1".to_string(),
            alias: "Test Device".to_string(),
            driver_id: "test-driver".to_string(),
            device_id: "device-0".to_string(),
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

        storage.save(&profile).unwrap();
        let loaded = storage.load(&profile.id).unwrap();
        assert_eq!(profile.id, loaded.id);
        assert_eq!(profile.alias, loaded.alias);
    }

    #[test]
    fn test_list_all_profiles() {
        let dir = tempdir().unwrap();
        let storage = DeviceStorage::new(dir.path().to_path_buf()).unwrap();

        // Save multiple profiles
        for i in 0..3 {
            let profile = DeviceProfile {
                id: format!("device-{}", i),
                alias: format!("Device {}", i),
                driver_id: "test-driver".to_string(),
                device_id: format!("dev-{}", i),
                config: DeviceConfig {
                    name: format!("Device {}", i),
                    sample_rate: 48000,
                    format: SampleFormat::F32,
                    buffer_size: 1024,
                    channel_mapping: ChannelMapping::default(),
                    calibration: Calibration::default(),
                },
                metadata: DeviceMetadata::default(),
            };
            storage.save(&profile).unwrap();
        }

        let profiles = storage.list_all().unwrap();
        assert_eq!(profiles.len(), 3);
    }
}

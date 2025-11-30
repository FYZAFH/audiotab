use serde::{Deserialize, Serialize};
use super::types::{DeviceConfig, SampleFormat, ChannelMapping, Calibration};

/// Complete device profile with configuration and metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceProfile {
    /// Unique identifier for this profile
    pub id: String,

    /// User-friendly name (displayed in UI dropdowns)
    pub alias: String,

    /// Driver that manages this device
    pub driver_id: String,

    /// Hardware device ID from driver discovery
    pub device_id: String,

    /// Device configuration (sample rate, channels, mapping, calibration)
    pub config: DeviceConfig,

    /// Additional metadata
    pub metadata: DeviceMetadata,
}

/// Device metadata for organization and tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceMetadata {
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: u64,  // Unix timestamp
    pub modified_at: u64, // Unix timestamp
}

impl Default for DeviceMetadata {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            description: None,
            tags: Vec::new(),
            created_at: now,
            modified_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_device_profile_serialization() {
        let profile = DeviceProfile {
            id: "test-mic-1".to_string(),
            alias: "Studio Microphone".to_string(),
            driver_id: "cpal-audio".to_string(),
            device_id: "input-0".to_string(),
            config: DeviceConfig {
                name: "USB Microphone".to_string(),
                sample_rate: 48000,
                format: SampleFormat::F32,
                buffer_size: 1024,
                channel_mapping: ChannelMapping::default(),
                calibration: Calibration::default(),
            },
            metadata: DeviceMetadata {
                description: Some("Main recording mic".to_string()),
                tags: vec!["recording".to_string(), "studio".to_string()],
                created_at: 1701436800,
                modified_at: 1701436800,
            },
        };

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: DeviceProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile.id, deserialized.id);
        assert_eq!(profile.alias, deserialized.alias);
    }
}

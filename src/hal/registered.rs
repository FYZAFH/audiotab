use serde::{Deserialize, Serialize};
use super::{HardwareType, ChannelMapping, Calibration};

/// Device direction (input or output)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Input,
    Output,
}

/// Audio protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioProtocol {
    ASIO,
    CoreAudio,
    ALSA,
    WASAPI,
    Jack,
}

/// Registered hardware device configuration (persistent)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegisteredHardware {
    // Identity
    pub registration_id: String,
    pub device_id: String,
    pub hardware_name: String,
    pub driver_id: String,
    pub hardware_type: HardwareType,
    pub direction: Direction,

    // User Configuration
    pub user_name: String,
    pub enabled: bool,

    // Audio Configuration
    pub protocol: Option<AudioProtocol>,
    pub sample_rate: u64,
    pub channels: usize,
    pub channel_mapping: ChannelMapping,
    pub calibration: Calibration,
    pub max_voltage: f64,
    pub notes: String,
}

/// Hardware configuration file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    pub version: String,
    pub registered_devices: Vec<RegisteredHardware>,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            registered_devices: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::ChannelRoute;

    #[test]
    fn test_registered_hardware_roundtrip() {
        let hw = RegisteredHardware {
            registration_id: "reg-001".to_string(),
            device_id: "dev-001".to_string(),
            hardware_name: "MacBook Pro Microphone".to_string(),
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
                routing: vec![
                    ChannelRoute::Direct(0),
                    ChannelRoute::Direct(1),
                ],
            },
            calibration: Calibration { gain: 1.0, offset: 0.0 },
            max_voltage: 0.0,
            notes: "".to_string(),
        };

        let json = serde_json::to_string(&hw).unwrap();
        let deserialized: RegisteredHardware = serde_json::from_str(&json).unwrap();

        assert_eq!(hw.registration_id, deserialized.registration_id);
        assert_eq!(hw.user_name, deserialized.user_name);
    }

    #[test]
    fn test_hardware_config_json_format() {
        let config = HardwareConfig {
            version: "1.0".to_string(),
            registered_devices: vec![],
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"registered_devices\""));
    }
}

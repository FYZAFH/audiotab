use async_trait::async_trait;
use anyhow::Result;
use cpal::traits::{HostTrait, DeviceTrait};
use crate::hal::traits::HardwareDriver;
use crate::hal::types::*;
use crate::hal::Device;
use super::audio_device::AudioDevice;

pub struct AudioDriver;

impl AudioDriver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HardwareDriver for AudioDriver {
    fn driver_id(&self) -> &str {
        "cpal-audio"
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::Acoustic
    }

    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>> {
        // Run CPAL device enumeration in a blocking task since it may block on macOS
        tokio::task::spawn_blocking(|| {
            let mut devices = Vec::new();
            let host = cpal::default_host();

            // Input devices
            if let Ok(input_devices) = host.input_devices() {
                for (idx, device) in input_devices.enumerate() {
                    if let Ok(name) = device.name() {
                        devices.push(DeviceInfo {
                            id: format!("input-{}", idx),
                            name: format!("{} (Input)", name),
                            hardware_type: HardwareType::Acoustic,
                            driver_id: "cpal-audio".to_string(),
                        });
                    }
                }
            }

            // Output devices
            if let Ok(output_devices) = host.output_devices() {
                for (idx, device) in output_devices.enumerate() {
                    if let Ok(name) = device.name() {
                        devices.push(DeviceInfo {
                            id: format!("output-{}", idx),
                            name: format!("{} (Output)", name),
                            hardware_type: HardwareType::Acoustic,
                            driver_id: "cpal-audio".to_string(),
                        });
                    }
                }
            }

            Ok(devices)
        })
        .await?
    }

    fn create_device(&self, _device_id: &str, config: DeviceConfig) -> Result<Box<dyn Device>> {
        let device = AudioDevice::new(
            config.name,
            config.sample_rate,
            config.format,
            config.buffer_size,
            config.channel_mapping.physical_channels,
        )?;

        Ok(Box::new(device))
    }
}

impl Default for AudioDriver {
    fn default() -> Self {
        Self::new()
    }
}

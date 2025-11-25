use async_trait::async_trait;
use anyhow::Result;
use super::types::{DeviceInfo, DeviceConfig, DeviceCapabilities, DeviceChannels, HardwareType};

/// Trait implemented by hardware drivers for device discovery and creation
#[async_trait]
pub trait HardwareDriver: Send + Sync {
    /// Unique driver identifier (e.g., "cpal-audio", "tcp-stream")
    fn driver_id(&self) -> &str;

    /// Hardware classification for framework support level
    fn hardware_type(&self) -> HardwareType {
        HardwareType::Special
    }

    /// Discover available devices (async for network discovery)
    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Create device instance with configuration
    fn create_device(
        &self,
        device_id: &str,
        config: DeviceConfig,
    ) -> Result<Box<dyn Device>>;
}

/// Trait implemented by device instances for data streaming
#[async_trait]
pub trait Device: Send {
    /// Start streaming data
    async fn start(&mut self) -> Result<()>;

    /// Stop streaming
    async fn stop(&mut self) -> Result<()>;

    /// Get channels for buffer ping-pong
    fn get_channels(&mut self) -> DeviceChannels;

    /// Device capabilities and metadata
    fn capabilities(&self) -> DeviceCapabilities;

    /// Check if device is currently streaming
    fn is_streaming(&self) -> bool;
}

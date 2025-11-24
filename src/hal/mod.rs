use crate::core::DataFrame;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub mod lifecycle;
pub mod registry;
pub mod mock;

/// Device states during lifecycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    Unopened,
    Opened,
    Running,
    Stopped,
    Closed,
    Error(String),
}

/// Trait for devices that produce data (microphones, DAQ inputs, triggers)
#[async_trait]
pub trait DeviceSource: Send + Sync {
    /// Configure device with JSON config
    async fn configure(&mut self, config: Value) -> Result<()>;

    /// Open device (allocate resources, connect to hardware)
    async fn open(&mut self) -> Result<()>;

    /// Start streaming data
    async fn start(&mut self) -> Result<()>;

    /// Get next frame (blocking until available)
    async fn read_frame(&mut self) -> Result<DataFrame>;

    /// Stop streaming
    async fn stop(&mut self) -> Result<()>;

    /// Close device (release resources)
    async fn close(&mut self) -> Result<()>;

    /// Get current device state
    fn state(&self) -> DeviceState;
}

/// Trait for devices that consume data (speakers, DAQ outputs, relays)
#[async_trait]
pub trait DeviceSink: Send + Sync {
    /// Configure device with JSON config
    async fn configure(&mut self, config: Value) -> Result<()>;

    /// Open device
    async fn open(&mut self) -> Result<()>;

    /// Start accepting data
    async fn start(&mut self) -> Result<()>;

    /// Write frame to device
    async fn write_frame(&mut self, frame: DataFrame) -> Result<()>;

    /// Stop accepting data
    async fn stop(&mut self) -> Result<()>;

    /// Close device
    async fn close(&mut self) -> Result<()>;

    /// Get current device state
    fn state(&self) -> DeviceState;
}

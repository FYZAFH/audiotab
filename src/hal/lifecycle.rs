use super::{DeviceSource, DeviceState};
use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::sync::mpsc;
use crate::core::DataFrame;

/// Manages the lifecycle of a DeviceSource with proper state transitions
pub struct ManagedSource {
    inner: Box<dyn DeviceSource>,
    state: DeviceState,
}

impl ManagedSource {
    pub fn new(source: Box<dyn DeviceSource>) -> Self {
        Self {
            inner: source,
            state: DeviceState::Unopened,
        }
    }

    pub async fn configure(&mut self, config: Value) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot configure device in state {:?}", self.state));
        }
        self.inner.configure(config).await?;
        Ok(())
    }

    pub async fn open(&mut self) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot open device in state {:?}", self.state));
        }
        self.inner.open().await?;
        self.state = DeviceState::Opened;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.state != DeviceState::Opened && self.state != DeviceState::Stopped {
            return Err(anyhow!("Cannot start device in state {:?}", self.state));
        }
        self.inner.start().await?;
        self.state = DeviceState::Running;
        Ok(())
    }

    pub async fn run_streaming(&mut self, tx: mpsc::Sender<DataFrame>) -> Result<()> {
        if self.state != DeviceState::Running {
            return Err(anyhow!("Device not in running state"));
        }

        loop {
            match self.inner.read_frame().await {
                Ok(frame) => {
                    if tx.send(frame).await.is_err() {
                        break; // Channel closed, stop streaming
                    }
                }
                Err(e) => {
                    self.state = DeviceState::Error(e.to_string());
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if self.state != DeviceState::Running {
            return Ok(()); // Already stopped
        }
        self.inner.stop().await?;
        self.state = DeviceState::Stopped;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        if self.state == DeviceState::Running {
            self.stop().await?;
        }
        if self.state != DeviceState::Closed {
            self.inner.close().await?;
            self.state = DeviceState::Closed;
        }
        Ok(())
    }

    pub fn state(&self) -> &DeviceState {
        &self.state
    }
}

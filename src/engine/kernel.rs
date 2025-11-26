use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::hal::{Device, DeviceChannels, HardwareRegistry, DeviceConfig};
use crate::hal::registered::HardwareConfig;
use crate::hal::format_converter;
use crate::engine::AsyncPipeline;

/// Kernel status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelStatus {
    Stopped,
    Initializing,
    Running,
    Error,
}

/// AudioKernelRuntime orchestrates the connection between HAL and Pipeline
pub struct AudioKernelRuntime {
    /// Active device instances
    active_devices: HashMap<String, Box<dyn Device>>,

    /// Device channels for buffer ping-pong
    device_channels: HashMap<String, DeviceChannels>,

    /// Processing pipeline (optional, can run without pipeline)
    pipeline: Option<AsyncPipeline>,

    /// Current kernel status
    status: KernelStatus,

    /// Shutdown signal broadcaster
    shutdown_tx: Option<broadcast::Sender<()>>,

    /// Device reader task handles
    reader_handles: Vec<JoinHandle<Result<()>>>,

    /// Hardware registry for device creation
    registry: HardwareRegistry,

    /// Hardware configuration
    hardware_config: HardwareConfig,
}

impl AudioKernelRuntime {
    /// Create new AudioKernelRuntime
    pub fn new(registry: HardwareRegistry, hardware_config: HardwareConfig) -> Self {
        Self {
            active_devices: HashMap::new(),
            device_channels: HashMap::new(),
            pipeline: None,
            status: KernelStatus::Stopped,
            shutdown_tx: None,
            reader_handles: Vec::new(),
            registry,
            hardware_config,
        }
    }

    /// Get current kernel status
    pub fn status(&self) -> KernelStatus {
        self.status
    }

    /// Get count of active devices
    pub fn active_device_count(&self) -> usize {
        self.active_devices.len()
    }

    /// Set pipeline (optional)
    pub fn set_pipeline(&mut self, pipeline: AsyncPipeline) {
        self.pipeline = Some(pipeline);
    }

    /// Start the kernel - creates and starts all enabled devices
    pub async fn start(&mut self) -> Result<()> {
        if self.status == KernelStatus::Running {
            return Err(anyhow!("Kernel is already running"));
        }

        self.status = KernelStatus::Initializing;

        // Create shutdown channel
        let (shutdown_tx, _) = broadcast::channel(16);
        self.shutdown_tx = Some(shutdown_tx.clone());

        // Create devices from registered hardware
        let registered_devices = self.hardware_config.registered_devices.clone();
        let num_registered = registered_devices.len();

        for registered in registered_devices {
            if !registered.enabled {
                continue;
            }

            // Create device config from registered hardware
            let device_config = DeviceConfig {
                name: registered.user_name.clone(),
                sample_rate: registered.sample_rate,
                format: crate::hal::SampleFormat::F32, // Default to F32
                buffer_size: 1024, // Default buffer size
                channel_mapping: registered.channel_mapping.clone(),
                calibration: registered.calibration,
            };

            // Create device from registry
            match self.registry.create_device(
                &registered.driver_id,
                &registered.device_id,
                device_config,
            ) {
                Ok(mut device) => {
                    // Start the device
                    device.start().await?;

                    // Get device channels
                    let channels = device.get_channels();

                    // Store channels
                    self.device_channels.insert(registered.registration_id.clone(), channels.clone());

                    // Spawn device reader task
                    self.spawn_device_reader_task(
                        registered.registration_id.clone(),
                        channels,
                        shutdown_tx.subscribe(),
                    );

                    // Store device
                    self.active_devices.insert(registered.registration_id.clone(), device);
                }
                Err(e) => {
                    eprintln!(
                        "Failed to create device {}: {}",
                        registered.registration_id, e
                    );
                    // Continue with other devices
                }
            }
        }

        // Check if all devices failed to start
        if self.active_devices.is_empty() && num_registered > 0 {
            self.status = KernelStatus::Error;
            return Err(anyhow!("All devices failed to start"));
        }

        // Start pipeline if available
        if let Some(ref mut pipeline) = self.pipeline {
            pipeline.start().await?;
        }

        self.status = KernelStatus::Running;
        Ok(())
    }

    /// Stop the kernel - stops all devices and pipeline
    pub async fn stop(&mut self) -> Result<()> {
        self.shutdown().await
    }

    /// Gracefully shutdown the kernel
    pub async fn shutdown(&mut self) -> Result<()> {
        if self.status == KernelStatus::Stopped {
            return Ok(());
        }

        // Send shutdown signal to all reader tasks
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }

        // Wait for all reader tasks to complete
        while let Some(handle) = self.reader_handles.pop() {
            let _ = handle.await;
        }

        // Stop all devices
        for (device_id, device) in self.active_devices.iter_mut() {
            if let Err(e) = device.stop().await {
                eprintln!("Failed to stop device {}: {}", device_id, e);
            }
        }

        // Stop pipeline if available
        if let Some(ref mut pipeline) = self.pipeline {
            pipeline.stop().await?;
        }

        // Clear all state
        self.active_devices.clear();
        self.device_channels.clear();
        self.shutdown_tx = None;
        self.status = KernelStatus::Stopped;

        Ok(())
    }

    /// Spawn a task to read from device and convert to DataFrame
    fn spawn_device_reader_task(
        &mut self,
        device_id: String,
        channels: DeviceChannels,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) {
        let handle = tokio::spawn(async move {
            let mut sequence_id = 0u64;

            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                // Try to receive filled buffer from device
                match channels.filled_rx.try_recv() {
                    Ok(packet) => {
                        // Convert PacketBuffer to DataFrame
                        match format_converter::packet_to_frame(&packet, sequence_id) {
                            Ok(_frame) => {
                                // TODO: Send frame to pipeline or RingBufferWriter
                                // This will be implemented in Phase 3 when AudioInputNode is created
                                sequence_id += 1;
                            }
                            Err(e) => {
                                eprintln!("Failed to convert packet to frame: {}", e);
                            }
                        }

                        // Return buffer to device
                        if let Err(e) = channels.empty_tx.try_send(packet) {
                            eprintln!("Failed to return buffer to device: {}", e);
                        }
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        // No data available, yield
                        tokio::task::yield_now().await;
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        eprintln!("Device {} disconnected", device_id);
                        break;
                    }
                }
            }

            Ok(())
        });

        self.reader_handles.push(handle);
    }
}

// Implement Drop to ensure clean shutdown
/// Note: This struct should be properly shut down via `shutdown()` before dropping.
/// The Drop implementation only sends a shutdown signal but cannot await cleanup.
/// Dropping without calling `shutdown()` may leave devices in an inconsistent state.
impl Drop for AudioKernelRuntime {
    fn drop(&mut self) {
        // Note: Can't call async shutdown in Drop, but we can send shutdown signal
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_status_default() {
        let registry = HardwareRegistry::new();
        let config = HardwareConfig::default();
        let kernel = AudioKernelRuntime::new(registry, config);

        assert_eq!(kernel.status(), KernelStatus::Stopped);
    }

    #[test]
    fn test_kernel_active_device_count() {
        let registry = HardwareRegistry::new();
        let config = HardwareConfig::default();
        let kernel = AudioKernelRuntime::new(registry, config);

        assert_eq!(kernel.active_device_count(), 0);
    }
}

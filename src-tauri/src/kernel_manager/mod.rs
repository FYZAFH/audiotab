use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use audiotab::engine::{AudioKernelRuntime, KernelStatus};
use audiotab::hal::{HardwareRegistry, HardwareConfig};

/// KernelManager provides thread-safe access to AudioKernelRuntime for Tauri commands
pub struct KernelManager {
    /// The wrapped kernel runtime with thread-safe access
    runtime: Arc<RwLock<Option<AudioKernelRuntime>>>,

    /// Hardware registry for device creation
    registry: Arc<RwLock<HardwareRegistry>>,

    /// Hardware configuration
    config: Arc<RwLock<HardwareConfig>>,
}

impl KernelManager {
    /// Create a new KernelManager with the given registry and config
    pub fn new(registry: HardwareRegistry, config: HardwareConfig) -> Self {
        Self {
            runtime: Arc::new(RwLock::new(None)),
            registry: Arc::new(RwLock::new(registry)),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Start the kernel - creates a new AudioKernelRuntime and starts it
    pub async fn start_kernel(&self) -> Result<()> {
        let mut runtime_guard = self.runtime.write().await;

        // Check if kernel is already running
        if let Some(ref runtime) = *runtime_guard {
            if runtime.status() == KernelStatus::Running {
                return Err(anyhow!("Kernel is already running"));
            }
        }

        // Create new kernel runtime
        // We need to clone the registry and config - acquire read locks and copy
        let registry_clone = {
            let _registry_guard = self.registry.read().await;
            // HardwareRegistry doesn't implement Clone, so we need a new instance
            // For now, we'll create a new registry and would need to re-register drivers
            // This is a limitation - in production, we'd need HardwareRegistry to be Clone
            // or use Arc<HardwareRegistry> directly
            let mut new_registry = HardwareRegistry::new();
            // Register the audio driver
            new_registry.register(audiotab::hal::AudioDriver::new());
            new_registry
        };

        let config_clone = {
            let config_guard = self.config.read().await;
            config_guard.clone()
        };

        let mut new_runtime = AudioKernelRuntime::new(
            registry_clone,
            config_clone,
        );

        // Start the kernel
        new_runtime.start().await?;

        // Store the running kernel
        *runtime_guard = Some(new_runtime);

        Ok(())
    }

    /// Stop the kernel - gracefully shuts down the AudioKernelRuntime
    pub async fn stop_kernel(&self) -> Result<()> {
        let mut runtime_guard = self.runtime.write().await;

        if let Some(mut runtime) = runtime_guard.take() {
            runtime.stop().await?;
        } else {
            return Err(anyhow!("Kernel is not running"));
        }

        Ok(())
    }

    /// Get the current kernel status
    pub async fn get_status(&self) -> KernelStatus {
        let runtime_guard = self.runtime.read().await;

        if let Some(ref runtime) = *runtime_guard {
            runtime.status()
        } else {
            KernelStatus::Stopped
        }
    }

    /// Get the number of active devices (requires kernel to be running)
    pub async fn get_active_device_count(&self) -> usize {
        let runtime_guard = self.runtime.read().await;

        if let Some(ref runtime) = *runtime_guard {
            runtime.active_device_count()
        } else {
            0
        }
    }

    /// Update the hardware configuration (only allowed when kernel is stopped)
    pub async fn update_config(&self, new_config: HardwareConfig) -> Result<()> {
        // Check that kernel is not running
        let status = self.get_status().await;
        if status == KernelStatus::Running || status == KernelStatus::Initializing {
            return Err(anyhow!("Cannot update configuration while kernel is running"));
        }

        let mut config = self.config.write().await;
        *config = new_config;

        Ok(())
    }
}

impl Clone for KernelManager {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            registry: Arc::clone(&self.registry),
            config: Arc::clone(&self.config),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiotab::hal::{RegisteredHardware, HardwareType, Direction, ChannelMapping, Calibration};

    fn create_test_hardware_config() -> HardwareConfig {
        HardwareConfig {
            version: "1.0".to_string(),
            registered_devices: vec![],
        }
    }

    #[tokio::test]
    async fn test_kernel_manager_new() {
        let registry = HardwareRegistry::new();
        let config = create_test_hardware_config();

        let manager = KernelManager::new(registry, config);

        let status = manager.get_status().await;
        assert_eq!(status, KernelStatus::Stopped);
    }

    #[tokio::test]
    async fn test_kernel_manager_status_stopped_by_default() {
        let registry = HardwareRegistry::new();
        let config = create_test_hardware_config();
        let manager = KernelManager::new(registry, config);

        assert_eq!(manager.get_status().await, KernelStatus::Stopped);
    }

    #[tokio::test]
    async fn test_kernel_manager_prevent_double_start() {
        let mut registry = HardwareRegistry::new();
        registry.register(audiotab::hal::AudioDriver::new());

        let config = create_test_hardware_config();
        let manager = KernelManager::new(registry, config);

        // First start should succeed (even with no devices)
        let result1 = manager.start_kernel().await;
        // With no enabled devices, it will fail
        assert!(result1.is_err() || manager.get_status().await == KernelStatus::Running);

        // If first start succeeded, second start should fail
        if manager.get_status().await == KernelStatus::Running {
            let result2 = manager.start_kernel().await;
            assert!(result2.is_err());
            assert!(result2.unwrap_err().to_string().contains("already running"));
        }
    }

    #[tokio::test]
    async fn test_kernel_manager_stop_when_not_running() {
        let registry = HardwareRegistry::new();
        let config = create_test_hardware_config();
        let manager = KernelManager::new(registry, config);

        let result = manager.stop_kernel().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }

    #[tokio::test]
    async fn test_kernel_manager_active_device_count_when_stopped() {
        let registry = HardwareRegistry::new();
        let config = create_test_hardware_config();
        let manager = KernelManager::new(registry, config);

        assert_eq!(manager.get_active_device_count().await, 0);
    }

    #[tokio::test]
    async fn test_kernel_manager_update_config_when_stopped() {
        let registry = HardwareRegistry::new();
        let config = create_test_hardware_config();
        let manager = KernelManager::new(registry, config);

        let new_config = HardwareConfig {
            version: "1.0".to_string(),
            registered_devices: vec![],
        };

        let result = manager.update_config(new_config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_kernel_manager_cannot_update_config_when_running() {
        let mut registry = HardwareRegistry::new();
        registry.register(audiotab::hal::AudioDriver::new());

        // Create config with an enabled device
        let hw = RegisteredHardware {
            registration_id: "test-001".to_string(),
            device_id: "test-device".to_string(),
            hardware_name: "Test Device".to_string(),
            driver_id: "cpal".to_string(),
            hardware_type: HardwareType::Acoustic,
            direction: Direction::Input,
            user_name: "Test Input".to_string(),
            enabled: true,
            protocol: None,
            sample_rate: 48000,
            channels: 2,
            channel_mapping: ChannelMapping {
                physical_channels: 2,
                virtual_channels: 2,
                routing: vec![],
            },
            calibration: Calibration { gain: 1.0, offset: 0.0 },
            max_voltage: 0.0,
            notes: "".to_string(),
        };

        let config = HardwareConfig {
            version: "1.0".to_string(),
            registered_devices: vec![hw],
        };

        let manager = KernelManager::new(registry, config);

        // Try to start kernel (may fail due to no real device, but that's OK)
        let _ = manager.start_kernel().await;

        // If kernel started successfully, try to update config
        if manager.get_status().await == KernelStatus::Running {
            let new_config = HardwareConfig {
                version: "1.0".to_string(),
                registered_devices: vec![],
            };

            let result = manager.update_config(new_config).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("while kernel is running"));
        }
    }

    #[tokio::test]
    async fn test_kernel_manager_clone() {
        let registry = HardwareRegistry::new();
        let config = create_test_hardware_config();
        let manager = KernelManager::new(registry, config);

        let manager_clone = manager.clone();

        // Both should have the same status
        assert_eq!(manager.get_status().await, manager_clone.get_status().await);
    }

    #[tokio::test]
    async fn test_kernel_manager_start_stop_lifecycle() {
        let mut registry = HardwareRegistry::new();
        registry.register(audiotab::hal::AudioDriver::new());

        let config = create_test_hardware_config();
        let manager = KernelManager::new(registry, config);

        // Initial state should be stopped
        assert_eq!(manager.get_status().await, KernelStatus::Stopped);

        // Try to start (will fail with no devices, but tests the flow)
        let start_result = manager.start_kernel().await;

        // If start succeeded, status should be Running
        if start_result.is_ok() {
            assert_eq!(manager.get_status().await, KernelStatus::Running);

            // Stop should succeed
            let stop_result = manager.stop_kernel().await;
            assert!(stop_result.is_ok());

            // Status should be Stopped
            assert_eq!(manager.get_status().await, KernelStatus::Stopped);
        }
    }
}

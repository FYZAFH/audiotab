use anyhow::Result;
use audiotab::engine::kernel::{AudioKernelRuntime, KernelStatus};
use audiotab::hal::{HardwareRegistry, AudioDriver};
use audiotab::hal::registered::HardwareConfig;

#[tokio::test]
async fn test_kernel_runtime_creation() -> Result<()> {
    // Create HardwareRegistry
    let mut registry = HardwareRegistry::new();
    registry.register(AudioDriver::new());

    // Create empty hardware config
    let config = HardwareConfig::default();

    // Create kernel runtime
    let kernel = AudioKernelRuntime::new(registry, config);

    // Should start in Stopped status
    assert_eq!(kernel.status(), KernelStatus::Stopped);
    assert_eq!(kernel.active_device_count(), 0);

    Ok(())
}

#[tokio::test]
async fn test_kernel_start_stop_lifecycle() -> Result<()> {
    // Create HardwareRegistry with audio driver
    let mut registry = HardwareRegistry::new();
    registry.register(AudioDriver::new());

    // Create empty hardware config
    let config = HardwareConfig::default();

    // Create kernel runtime
    let mut kernel = AudioKernelRuntime::new(registry, config);

    // Start kernel
    kernel.start().await?;
    assert_eq!(kernel.status(), KernelStatus::Running);

    // Stop kernel
    kernel.stop().await?;
    assert_eq!(kernel.status(), KernelStatus::Stopped);

    Ok(())
}

#[tokio::test]
async fn test_kernel_graceful_shutdown() -> Result<()> {
    // Create HardwareRegistry with audio driver
    let mut registry = HardwareRegistry::new();
    registry.register(AudioDriver::new());

    // Create empty hardware config
    let config = HardwareConfig::default();

    // Create kernel runtime
    let mut kernel = AudioKernelRuntime::new(registry, config);

    // Start kernel
    kernel.start().await?;

    // Shutdown should stop all devices and tasks
    kernel.shutdown().await?;
    assert_eq!(kernel.status(), KernelStatus::Stopped);
    assert_eq!(kernel.active_device_count(), 0);

    Ok(())
}

#[tokio::test]
async fn test_kernel_status_transitions() -> Result<()> {
    // Create HardwareRegistry with audio driver
    let mut registry = HardwareRegistry::new();
    registry.register(AudioDriver::new());

    // Create empty hardware config
    let config = HardwareConfig::default();

    // Create kernel runtime
    let mut kernel = AudioKernelRuntime::new(registry, config);

    // Initial status
    assert_eq!(kernel.status(), KernelStatus::Stopped);

    // Start transitions to Initializing then Running
    kernel.start().await?;
    let status = kernel.status();
    assert!(status == KernelStatus::Running || status == KernelStatus::Initializing);

    // Stop transitions back to Stopped
    kernel.stop().await?;
    assert_eq!(kernel.status(), KernelStatus::Stopped);

    Ok(())
}

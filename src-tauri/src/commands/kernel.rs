use crate::kernel_manager::KernelManager;
use audiotab::engine::KernelStatus;
use serde::Serialize;
use tauri::State;

/// Response type for kernel status queries
#[derive(Debug, Serialize, Clone)]
pub struct KernelStatusResponse {
    pub status: KernelStatus,
    pub active_devices: usize,
}

/// Start the kernel with registered devices
///
/// This command initializes and starts the AudioKernelRuntime, connecting
/// all enabled registered devices to the processing pipeline.
///
/// Returns an error if:
/// - The kernel is already running
/// - No enabled devices are registered
/// - Device initialization fails
///
/// # Example
/// ```no_run
/// const response = await invoke('start_kernel');
/// ```
#[tauri::command]
pub fn start_kernel(
    kernel_manager: State<'_, KernelManager>,
) -> Result<KernelStatusResponse, String> {
    // Use the synchronous start_kernel method
    // The kernel startup will happen in a background task managed by KernelManager
    kernel_manager
        .start_kernel_sync()
        .map_err(|e| format!("Failed to start kernel: {}", e))?;

    let status = kernel_manager.get_status_sync();
    let active_devices = kernel_manager.get_active_device_count_sync();

    Ok(KernelStatusResponse {
        status,
        active_devices,
    })
}

/// Stop the kernel and disconnect all devices
///
/// This command gracefully shuts down the AudioKernelRuntime, properly
/// closing all device streams and stopping the processing pipeline.
///
/// Returns an error if:
/// - The kernel is not running
/// - Shutdown fails
///
/// # Example
/// ```no_run
/// const response = await invoke('stop_kernel');
/// ```
#[tauri::command]
pub fn stop_kernel(
    kernel_manager: State<'_, KernelManager>,
) -> Result<KernelStatusResponse, String> {
    // Use the synchronous stop_kernel method
    kernel_manager
        .stop_kernel_sync()
        .map_err(|e| format!("Failed to stop kernel: {}", e))?;

    let status = kernel_manager.get_status_sync();
    let active_devices = kernel_manager.get_active_device_count_sync();

    Ok(KernelStatusResponse {
        status,
        active_devices,
    })
}

/// Get the current kernel status
///
/// This command queries the current state of the AudioKernelRuntime,
/// including whether it's running and how many devices are active.
///
/// # Example
/// ```no_run
/// const response = await invoke('get_kernel_status');
/// console.log(response.status); // e.g., "Running"
/// console.log(response.active_devices); // e.g., 2
/// ```
#[tauri::command]
pub fn get_kernel_status(
    kernel_manager: State<'_, KernelManager>,
) -> Result<KernelStatusResponse, String> {
    let status = kernel_manager.get_status_sync();
    let active_devices = kernel_manager.get_active_device_count_sync();

    Ok(KernelStatusResponse {
        status,
        active_devices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // These tests would require more setup to mock the KernelManager
    // For now, we rely on integration tests in the kernel_manager module
    // and Tauri's command dispatch tests

    #[test]
    fn test_kernel_status_response_serialization() {
        let response = KernelStatusResponse {
            status: KernelStatus::Running,
            active_devices: 2,
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");
        assert!(json.contains("Running"));
        assert!(json.contains("2"));
    }

    #[test]
    fn test_kernel_status_response_stopped() {
        let response = KernelStatusResponse {
            status: KernelStatus::Stopped,
            active_devices: 0,
        };

        assert_eq!(response.active_devices, 0);
    }
}
